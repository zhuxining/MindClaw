//! AgentLoop：业务编排层
//!
//! Channel → MessageBus → AgentLoop → AgentRunner
//!
//! 负责消息消费、会话管理、上下文构建、命令路由、结果持久化
//! 不执行工具，也不维护复杂任务状态，只负责编排
//!
//! 当前文件集中承接：
//! - session 串行化
//! - /new /stop /restart /status 等控制命令
//! - ContextPipeline 调用
//! - RunHooks 创建
//! - AgentRunner 调用与结果落库
//! - 后台派发入口初始化
//!
//! 这里保持单文件是刻意的：
//! - loop 控制面与 turn 编排高度耦合，拆成 commands/control 子目录收益不高
//! - 当前阶段先稳定边界，再决定是否需要进一步细分文件

use crate::agent::agents::{AgentRegistry, ModelRouter, MAIN_AGENT_ID};
use crate::agent::context::{ContextBuildContext, ContextPipeline};
use crate::agent::events::{AgentEvent, UserVisiblePhase};
use crate::agent::hooks::{InteractiveRunHooks, RunHookPublisher};
use crate::agent::observability::{AgentObserver, TracingObserver};
use crate::agent::runner::AgentRunner;
use crate::agent::session::{SessionManager, ToolTrace};
use crate::agent::spawn::AgentSpawnDispatcher;
use crate::agent::spec::AgentRunResult;
use crate::agent::tools::agent_spawn::{DelegateToAgentTool, SpawnBackgroundAgentTool};
use crate::agent::tools::traits::Tool;
use crate::agent::tools::ToolRegistry;
use crate::bus::events::{InboundMessage, OutboundPayload};
use crate::bus::MessageBus;
use crate::error::{AppError, AppResult};
use crate::providers::{ChatMessage, Provider};
use crate::runtime::config::AppConfig;
use dashmap::DashMap;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};
use tokio_util::sync::CancellationToken;

/// Session 级串行化状态
struct SessionSlot {
    queue: VecDeque<InboundMessage>,
    active_run: Option<RunHandle>,
}

impl SessionSlot {
    fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            active_run: None,
        }
    }
}

#[derive(Clone)]
struct RunHandle {
    cancel: CancellationToken,
}

/// Agent 主循环 - 业务编排层
pub struct AgentLoop {
    /// 消息总线
    bus: Arc<MessageBus>,
    /// 会话管理器
    session_mgr: Arc<SessionManager>,
    /// 上下文管线
    context_pipeline: Arc<ContextPipeline>,
    /// AgentRunner（纯执行层）
    runner: Arc<AgentRunner>,
    /// 工具注册表
    tools: Arc<ToolRegistry>,
    /// Agent profile 注册表
    agent_registry: Arc<AgentRegistry>,
    /// 模型路由
    model_router: Arc<ModelRouter>,
    /// Agent 派生执行调度器
    spawn_dispatcher: Arc<AgentSpawnDispatcher>,
    /// Session 级串行化状态表
    sessions: DashMap<String, Mutex<SessionSlot>>,
    /// 观测器
    observer: Arc<dyn AgentObserver>,
    /// 并发闸（限制并行 LLM 请求数）
    concurrency_gate: Arc<tokio::sync::Semaphore>,
}

