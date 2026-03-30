//! AgentLoop：消息处理主循环
//!
//! Channel → Hooks → Context → Provider → Tool 循环 → 响应
//!
//! Agent 核心围绕"事件驱动外层 + 单次 run 状态机 + 有限工具循环"构建

use crate::agent::commands::traits::{AgentAction, AgentCommandContext};
use crate::agent::commands::AgentCommandRegistry;
use crate::agent::context::{ContextBuildContext, ContextPipeline};
use crate::agent::events::{AgentEvent, ProviderEvent, UsageStats, UserVisiblePhase};
use crate::agent::observer::AgentObserver;
use crate::agent::session::{SessionManager, ToolTrace};
use crate::agent::tools::{ToolCall, ToolRegistry};
use crate::bus::events::{InboundMessage, OutboundPayload};
use crate::bus::MessageBus;
use crate::error::AppError;
use crate::providers::{ChatMessage, ChatRequest, MessageContent, Provider, ToolChoice};
use dashmap::DashMap;
use futures_util::StreamExt;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

/// 最大工具回合数
const MAX_TOOL_ROUNDS: usize = 8;

/// Session 级串行化状态
///
/// 队列 + 活跃 run 合并在同一 Mutex 内，避免 check-then-act 竞态
struct SessionSlot {
    /// 等待处理的消息队列
    queue: VecDeque<InboundMessage>,
    /// 当前活跃 run 的取消句柄
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

/// 活跃 run 句柄
#[derive(Clone)]
struct RunHandle {
    /// 取消令牌
    cancel: CancellationToken,
}

/// Agent 主循环
pub struct AgentLoop {
    /// 消息总线
    bus: Arc<MessageBus>,
    /// 会话管理器
    session_mgr: Arc<SessionManager>,
    /// 上下文管线
    context_pipeline: Arc<ContextPipeline>,
    /// LLM Provider
    provider: Arc<dyn Provider>,
    /// 工具注册表
    tools: Arc<ToolRegistry>,
    /// Agent 命令注册表
    agent_commands: Arc<AgentCommandRegistry>,
    /// Session 级串行化状态表
    sessions: DashMap<String, Mutex<SessionSlot>>,
    /// 观测器
    observer: Arc<dyn AgentObserver>,
}

impl AgentLoop {
    /// 创建新的 AgentLoop
    pub fn new(
        bus: Arc<MessageBus>,
        session_mgr: Arc<SessionManager>,
        context_pipeline: Arc<ContextPipeline>,
        provider: Arc<dyn Provider>,
        tools: Arc<ToolRegistry>,
        agent_commands: Arc<AgentCommandRegistry>,
        observer: Arc<dyn AgentObserver>,
    ) -> Self {
        Self {
            bus,
            session_mgr,
            context_pipeline,
            provider,
            tools,
            agent_commands,
            sessions: DashMap::new(),
            observer,
        }
    }

    /// 启动 AgentLoop（消费 MessageBus 入站消息）
    pub async fn run(self_arc: Arc<Self>) -> Result<(), AppError> {
        let mut rx = self_arc.bus.take_inbound_rx().await?;

        tracing::info!("agent_loop_started");

        while let Some(message) = rx.recv().await {
            Self::on_inbound(&self_arc, message).await;
        }

        tracing::info!("agent_loop_stopped");
        Ok(())
    }

