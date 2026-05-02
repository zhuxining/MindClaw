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

use crate::agent::agent::AgentProfile;
use crate::agent::context::{ContextBuildState, ContextPipeline, SystemPromptBuilder};
use crate::agent::events::UserVisiblePhase;
use crate::agent::hooks::{InteractiveRunHooks, RunHookPublisher};
use crate::agent::messages::ChatMessage;
use crate::agent::runner::AgentRunner;
use crate::agent::session::{SessionManager, ToolTrace};
use crate::agent::spec::{AgentRunResult, InvocationMode, StopReason, ToolStatus};
use crate::agent::subagent::AgentSpawnDispatcher;
use crate::agent::tools::spawn::DelegateToAgentTool;
use crate::agent::tools::{build_tools, ToolScope};
use crate::bus::events::{InboundMessage, OutboundPayload};
use crate::bus::MessageBus;
use crate::error::AppError;
use crate::runtime::config::AppConfig;
use dashmap::DashMap;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};
use tokio_util::sync::CancellationToken;

// ============================================================================
// 常量定义
// ============================================================================

/// 优先级命令前缀（绕过队列直接处理）
const CMD_STOP: &str = "/stop";
const CMD_RESTART: &str = "/restart";

/// 普通斜杠命令
const CMD_NEW: &str = "/new";
const CMD_STATUS: &str = "/status";

/// 消息目标
const TARGET_SENDER: &str = "local_user";
const TARGET_CHANNEL: &str = "desktop";

/// Session 级串行化状态
struct SessionSlot {
    /// 等待处理的消息队列
    queue: VecDeque<InboundMessage>,
    /// 当前活跃的 run
    active_run: Option<RunHandle>,
    /// Mid-turn 注入队列（subagent 结果等在 run 期间到达的消息）
    injection_queue: VecDeque<InboundMessage>,
}

impl SessionSlot {
    fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            active_run: None,
            injection_queue: VecDeque::new(),
        }
    }
}

#[derive(Clone)]
struct RunHandle {
    cancel: CancellationToken,
}

// ============================================================================
// AgentLoop
// ============================================================================

/// Agent 主循环 - 业务编排层
pub struct AgentLoop {
    bus: Arc<MessageBus>,
    session_mgr: Arc<SessionManager>,
    context_pipeline: Arc<ContextPipeline>,
    runner: Arc<AgentRunner>,
    app_config: Arc<AppConfig>,
    main_agent: Arc<AgentProfile>,
    spawn_dispatcher: Arc<AgentSpawnDispatcher>,
    /// Session 级串行化状态表
    sessions: DashMap<String, Mutex<SessionSlot>>,
    /// 全局 LLM 并发闸（与 SpawnDispatcher 共享）
    concurrency_gate: Arc<Semaphore>,
}