impl AgentLoop {
    /// 创建新的 AgentLoop
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        bus: Arc<MessageBus>,
        session_mgr: Arc<SessionManager>,
        context_pipeline: Arc<ContextPipeline>,
        provider: Arc<dyn Provider>,
        tools: Arc<ToolRegistry>,
        agent_registry: Arc<AgentRegistry>,
        model_router: Arc<ModelRouter>,
        spawn_dispatcher: Arc<AgentSpawnDispatcher>,
        observer: Arc<dyn AgentObserver>,
        config: Arc<AppConfig>,
    ) -> Self {
        let runner = Arc::new(AgentRunner::new(provider, tools.clone()));
        let llm_concurrency = config.llm_concurrency;

        Self {
            bus,
            session_mgr,
            context_pipeline,
            runner,
            tools,
            agent_registry,
            model_router,
            spawn_dispatcher,
            sessions: DashMap::new(),
            observer,
            concurrency_gate: Arc::new(tokio::sync::Semaphore::new(llm_concurrency)),
        }
    }

    /// 启动 AgentLoop（消费 MessageBus 入站消息）
    pub async fn run(self_arc: Arc<Self>) -> Result<(), AppError> {
        let mut rx = self_arc.bus.take_inbound_rx().await?;

        tracing::info!("agent_loop_started");

        while let Some(message) = rx.recv().await {
            Self::dispatch(&self_arc, message).await;
        }

        tracing::info!("agent_loop_stopped");
        Ok(())
    }

    /// 消息分发
    async fn dispatch(self_arc: &Arc<Self>, message: InboundMessage) {
        let session_key = message
            .session_id
            .clone()
            .unwrap_or_else(|| format!("pending:{}", message.request_id));

        // 检查是否为优先级命令（如 /stop）
        if self_arc.is_priority_command(&message) {
            self_arc
                .handle_priority_command(&session_key, message)
                .await;
            return;
        }

        // 包装为异步任务
        let slot = self_arc
            .sessions
            .entry(session_key.clone())
            .or_insert_with(|| Mutex::new(SessionSlot::new()));

        let mut slot = slot.lock().await;

        if slot.active_run.is_some() {
            slot.queue.push_back(message);
            tracing::debug!(
                session_key = %session_key,
                queue_len = %slot.queue.len(),
                "message_queued"
            );
            return;
        }

        let cancel = CancellationToken::new();
        slot.active_run = Some(RunHandle {
            cancel: cancel.clone(),
        });
        drop(slot);

        let self_clone = Arc::clone(self_arc);
        tokio::spawn(async move {
            self_clone
                .run_session_loop(session_key, message, cancel)
                .await;
        });
    }

    /// Session 串行循环
    async fn run_session_loop(
        &self,
        session_key: String,
        first: InboundMessage,
        cancel: CancellationToken,
    ) {
        if let Err(error) = self.process_message(first, cancel.clone()).await {
            tracing::error!(session_key = %session_key, error = %error, "process_message_failed");
        }

        loop {
            let next = {
                let slot = self.sessions.get(&session_key).unwrap();
                let mut slot = slot.lock().await;

                match slot.queue.pop_front() {
                    Some(msg) => {
                        let cancel = CancellationToken::new();
                        slot.active_run = Some(RunHandle {
                            cancel: cancel.clone(),
                        });
                        Some((msg, cancel))
                    }
                    None => {
                        slot.active_run = None;
                        None
                    }
                }
            };

            match next {
                Some((msg, cancel)) => {
                    if let Err(error) = self.process_message(msg, cancel).await {
                        tracing::error!(session_key = %session_key, error = %error, "process_message_failed");
                    }
                }
                None => break,
            }
        }
    }

    /// 处理单条消息（核心管线）
    async fn process_message(
        &self,
        message: InboundMessage,
        cancel: CancellationToken,
    ) -> Result<(), AppError> {
        let request_id = message.request_id.clone();

        // 1. 双层并发控制
        let _permit = self
            .concurrency_gate
            .acquire()
            .await
            .map_err(|_| AppError::Internal("Concurrency gate closed".to_string()))?;

        // 2. 获取或创建会话
        let session = self
            .session_mgr
            .get_or_create(
                &message.sender,
                &message.mode,
                message.session_id.as_deref(),
            )
            .await?;
        let session_id = session.id.clone();

        self.observer
            .on_event(&AgentEvent::RunStarted {
                session_id: session_id.clone(),
                request_id: request_id.clone(),
            })
            .await;

        if cancel.is_cancelled() {
            self.session_mgr.mark_cancelled(&session_id).await?;
            self.observer.on_event(&AgentEvent::RunCancelled).await;
            return Ok(());
        }

        // 3. 检查斜杠命令
        if self
            .maybe_handle_agent_command(&message, &session_id, cancel.clone())
            .await?
        {
            return Ok(());
        }

        // 4. 构建上下文
        let ctx = ContextBuildContext::new(message.clone(), Arc::new(session.clone()));
        let built_context = self.context_pipeline.build(&ctx).await?;

        self.observer
            .on_event(&AgentEvent::ContextBuilt {
                fragments: built_context.fragments.len(),
            })
            .await;

        // 5. 更新派生执行路由上下文
        self.spawn_dispatcher
            .update_routing_context(crate::agent::spawn::RoutingContext {
                session_key: session_id.clone(),
                channel: message.channel.clone(),
            })
            .await;

        // 6. 构建主 AgentProfile 与本次 run spec
        let profile = self
            .agent_registry
            .get(MAIN_AGENT_ID)
            .ok_or_else(|| AppError::Internal("main agent profile not found".to_string()))?;
        let resolved_model = self.model_router.resolve(profile.as_ref()).to_string();
        let spec = profile
            .build_run_spec(
                built_context.system_prompt.clone(),
                built_context.messages.clone(),
                self.tools.schemas(),
            )
            .with_model(resolved_model);

        // 7. 创建 Hook（桥接两层）
        let publisher = BusPublisher::new(self.bus.clone(), request_id.clone(), session_id.clone());
        let mut hook =
            InteractiveRunHooks::new(Box::new(publisher), session_id.clone(), request_id.clone());

        // 8. 调用 AgentRunner
        let result = self.runner.run(spec, &mut hook, cancel.clone()).await?;

        // 9. 持久化 turn
        self.persist_turn(&session_id, &message.content, &result)
            .await?;

        // 10. 发送完成信号
        self.emit_done(&request_id, &session_id).await?;

        self.observer.on_event(&AgentEvent::RunCompleted).await;

        Ok(())
    }

    /// 检查是否为优先级命令
    fn is_priority_command(&self, message: &InboundMessage) -> bool {
        message.content.starts_with("/stop") || message.content.starts_with("/restart")
    }

    /// 处理优先级命令（同步分发，绕过并发控制）
    async fn handle_priority_command(&self, session_key: &str, message: InboundMessage) {
        match message.content.as_str() {
            "/stop" => {
                // 取消活跃任务
                if let Some(slot) = self.sessions.get(session_key) {
                    let slot = slot.lock().await;
                    if let Some(run) = &slot.active_run {
                        run.cancel.cancel();
                    }
                }
                // 发送停止确认
                let _ = self
                    .emit_text(&message.request_id, session_key, "Stopped.")
                    .await;
                let _ = self.emit_done(&message.request_id, session_key).await;
            }
            "/restart" => {
                // 重启逻辑（配置不变）
                tracing::info!(session_key = %session_key, "restart_requested");
            }
            _ => {}
        }
    }

    /// 处理普通斜杠命令
    async fn maybe_handle_agent_command(
        &self,
        message: &InboundMessage,
        session_id: &str,
        cancel: CancellationToken,
    ) -> Result<bool, AppError> {
        let Some((cmd_name, args)) = parse_agent_command(&message.content) else {
            return Ok(false);
        };

        self.observer
            .on_event(&AgentEvent::CommandIntercepted {
                name: cmd_name.to_string(),
            })
            .await;

        let action = self
            .resolve_agent_action(cmd_name, args, session_id, &message.sender)
            .await?;

        let (response_session_id, response_text) = self
            .apply_agent_action(action, &message.sender, &message.mode, session_id, cancel)
            .await?;

        self.emit_text(&message.request_id, &response_session_id, &response_text)
            .await?;
        self.emit_done(&message.request_id, &response_session_id)
            .await?;
        self.observer.on_event(&AgentEvent::RunCompleted).await;

        Ok(true)
    }

    async fn apply_agent_action(
        &self,
        action: AgentAction,
        sender: &str,
        mode: &crate::models::conversation::ConversationMode,
        session_id: &str,
        cancel: CancellationToken,
    ) -> Result<(String, String), AppError> {
        match action {
            AgentAction::Reply { content } => Ok((session_id.to_string(), content)),
            AgentAction::NewSession => {
                let session = self.session_mgr.create_new(sender, mode).await;
                Ok((
                    session.id,
                    "Started a new conversation session.".to_string(),
                ))
            }
            AgentAction::StopAll => {
                cancel.cancel();
                Ok((
                    session_id.to_string(),
                    "Stopped the active run.".to_string(),
                ))
            }
            AgentAction::RestartAgent => Ok((
                session_id.to_string(),
                "Agent restart is not wired yet; configuration remains unchanged.".to_string(),
            )),
            AgentAction::Status { info } => Ok((session_id.to_string(), info)),
        }
    }

    async fn resolve_agent_action(
        &self,
        cmd_name: &str,
        _args: &str,
        _session_id: &str,
        _sender: &str,
    ) -> Result<AgentAction, AppError> {
        match cmd_name {
            "new" => Ok(AgentAction::NewSession),
            "stop" => Ok(AgentAction::StopAll),
            "restart" => Ok(AgentAction::RestartAgent),
            "status" => {
                let queued_sessions = self
                    .sessions
                    .iter()
                    .filter(|entry| {
                        entry
                            .try_lock()
                            .map(|slot| !slot.queue.is_empty())
                            .unwrap_or(false)
                    })
                    .count();
                let info = format!(
                    "Agent: running | Channels: desktop | Active sessions: {} | Queued sessions: {}",
                    self.sessions.len(),
                    queued_sessions
                );
                Ok(AgentAction::Status { info })
            }
            _ => Ok(AgentAction::Reply {
                content: format!("Unknown command: /{cmd_name}"),
            }),
        }
    }

    /// 持久化 turn 到会话
    async fn persist_turn(
        &self,
        session_id: &str,
        user_content: &str,
        result: &AgentRunResult,
    ) -> Result<(), AppError> {
        // 转换 tool_events 为 tool_traces
        let tool_traces: Vec<ToolTrace> = result
            .tool_events
            .iter()
            .map(|event| {
                ToolTrace {
                    tool_name: event.name.clone(),
                    input_summary: event.input_summary.clone(),
                    output_summary: event.output_summary.clone(),
                    duration_ms: event.duration_ms,
                    success: event.status == crate::agent::spec::ToolStatus::Succeeded,
                    round: 0, // TODO: 从 context 获取
                }
            })
            .collect();

        // 追加 turn - 使用实际的用户消息内容
        self.session_mgr
            .append_turn(
                session_id,
                ChatMessage::user(user_content),
                Some(ChatMessage::assistant_text(&result.content)),
                tool_traces,
            )
            .await?;

        Ok(())
    }

    // ── 发送方法 ─────────────────────────────────────────────────

    #[allow(dead_code)]
    async fn emit_status(
        &self,
        request_id: &str,
        session_id: &str,
        phase: UserVisiblePhase,
    ) -> Result<(), AppError> {
        self.bus
            .publish_outbound(crate::bus::events::OutboundMessage {
                id: uuid::Uuid::new_v4().to_string(),
                request_id: request_id.to_string(),
                session_id: session_id.to_string(),
                target_sender: "local_user".to_string(),
                target_channel: "desktop".to_string(),
                payload: OutboundPayload::Status { phase },
            })
            .await
    }

    async fn emit_text(
        &self,
        request_id: &str,
        session_id: &str,
        content: &str,
    ) -> Result<(), AppError> {
        self.bus
            .publish_outbound(crate::bus::events::OutboundMessage {
                id: uuid::Uuid::new_v4().to_string(),
                request_id: request_id.to_string(),
                session_id: session_id.to_string(),
                target_sender: "local_user".to_string(),
                target_channel: "desktop".to_string(),
                payload: OutboundPayload::Chunk {
                    content: content.to_string(),
                },
            })
            .await
    }

    async fn emit_done(&self, request_id: &str, session_id: &str) -> Result<(), AppError> {
        self.bus
            .publish_outbound(crate::bus::events::OutboundMessage {
                id: uuid::Uuid::new_v4().to_string(),
                request_id: request_id.to_string(),
                session_id: session_id.to_string(),
                target_sender: "local_user".to_string(),
                target_channel: "desktop".to_string(),
                payload: OutboundPayload::Done,
            })
            .await
    }

    /// 取消指定 session 的活跃 run
    pub async fn cancel_session(&self, session_id: &str) -> Result<(), AppError> {
        if let Some(slot) = self.sessions.get(session_id) {
            let slot = slot.lock().await;
            if let Some(run) = &slot.active_run {
                run.cancel.cancel();
            }
        }
        Ok(())
    }

    /// 从配置初始化完整的 AgentLoop（含 AgentSpawnDispatcher 和 SpawnBackgroundAgentTool）
    pub async fn init(
        config: Arc<AppConfig>,
        bus: Arc<MessageBus>,
        session_mgr: Arc<SessionManager>,
        provider: Arc<dyn Provider>,
        agent_registry: Arc<AgentRegistry>,
        model_router: Arc<ModelRouter>,
    ) -> AppResult<Self> {
        let concurrency_gate = Arc::new(Semaphore::new(config.llm_concurrency));

        // 1. base tools（不含 spawn_background_agent）—— 供派生执行使用
        let base_tools = init_tools(&config, vec![]).await?;

        // 2. AgentSpawnDispatcher（共享 provider + base_tools + 并发闸）
        let spawn_dispatcher = Arc::new(AgentSpawnDispatcher::new(
            Arc::clone(&provider),
            Arc::clone(&base_tools),
            Arc::clone(&bus),
            config.data_dir.clone(),
            Arc::clone(&concurrency_gate),
            Arc::clone(&agent_registry),
            Arc::clone(&model_router),
        ));

        // 3. 主工具集 = base_tools + spawn_background_agent
        let spawn_tool = Arc::new(SpawnBackgroundAgentTool::new(Arc::clone(&spawn_dispatcher)));
        let delegate_tool = Arc::new(DelegateToAgentTool::new(Arc::clone(&spawn_dispatcher)));
        let main_tools = init_tools(&config, vec![spawn_tool, delegate_tool]).await?;

        // 4. 其他组件
        let runner = Arc::new(AgentRunner::new(
            Arc::clone(&provider),
            Arc::clone(&main_tools),
        ));
        let context_pipeline = ContextPipeline::build_default(&config);
        let observer: Arc<dyn AgentObserver> = Arc::new(TracingObserver::new());

        Ok(Self {
            bus,
            session_mgr,
            context_pipeline,
            runner,
            tools: main_tools,
            agent_registry,
            model_router,
            spawn_dispatcher,
            sessions: DashMap::new(),
            observer,
            concurrency_gate,
        })
    }
}