    /// 处理入站消息（按 session 串行化）
    async fn on_inbound(self_arc: &Arc<Self>, message: InboundMessage) {
        let session_key = message
            .session_id
            .clone()
            .unwrap_or_else(|| format!("pending:{}", message.request_id));

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
        if let Err(error) = self.run_once(first, cancel.clone()).await {
            tracing::error!(session_key = %session_key, error = %error, "run_once_failed");
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
                    if let Err(error) = self.run_once(msg, cancel).await {
                        tracing::error!(session_key = %session_key, error = %error, "run_once_failed");
                    }
                }
                None => break,
            }
        }
    }

    /// 单次 run 执行（核心状态机）
    async fn run_once(
        &self,
        message: InboundMessage,
        cancel: CancellationToken,
    ) -> Result<(), AppError> {
        let request_id = message.request_id.clone();

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
        self.observer
            .on_event(&AgentEvent::SessionResolved {
                session_id: session_id.clone(),
            })
            .await;

        if cancel.is_cancelled() {
            self.session_mgr.mark_cancelled(&session_id).await?;
            self.observer.on_event(&AgentEvent::RunCancelled).await;
            return Ok(());
        }

        if self
            .maybe_handle_agent_command(&message, &session_id, cancel.clone())
            .await?
        {
            return Ok(());
        }

        self.emit_status(&request_id, &session_id, UserVisiblePhase::Thinking)
            .await?;

        let ctx = ContextBuildContext::new(message.clone(), Arc::new(session.clone()));
        let built_context = match self.context_pipeline.build(&ctx).await {
            Ok(ctx) => {
                self.observer
                    .on_event(&AgentEvent::ContextBuilt {
                        fragments: ctx.fragments.len(),
                    })
                    .await;
                ctx
            }
            Err(error) => {
                self.session_mgr
                    .mark_failed(&session_id, &request_id, &error.to_string())
                    .await?;
                self.observer
                    .on_event(&AgentEvent::RunFailed {
                        message: error.to_string(),
                    })
                    .await;
                self.emit_error(&request_id, &session_id, &error.to_string(), false)
                    .await?;
                return Ok(());
            }
        };

        let mut messages = built_context.messages;
        let mut final_text = String::new();
        let mut all_tool_traces = Vec::new();
        let tool_schemas = self.tools.schemas();

        for round in 0..MAX_TOOL_ROUNDS {
            if cancel.is_cancelled() {
                self.session_mgr.mark_cancelled(&session_id).await?;
                self.observer.on_event(&AgentEvent::RunCancelled).await;
                return Ok(());
            }

            let (text_buffer, tool_calls, usage) = match self
                .stream_provider_round(
                    &request_id,
                    &session_id,
                    &built_context.system_prompt,
                    &messages,
                    &tool_schemas,
                    cancel.clone(),
                )
                .await
            {
                Ok(result) => result,
                Err(error) => {
                    self.session_mgr
                        .mark_failed(&session_id, &request_id, &error.to_string())
                        .await?;
                    self.observer
                        .on_event(&AgentEvent::RunFailed {
                            message: error.to_string(),
                        })
                        .await;
                    self.emit_error(&request_id, &session_id, &error.to_string(), true)
                        .await?;
                    return Ok(());
                }
            };

            final_text = text_buffer.clone();
            let _ = usage;

            if tool_calls.is_empty() {
                break;
            }

            self.emit_status(&request_id, &session_id, UserVisiblePhase::UsingTools)
                .await?;

            for call in &tool_calls {
                self.observer
                    .on_event(&AgentEvent::ToolStarted {
                        name: call.name.clone(),
                    })
                    .await;
            }

            let tool_start = std::time::Instant::now();
            let results = self
                .tools
                .execute_calls(tool_calls.clone(), cancel.clone())
                .await?;
            let tool_duration_ms = tool_start.elapsed().as_millis() as u64;

            let assistant_parts = {
                let mut parts = Vec::new();
                if !text_buffer.is_empty() {
                    parts.push(MessageContent::Text {
                        text: text_buffer.clone(),
                    });
                }
                parts.extend(tool_calls.iter().map(|call| MessageContent::ToolUse {
                    id: call.id.clone(),
                    name: call.name.clone(),
                    input: call.arguments.clone(),
                }));
                parts
            };

            messages.push(ChatMessage::assistant_parts(assistant_parts));

            for (call, result) in tool_calls.iter().zip(results.iter()) {
                self.observer
                    .on_event(&AgentEvent::ToolFinished {
                        name: call.name.clone(),
                        success: !result.is_error,
                    })
                    .await;

                all_tool_traces.push(ToolTrace {
                    tool_name: call.name.clone(),
                    input_summary: truncate(&call.arguments.to_string(), 500),
                    output_summary: truncate(&result.content, 1000),
                    duration_ms: tool_duration_ms,
                    success: !result.is_error,
                    round: round as u32,
                });

                messages.push(ChatMessage::tool_result(
                    truncate(&result.content, 4_000),
                    result.tool_call_id.clone(),
                    result.is_error,
                ));
            }

            // 最后一轮仍有工具调用 → 超限报错（检查在工具执行后，确保 8 轮完整执行）
            if round == MAX_TOOL_ROUNDS - 1 {
                self.session_mgr
                    .mark_failed(&session_id, &request_id, "tool loop exceeded max rounds")
                    .await?;
                self.emit_error(&request_id, &session_id, "工具循环超过最大轮数限制", false)
                    .await?;
                return Ok(());
            }

            self.emit_status(&request_id, &session_id, UserVisiblePhase::Thinking)
                .await?;
        }

        self.session_mgr
            .append_turn(
                &session_id,
                ChatMessage::user(&message.content),
                Some(ChatMessage::assistant_text(&final_text)),
                all_tool_traces,
            )
            .await?;

        self.emit_done(&request_id, &session_id).await?;
        self.observer.on_event(&AgentEvent::RunCompleted).await;

        Ok(())
    }

    async fn maybe_handle_agent_command(
        &self,
        message: &InboundMessage,
        session_id: &str,
        cancel: CancellationToken,
    ) -> Result<bool, AppError> {
        let Some((cmd_name, args)) = AgentCommandRegistry::parse(&message.content) else {
            return Ok(false);
        };

        let Some(command) = self.agent_commands.get(cmd_name) else {
            return Ok(false);
        };

        self.observer
            .on_event(&AgentEvent::CommandIntercepted {
                name: cmd_name.to_string(),
            })
            .await;

        let action = command
            .execute(AgentCommandContext {
                session_id: session_id.to_string(),
                sender: message.sender.clone(),
                args: args.to_string(),
            })
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

    async fn stream_provider_round(
        &self,
        request_id: &str,
        session_id: &str,
        system_prompt: &str,
        messages: &[ChatMessage],
        tool_schemas: &[crate::providers::ToolSchema],
        cancel: CancellationToken,
    ) -> Result<(String, Vec<ToolCall>, UsageStats), AppError> {
        let mut stream = self
            .provider
            .chat_stream(ChatRequest {
                model: self.provider.model_id(),
                messages,
                system: Some(system_prompt),
                tools: tool_schemas,
                tool_choice: ToolChoice::Auto,
                max_tokens: None,
                cancel,
            })
            .await?;

        let mut text_buffer = String::new();
        let mut tool_calls = Vec::new();
        let mut usage = UsageStats {
            input_tokens: 0,
            output_tokens: 0,
        };
        let mut streamed = false;

        while let Some(event) = stream.next().await {
            match event? {
                ProviderEvent::TextDelta { text } => {
                    if !streamed {
                        self.emit_status(request_id, session_id, UserVisiblePhase::Streaming)
                            .await?;
                        streamed = true;
                    }
                    text_buffer.push_str(&text);
                    self.emit_text(request_id, session_id, &text).await?;
                    self.observer
                        .on_event(&AgentEvent::ProviderTextDelta { len: text.len() })
                        .await;
                }
                ProviderEvent::ToolCall {
                    id,
                    name,
                    arguments_json,
                } => {
                    self.observer
                        .on_event(&AgentEvent::ProviderToolCall { name: name.clone() })
                        .await;
                    tool_calls.push(ToolCall {
                        id,
                        name,
                        arguments: arguments_json,
                    });
                }
                ProviderEvent::Finished {
                    usage: round_usage, ..
                } => {
                    usage = round_usage;
                    break;
                }
            }
        }

        Ok((text_buffer, tool_calls, usage))
    }

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

    async fn emit_error(
        &self,
        request_id: &str,
        session_id: &str,
        message: &str,
        retryable: bool,
    ) -> Result<(), AppError> {
        self.bus
            .publish_outbound(crate::bus::events::OutboundMessage {
                id: uuid::Uuid::new_v4().to_string(),
                request_id: request_id.to_string(),
                session_id: session_id.to_string(),
                target_sender: "local_user".to_string(),
                target_channel: "desktop".to_string(),
                payload: OutboundPayload::Error {
                    message: message.to_string(),
                    retryable,
                },
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
}

fn truncate(input: &str, max_len: usize) -> String {
    if input.chars().count() <= max_len {
        return input.to_string();
    }

    input.chars().take(max_len).collect()
}