impl AgentLoop {
    /// 创建 AgentLoop（同步，不可失败）
    ///
    /// 所有依赖由外部（Agent::init()）构建好后传入。
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        bus: Arc<MessageBus>,
        session_mgr: Arc<SessionManager>,
        context_pipeline: Arc<ContextPipeline>,
        runner: Arc<AgentRunner>,
        app_config: Arc<AppConfig>,
        main_agent: Arc<AgentProfile>,
        spawn_dispatcher: Arc<AgentSpawnDispatcher>,
        concurrency_gate: Arc<Semaphore>,
    ) -> Self {
        Self {
            bus,
            session_mgr,
            context_pipeline,
            runner,
            app_config,
            main_agent,
            spawn_dispatcher,
            sessions: DashMap::new(),
            concurrency_gate,
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

    /// 取消指定 session 的活跃 run
    pub async fn cancel_session(&self, session_id: &str) -> Result<(), AppError> {
        // 使用 try_lock 避免 await，这样 DashMap entry 不会跨 await 持有
        // 如果获取失败，说明有其他任务正在处理，直接返回
        if let Some(entry) = self.sessions.get(session_id) {
            if let Ok(mut slot) = entry.value().try_lock() {
                if let Some(run) = &slot.active_run {
                    run.cancel.cancel();
                }
                // 清除 active_run，避免取消和清理之间有窗口期
                slot.active_run = None;
            }
        }
        Ok(())
    }

    // ── 消息分发与串行化 ──────────────────────────────────────────

    /// 消息分发
    async fn dispatch(self_arc: &Arc<Self>, message: InboundMessage) {
        let session_key = message
            .session_id
            .clone()
            .unwrap_or_else(|| format!("pending:{}", message.request_id));

        // /stop /restart 优先级命令：绕过队列直接处理
        if self_arc.is_priority_command(&message) {
            self_arc
                .handle_priority_command(&session_key, message)
                .await;
            return;
        }

        let slot = self_arc
            .sessions
            .entry(session_key.clone())
            .or_insert_with(|| Mutex::new(SessionSlot::new()));

        let mut slot = slot.lock().await;

        if slot.active_run.is_some() {
            if message.is_injection {
                slot.injection_queue.push_back(message);
                tracing::debug!(
                    session_key = %session_key,
                    injection_len = %slot.injection_queue.len(),
                    "injection_queued"
                );
            } else {
                slot.queue.push_back(message);
                tracing::debug!(
                    session_key = %session_key,
                    queue_len = %slot.queue.len(),
                    "message_queued"
                );
            }
            return;
        }

        let cancel = CancellationToken::new();
        slot.active_run = Some(RunHandle {
            cancel: cancel.clone(),
        });
        drop(slot);

        let self_clone = Arc::clone(self_arc);
        let session_key_for_task = session_key.clone();
        tokio::spawn(async move {
            if let Err(e) = self_clone
                .run_session_loop(session_key_for_task, message, cancel)
                .await
            {
                tracing::error!(session_key = %session_key, error = %e, "session_loop_failed");
            }
        });
    }

    /// Session 串行循环
    async fn run_session_loop(
        &self,
        session_key: String,
        first: InboundMessage,
        cancel: CancellationToken,
    ) -> Result<(), AppError> {
        if let Err(error) = self.process_message(first, cancel.clone()).await {
            tracing::error!(session_key = %session_key, error = %error, "process_message_failed");
            // 首条消息失败时清除 active_run，避免 session 永久卡死
            if let Some(slot) = self.sessions.get(&session_key) {
                slot.lock().await.active_run = None;
            }
        }

        loop {
            let next = {
                let Some(slot) = self.sessions.get(&session_key) else {
                    break Ok(());
                };
                let mut slot = slot.lock().await;

                // 注入消息优先（subagent 结果等系统消息）
                let msg = slot
                    .injection_queue
                    .pop_front()
                    .or_else(|| slot.queue.pop_front());

                match msg {
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
                None => break Ok(()),
            }
        }
    }

    // ── 核心管线 ─────────────────────────────────────────────────

    /// 处理单条消息（核心管线）
    async fn process_message(
        &self,
        message: InboundMessage,
        cancel: CancellationToken,
    ) -> Result<(), AppError> {
        let request_id = message.request_id.clone();

        // 1. 并发控制
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

        // 设置父 session，供 subagent announce 注入
        self.spawn_dispatcher
            .set_parent_session(session_id.clone())
            .await;

        tracing::info!(
            session_id = %session_id,
            request_id = %request_id,
            "agent_run_started"
        );

        if cancel.is_cancelled() {
            self.session_mgr.mark_cancelled(&session_id).await?;
            tracing::info!("agent_run_cancelled");
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
        let ctx = ContextBuildState::new(message.clone(), Arc::new(session.clone()));
        let built_context = self.context_pipeline.build(&ctx).await?;

        tracing::debug!(fragments = %built_context.fragments.len(), "agent_context_prepared");

        // 5. 构建 run spec
        let system_prompt =
            compose_system_prompt(self.app_config.data_dir(), &built_context.system_prompt);

        let mut spec = self.main_agent.build_run_spec(
            uuid::Uuid::new_v4().to_string(),
            session_id.clone(),
            InvocationMode::Interactive,
            system_prompt,
            built_context.messages,
            vec![],
        );
        spec.context_window_tokens = Some(self.app_config.context_token_limit);

        // 7. 构建本次 run 的工具范围（轻量，按需重建）
        let spawn_tools: Vec<Box<dyn rig::tool::ToolDyn>> = vec![Box::new(
            DelegateToAgentTool::new(Arc::clone(&self.spawn_dispatcher)),
        )];
        let tools = build_tools(ToolScope::Main {
            config: self.app_config.clone(),
            spawn_tools,
            allowed_tools: vec![],
        })
        .await?;

        let publisher = Box::new(BusPublisher::new(
            self.bus.clone(),
            request_id.clone(),
            session_id.clone(),
        ));
        let hook = Box::new(InteractiveRunHooks::new(
            publisher,
            session_id.clone(),
            request_id.clone(),
        ));

        // 8. 执行 run；流式输出和工具状态由 Runner 的 RigPromptHook 桥接
        let result = self
            .runner
            .run_spec(&spec, tools, hook, cancel.clone())
            .await?;

        if cancel.is_cancelled() || result.stop_reason == StopReason::Cancelled {
            self.emit_done(&request_id, &session_id).await?;
            return Ok(());
        }

        // 9. 持久化
        self.persist_turn(&session_id, &message.content, &result)
            .await?;

        // 10. 完成信号
        self.emit_done(&request_id, &session_id).await?;
        tracing::info!("agent_run_completed");

        Ok(())
    }

    // ── 命令处理 ─────────────────────────────────────────────────

    /// 检查是否为优先级命令（绕过队列直接执行）
    fn is_priority_command(&self, message: &InboundMessage) -> bool {
        message.content.starts_with(CMD_STOP) || message.content.starts_with(CMD_RESTART)
    }

    /// 处理优先级命令
    async fn handle_priority_command(&self, session_key: &str, message: InboundMessage) {
        match message.content.as_str() {
            cmd if cmd.starts_with(CMD_STOP) => {
                // 先获取 cancel token，然后立即释放 DashMap entry，不持锁跨越 await
                let maybe_cancel: Option<CancellationToken> =
                    if let Some(slot) = self.sessions.get(session_key) {
                        let guard = slot.lock().await;
                        guard.active_run.as_ref().map(|r| r.cancel.clone())
                    } else {
                        None
                    };

                if let Some(cancel) = maybe_cancel {
                    cancel.cancel();
                }

                if let Err(e) = self
                    .emit_text(&message.request_id, session_key, "Stopped.")
                    .await
                {
                    tracing::warn!(error = %e, "failed_to_emit_stop_confirmation");
                }
                if let Err(e) = self.emit_done(&message.request_id, session_key).await {
                    tracing::warn!(error = %e, "failed_to_emit_stop_done");
                }
            }
            cmd if cmd.starts_with(CMD_RESTART) => {
                tracing::info!(session_key = %session_key, "restart_requested");
            }
            _ => unreachable!("priority command filter should prevent unknown commands"),
        }
    }

    /// 处理普通斜杠命令（/new /status）
    ///
    /// /stop /restart 已在优先级路径处理，不会到达这里。
    async fn maybe_handle_agent_command(
        &self,
        message: &InboundMessage,
        session_id: &str,
        cancel: CancellationToken,
    ) -> Result<bool, AppError> {
        let Some((cmd_name, args)) = parse_agent_command(&message.content) else {
            return Ok(false);
        };

        tracing::info!(command = %cmd_name, "agent_control_command_intercepted");

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
        tracing::info!("agent_command_run_completed");

        Ok(true)
    }

    async fn resolve_agent_action(
        &self,
        cmd_name: &str,
        _args: &str,
        _session_id: &str,
        _sender: &str,
    ) -> Result<AgentAction, AppError> {
        match cmd_name {
            CMD_NEW => Ok(AgentAction::NewSession),
            CMD_STATUS => {
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
                Ok(AgentAction::Status {
                    info: format!(
                        "Agent: running | Active sessions: {} | Queued sessions: {}",
                        self.sessions.len(),
                        queued_sessions
                    ),
                })
            }
            _ => Ok(AgentAction::Reply {
                content: format!("Unknown command: {cmd_name}"),
            }),
        }
    }

    async fn apply_agent_action(
        &self,
        action: AgentAction,
        sender: &str,
        mode: &crate::models::conversation::ConversationMode,
        session_id: &str,
        _cancel: CancellationToken,
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
            AgentAction::Status { info } => Ok((session_id.to_string(), info)),
        }
    }

    // ── 持久化与发送 ──────────────────────────────────────────────

    async fn persist_turn(
        &self,
        session_id: &str,
        user_content: &str,
        result: &AgentRunResult,
    ) -> Result<(), AppError> {
        let tool_trace = result
            .tool_events
            .iter()
            .map(|event| ToolTrace {
                tool_name: event.name.clone(),
                input_summary: event.input_summary.clone(),
                output_summary: event.output_summary.clone(),
                duration_ms: event.duration_ms,
                success: event.status == ToolStatus::Succeeded,
                round: 0,
            })
            .collect();

        self.session_mgr
            .append_turn(
                session_id,
                ChatMessage::user(user_content),
                Some(ChatMessage::assistant_text(&result.final_text)),
                tool_trace,
            )
            .await
    }

    /// 构建 OutboundMessage
    fn build_outbound(
        &self,
        request_id: &str,
        session_id: &str,
        payload: OutboundPayload,
    ) -> crate::bus::events::OutboundMessage {
        crate::bus::events::OutboundMessage {
            id: uuid::Uuid::new_v4().to_string(),
            request_id: request_id.to_string(),
            session_id: session_id.to_string(),
            target_sender: TARGET_SENDER.to_string(),
            target_channel: TARGET_CHANNEL.to_string(),
            payload,
        }
    }

    async fn emit_text(
        &self,
        request_id: &str,
        session_id: &str,
        content: &str,
    ) -> Result<(), AppError> {
        self.bus
            .publish_outbound(self.build_outbound(
                request_id,
                session_id,
                OutboundPayload::Chunk {
                    content: content.to_string(),
                },
            ))
            .await
    }

    async fn emit_done(&self, request_id: &str, session_id: &str) -> Result<(), AppError> {
        self.bus
            .publish_outbound(self.build_outbound(request_id, session_id, OutboundPayload::Done))
            .await
    }
}

// ============================================================================
// 内部辅助类型
// ============================================================================

#[derive(Debug, Clone)]
enum AgentAction {
    Reply { content: String },
    NewSession,
    Status { info: String },
}

fn parse_agent_command(input: &str) -> Option<(&str, &str)> {
    let trimmed = input.trim();
    // 保留斜杠，匹配统一
    if !trimmed.starts_with('/') {
        return None;
    }
    Some(trimmed.split_once(' ').unwrap_or((trimmed, "")))
}

fn compose_system_prompt(workspace: &std::path::Path, context_system_prompt: &str) -> String {
    let builder = SystemPromptBuilder::new(workspace.to_path_buf());
    let mut parts = vec![builder.build_main_prompt(None)];

    let context_prompt = context_system_prompt.trim();
    if !context_prompt.is_empty() {
        parts.push(context_prompt.to_string());
    }

    parts.join("\n\n---\n\n")
}

// ============================================================================
// BusPublisher - RunHook 发布器实现
// ============================================================================

/// BusPublisher - 将 RunHooks 事件桥接到 MessageBus
///
/// 实现 `RunHookPublisher` trait，但使用构造时绑定的 request_id/session_id
/// 而非 trait 方法传入的参数（一个 publisher 实例对应一次 run，不跨 run 复用）。
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

    fn publish(&self, payload: OutboundPayload) {
        let msg = crate::bus::events::OutboundMessage {
            id: uuid::Uuid::new_v4().to_string(),
            request_id: self.request_id.clone(),
            session_id: self.session_id.clone(),
            target_sender: TARGET_SENDER.to_string(),
            target_channel: TARGET_CHANNEL.to_string(),
            payload,
        };
        let bus = self.bus.clone();
        // 注意：tokio::spawn 在极端资源耗尽时可能延迟或失败，但这种情况很少见
        // 如果需要更强的保证，可以考虑使用 bounded channel 进行反压控制
        tokio::spawn(async move {
            if let Err(e) = bus.publish_outbound(msg).await {
                tracing::error!(error = %e, "publish_outbound_failed");
            }
        });
    }
}

impl RunHookPublisher for BusPublisher {
    fn emit_status(&self, _: &str, _: &str, status: UserVisiblePhase) {
        self.publish(OutboundPayload::Status { status });
    }

    fn emit_chunk(&self, _: &str, _: &str, _: u64, content: &str) {
        self.publish(OutboundPayload::Chunk {
            content: content.to_string(),
        });
    }

    fn emit_segment_end(&self, _: &str, _: &str, _: u64, _: bool) {
        // 段结束信号（当前 UI 不需要，可扩展）
    }
}