#[derive(Debug, Clone)]
enum AgentAction {
    Reply { content: String },
    NewSession,
    StopAll,
    RestartAgent,
    Status { info: String },
}

fn parse_agent_command(input: &str) -> Option<(&str, &str)> {
    let trimmed = input.trim();
    let rest = trimmed.strip_prefix('/')?;
    Some(rest.split_once(' ').unwrap_or((rest, "")))
}

// ============================================================================
// BusPublisher - RunHook 发布器实现
// ============================================================================

/// BusPublisher - 将 RunHooks 事件桥接到 MessageBus
struct BusPublisher {
    bus: Arc<MessageBus>,
    request_id: String,
    session_id: String,
}

impl BusPublisher {
    fn new(bus: Arc<MessageBus>, request_id: String, session_id: String) -> Self {
        Self {
            bus,
            request_id,
            session_id,
        }
    }
}

impl RunHookPublisher for BusPublisher {
    fn emit_status(&self, _request_id: &str, _session_id: &str, phase: UserVisiblePhase) {
        let msg = crate::bus::events::OutboundMessage {
            id: uuid::Uuid::new_v4().to_string(),
            request_id: self.request_id.clone(),
            session_id: self.session_id.clone(),
            target_sender: "local_user".to_string(),
            target_channel: "desktop".to_string(),
            payload: OutboundPayload::Status { phase },
        };
        let bus = self.bus.clone();
        tokio::spawn(async move {
            let _ = bus.publish_outbound(msg).await;
        });
    }

    fn emit_chunk(&self, _request_id: &str, _session_id: &str, _segment_id: u64, content: &str) {
        let msg = crate::bus::events::OutboundMessage {
            id: uuid::Uuid::new_v4().to_string(),
            request_id: self.request_id.clone(),
            session_id: self.session_id.clone(),
            target_sender: "local_user".to_string(),
            target_channel: "desktop".to_string(),
            payload: OutboundPayload::Chunk {
                content: content.to_string(),
            },
        };
        let bus = self.bus.clone();
        tokio::spawn(async move {
            let _ = bus.publish_outbound(msg).await;
        });
    }

    fn emit_segment_end(
        &self,
        _request_id: &str,
        _session_id: &str,
        _segment_id: u64,
        _resuming: bool,
    ) {
        // 段结束信号（当前 UI 不需要，可扩展）
    }
}

/// 截断字符串
#[allow(dead_code)]
fn truncate(input: &str, max_len: usize) -> String {
    if input.chars().count() <= max_len {
        return input.to_string();
    }
    input.chars().take(max_len).collect()
}

/// 初始化工具注册表（基础工具 + 额外工具）
async fn init_tools(
    config: &AppConfig,
    extra: Vec<Arc<dyn Tool + Send + Sync>>,
) -> AppResult<Arc<ToolRegistry>> {
    use crate::agent::tools::file_ops::{FileEditTool, FileReadTool, FileWriteTool};
    use crate::agent::tools::find_files::FindFilesTool;
    use crate::agent::tools::mcp::{register_mcp_tools, MCPManager};
    use crate::agent::tools::path_guard::PathGuard;
    use crate::agent::tools::search_content::SearchContentTool;
    use crate::agent::tools::shell::ShellTool;

    let mut tools = ToolRegistry::new().with_concurrency_limit(config.tool_concurrency);

    let guard = Arc::new(PathGuard::vault_only(config.vault_path.clone()).with_denied("private"));
    tools.register(Arc::new(ShellTool::new(config.data_dir.clone())));
    tools.register(Arc::new(FileReadTool::with_guard(Arc::clone(&guard))));
    tools.register(Arc::new(FileWriteTool::with_guard(Arc::clone(&guard))));
    tools.register(Arc::new(FileEditTool::with_guard(Arc::clone(&guard))));
    tools.register(Arc::new(FindFilesTool::new(Arc::clone(&guard))));
    tools.register(Arc::new(SearchContentTool::new(Arc::clone(&guard))));

    let mcp_manager = MCPManager::from_file(&config.data_dir);
    if mcp_manager.server_count() > 0 {
        if let Err(e) = register_mcp_tools(&mut tools, &mcp_manager).await {
            tracing::warn!(error = %e, "MCP registration failed, continuing without MCP tools");
        } else {
            tracing::info!(servers = %mcp_manager.server_count(), "MCP bridge initialized");
        }
    }

    for tool in extra {
        tools.register(tool);
    }

    tracing::info!(tool_count = %tools.len(), "tools_initialized");
    Ok(Arc::new(tools))
}
