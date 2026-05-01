//! AgentRunner — Rig-backed run 执行内核
//!
//! MindClaw 保留 run 契约和业务编排，run 内部的 LLM/tool/streaming
//! 循环交给 rig Agent、StreamingPromptRequest 和 PromptHook。

use crate::agent::hooks::{
    IterationFinishContext, IterationStartContext, ModelRequestContext, ModelResponseContext,
    RunAbortReason, RunHooks, RunStartContext, ToolCallPlaceholder,
};
use crate::agent::messages::{ChatMessage, MessageContent, MessageRole, ToolChoice};
use crate::agent::spec::{
    AgentRunResult, AgentRunSpec, StopReason, TokenUsage, ToolEvent, ToolStatus,
};
use crate::error::AppError;
use crate::match_completion_model;
use crate::providers::AgentModelSet;
use rig::agent::{
    AgentBuilder, HookAction, MultiTurnStreamItem, PromptHook, StreamingError,
    StreamingPromptRequest, ToolCallHookAction,
};
use rig::completion::request::GetTokenUsage;
use rig::completion::{CompletionModel, PromptError};
use rig::message::Message;
use rig::tool::{server::ToolServer, ToolDyn, ToolSet};
use std::collections::HashMap;
use std::time::Instant;
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

pub struct AgentRunner {
    models: AgentModelSet,
}

impl AgentRunner {
    pub fn new(models: AgentModelSet) -> Self {
        Self { models }
    }

    pub async fn run_spec(
        &self,
        spec: &AgentRunSpec,
        tools: Vec<Box<dyn ToolDyn>>,
        hook: Box<dyn RunHooks>,
        cancel: CancellationToken,
    ) -> Result<AgentRunResult, AppError> {
        let observer = Arc::new(Mutex::new(hook));
        {
            let mut observer = observer.lock().await;
            observer.on_run_start(&RunStartContext {
                run_id: spec.run_id.clone(),
                session_id: spec.session_id.clone(),
                agent_id: spec.agent_id.clone(),
                message_count: spec.messages.len(),
            });
        }

        if cancel.is_cancelled() {
            notify_abort(&observer, RunAbortReason::Cancelled).await;
            return Ok(AgentRunResult::cancelled());
        }

        let rig_hook = RigPromptHook::new(spec, Arc::clone(&observer), cancel.clone());
        let model = self.models.model_for(&spec.model);
        let outcome = match_completion_model!(
            model,
            run_with_model,
            spec,
            tools,
            rig_hook.clone(),
            cancel.clone()
        );

        let mut result = match outcome {
            Ok(outcome) => {
                let final_text = {
                    let mut observer = observer.lock().await;
                    observer.finalize_response(&outcome.final_text)
                };

                AgentRunResult {
                    final_text,
                    full_message_chain: outcome.messages,
                    tools_used: Vec::new(),
                    usage: outcome.usage,
                    stop_reason: outcome.stop_reason,
                    error: outcome.error,
                    tool_events: rig_hook.tool_events().await,
                }
            }
            Err(RunFailure::Cancelled) => {
                notify_abort(&observer, RunAbortReason::Cancelled).await;
                AgentRunResult::cancelled()
            }
            Err(RunFailure::MaxTurns { messages }) => {
                AgentRunResult::max_iterations(String::new(), messages, spec.max_iterations)
            }
            Err(RunFailure::ToolError { error, messages }) => {
                notify_abort(
                    &observer,
                    RunAbortReason::Error {
                        message: error.clone(),
                    },
                )
                .await;
                AgentRunResult::tool_error(error, messages)
            }
            Err(RunFailure::Provider(error)) => {
                notify_abort(
                    &observer,
                    RunAbortReason::Error {
                        message: error.clone(),
                    },
                )
                .await;
                return Err(AppError::Provider(error));
            }
        };

        result.tools_used = result
            .tool_events
            .iter()
            .map(|event| event.name.clone())
            .collect();

        if result.stop_reason == StopReason::Completed {
            let mut observer = observer.lock().await;
            observer.on_finish(&result);
        }

        Ok(result)
    }
}

async fn run_with_model<M>(
    model: M,
    spec: &AgentRunSpec,
    tools: Vec<Box<dyn ToolDyn>>,
    hook: RigPromptHook,
    cancel: CancellationToken,
) -> Result<RunOutcome, RunFailure>
where
    M: CompletionModel + Clone + 'static,
    M::StreamingResponse: GetTokenUsage + Clone + Unpin + Send + Sync + 'static,
{
    let rig_messages = to_rig_messages(&spec.messages);
    let Some((prompt, history)) = rig_messages.split_last() else {
        return Ok(RunOutcome::completed(
            String::new(),
            TokenUsage::default(),
            Vec::new(),
        ));
    };

    let tool_server = ToolServer::new().run();
    tool_server
        .append_toolset(ToolSet::from_tools_boxed(tools))
        .await
        .map_err(|error| RunFailure::Provider(error.to_string()))?;

    let mut builder = AgentBuilder::new(model)
        .name(&spec.agent_id)
        .preamble(&spec.system_prompt)
        .hook(hook.clone())
        .tool_server_handle(tool_server)
        .default_max_turns(spec.max_iterations);

    if let Some(temperature) = spec.temperature {
        builder = builder.temperature(temperature as f64);
    }
    if let Some(max_tokens) = spec.max_tokens {
        builder = builder.max_tokens(max_tokens as u64);
    }
    if let Some(tool_choice) = to_rig_tool_choice(&spec.tool_choice) {
        builder = builder.tool_choice(tool_choice);
    }

    let agent = builder.build();
    let request = StreamingPromptRequest::<M, RigPromptHook>::from_agent(&agent, prompt.clone())
        .with_history(history.to_vec())
        .multi_turn(spec.max_iterations)
        .with_hook(hook.clone());
    let mut stream = request.await;

    use futures_util::StreamExt;

    let mut final_text = String::new();
    let mut final_usage = TokenUsage::default();
    let mut final_messages = Vec::new();

    while let Some(item) = stream.next().await {
        if cancel.is_cancelled() {
            return Err(RunFailure::Cancelled);
        }

        match item {
            Ok(MultiTurnStreamItem::FinalResponse(response)) => {
                final_text = response.response().to_string();
                final_usage = TokenUsage::new(
                    response.usage().input_tokens as usize,
                    response.usage().output_tokens as usize,
                );
                final_messages = response
                    .history()
                    .map(to_chat_messages)
                    .unwrap_or_else(Vec::new);
            }
            Ok(_) => {}
            Err(error) => {
                return handle_stream_error(error, &hook).await;
            }
        }
    }

    Ok(RunOutcome::completed(
        final_text,
        final_usage,
        final_messages,
    ))
}

async fn handle_stream_error(
    error: StreamingError,
    hook: &RigPromptHook,
) -> Result<RunOutcome, RunFailure> {
    match error {
        StreamingError::Prompt(prompt_error) => match *prompt_error {
            PromptError::MaxTurnsError { chat_history, .. } => Err(RunFailure::MaxTurns {
                messages: to_chat_messages(&chat_history),
            }),
            PromptError::PromptCancelled {
                chat_history,
                reason,
            } => {
                if reason == "run cancelled" {
                    return Err(RunFailure::Cancelled);
                }
                if let Some(tool_error) = hook.last_tool_error().await {
                    Err(RunFailure::ToolError {
                        error: tool_error,
                        messages: to_chat_messages(&chat_history),
                    })
                } else {
                    Err(RunFailure::Provider(reason))
                }
            }
            other => Err(RunFailure::Provider(other.to_string())),
        },
        other => Err(RunFailure::Provider(other.to_string())),
    }
}

async fn notify_abort(observer: &SharedObserver, reason: RunAbortReason) {
    let mut observer = observer.lock().await;
    observer.on_abort(&reason);
}

fn to_rig_tool_choice(choice: &ToolChoice) -> Option<rig::message::ToolChoice> {
    match choice {
        ToolChoice::Auto => Some(rig::message::ToolChoice::Auto),
        ToolChoice::None => Some(rig::message::ToolChoice::None),
        ToolChoice::Required => Some(rig::message::ToolChoice::Required),
        ToolChoice::Specific(name) => Some(rig::message::ToolChoice::Specific {
            function_names: vec![name.clone()],
        }),
    }
}

fn to_rig_messages(messages: &[ChatMessage]) -> Vec<Message> {
    messages
        .iter()
        .map(|message| match message.role {
            MessageRole::System => Message::system(message.text_content()),
            MessageRole::User => Message::user(message.text_content()),
            MessageRole::Assistant => Message::assistant(message.text_content()),
        })
        .collect()
}

fn to_chat_messages(messages: &[Message]) -> Vec<ChatMessage> {
    messages.iter().map(to_chat_message).collect()
}

fn to_chat_message(message: &Message) -> ChatMessage {
    match message {
        Message::System { content } => ChatMessage::system(content),
        Message::User { content } => {
            let parts = content
                .iter()
                .filter_map(|item| match item {
                    rig::message::UserContent::Text(text) => Some(MessageContent::Text {
                        text: text.text.clone(),
                    }),
                    rig::message::UserContent::ToolResult(result) => {
                        Some(MessageContent::ToolResult {
                            tool_use_id: result.id.clone(),
                            content: result
                                .content
                                .iter()
                                .filter_map(|content| match content {
                                    rig::message::ToolResultContent::Text(text) => {
                                        Some(text.text.as_str())
                                    }
                                    rig::message::ToolResultContent::Image(_) => None,
                                })
                                .collect::<Vec<_>>()
                                .join("\n"),
                            is_error: false,
                        })
                    }
                    _ => None,
                })
                .collect();
            ChatMessage {
                role: MessageRole::User,
                content: parts,
            }
        }
        Message::Assistant { content, .. } => {
            let parts = content
                .iter()
                .filter_map(|item| match item {
                    rig::message::AssistantContent::Text(text) => Some(MessageContent::Text {
                        text: text.text.clone(),
                    }),
                    rig::message::AssistantContent::ToolCall(call) => {
                        Some(MessageContent::ToolUse {
                            id: call.id.clone(),
                            name: call.function.name.clone(),
                            input: call.function.arguments.clone(),
                        })
                    }
                    _ => None,
                })
                .collect();
            ChatMessage {
                role: MessageRole::Assistant,
                content: parts,
            }
        }
    }
}

type SharedObserver = Arc<Mutex<Box<dyn RunHooks>>>;

#[derive(Clone)]
struct RigPromptHook {
    run_id: String,
    session_id: String,
    agent_id: String,
    model: String,
    fail_on_tool_error: bool,
    observer: SharedObserver,
    state: Arc<Mutex<RigHookState>>,
    cancel: CancellationToken,
}

impl RigPromptHook {
    fn new(spec: &AgentRunSpec, observer: SharedObserver, cancel: CancellationToken) -> Self {
        Self {
            run_id: spec.run_id.clone(),
            session_id: spec.session_id.clone(),
            agent_id: spec.agent_id.clone(),
            model: spec.model.clone(),
            fail_on_tool_error: spec.fail_on_tool_error,
            observer,
            state: Arc::new(Mutex::new(RigHookState::default())),
            cancel,
        }
    }

    async fn tool_events(&self) -> Vec<ToolEvent> {
        self.state.lock().await.tool_events.clone()
    }

    async fn last_tool_error(&self) -> Option<String> {
        self.state.lock().await.last_tool_error.clone()
    }
}

impl<M> PromptHook<M> for RigPromptHook
where
    M: CompletionModel,
    M::StreamingResponse: GetTokenUsage + Send + Sync + 'static,
{
    async fn on_completion_call(&self, _prompt: &Message, history: &[Message]) -> HookAction {
        if self.cancel.is_cancelled() {
            return HookAction::terminate("run cancelled");
        }

        let iteration = {
            let mut state = self.state.lock().await;
            let iteration = state.current_iteration;
            state.current_iteration += 1;
            state.current_tool_call_count = 0;
            state.model_response_reported = false;
            iteration
        };

        let mut observer = self.observer.lock().await;
        observer.on_iteration_start(&IterationStartContext {
            run_id: self.run_id.clone(),
            session_id: self.session_id.clone(),
            agent_id: self.agent_id.clone(),
            iteration,
            message_count: history.len() + 1,
        });
        observer.on_model_request_start(&ModelRequestContext {
            run_id: self.run_id.clone(),
            session_id: self.session_id.clone(),
            agent_id: self.agent_id.clone(),
            iteration,
            model: self.model.clone(),
            is_streaming: true,
        });

        HookAction::cont()
    }

    async fn on_tool_call(
        &self,
        tool_name: &str,
        tool_call_id: Option<String>,
        internal_call_id: &str,
        args: &str,
    ) -> ToolCallHookAction {
        if self.cancel.is_cancelled() {
            return ToolCallHookAction::terminate("run cancelled");
        }

        let call_id = tool_call_id.unwrap_or_else(|| internal_call_id.to_string());
        let call = ToolCallPlaceholder {
            name: tool_name.to_string(),
            id: call_id.clone(),
        };
        let (iteration, first_tool_call) = {
            let mut state = self.state.lock().await;
            state.current_tool_call_count += 1;
            let first_tool_call = !state.model_response_reported;
            state.model_response_reported = true;
            state.active_tools.insert(
                internal_call_id.to_string(),
                ToolStart {
                    name: tool_name.to_string(),
                    tool_call_id: call_id,
                    args: args.to_string(),
                    started_at: Instant::now(),
                },
            );
            (state.current_iteration.saturating_sub(1), first_tool_call)
        };

        let mut observer = self.observer.lock().await;
        if first_tool_call {
            observer.on_model_response_ready(&ModelResponseContext {
                run_id: self.run_id.clone(),
                session_id: self.session_id.clone(),
                agent_id: self.agent_id.clone(),
                iteration,
                has_tool_calls: true,
                tool_call_count: 1,
                usage: TokenUsage::default(),
            });
        }
        observer.on_tool_batch_start(std::slice::from_ref(&call));
        observer.on_tool_call_start(&call);

        ToolCallHookAction::cont()
    }

    async fn on_tool_result(
        &self,
        tool_name: &str,
        tool_call_id: Option<String>,
        internal_call_id: &str,
        args: &str,
        result: &str,
    ) -> HookAction {
        if self.cancel.is_cancelled() {
            return HookAction::terminate("run cancelled");
        }

        let success = !is_tool_failure(result);
        let call_id = tool_call_id.unwrap_or_else(|| internal_call_id.to_string());
        let call = ToolCallPlaceholder {
            name: tool_name.to_string(),
            id: call_id.clone(),
        };
        let event = {
            let mut state = self.state.lock().await;
            let start = state.active_tools.remove(internal_call_id);
            let duration = start
                .as_ref()
                .map(|start| start.started_at.elapsed())
                .unwrap_or(Duration::ZERO);
            let event = ToolEvent {
                name: start
                    .as_ref()
                    .map(|start| start.name.clone())
                    .unwrap_or_else(|| tool_name.to_string()),
                tool_call_id: start
                    .as_ref()
                    .map(|start| start.tool_call_id.clone())
                    .unwrap_or(call_id),
                status: if success {
                    ToolStatus::Succeeded
                } else {
                    ToolStatus::Failed {
                        error: result.to_string(),
                    }
                },
                duration_ms: duration.as_millis() as u64,
                input_summary: truncate_summary(
                    start
                        .as_ref()
                        .map(|start| start.args.as_str())
                        .unwrap_or(args),
                ),
                output_summary: truncate_summary(result),
            };
            if !success {
                state.last_tool_error = Some(result.to_string());
            }
            state.tool_events.push(event.clone());
            event
        };

        let mut observer = self.observer.lock().await;
        observer.on_tool_call_finish(&call, success, &event.output_summary);

        if self.fail_on_tool_error && !success {
            HookAction::terminate(result.to_string())
        } else {
            HookAction::cont()
        }
    }

    async fn on_text_delta(&self, text_delta: &str, _aggregated_text: &str) -> HookAction {
        if self.cancel.is_cancelled() {
            return HookAction::terminate("run cancelled");
        }

        let mut observer = self.observer.lock().await;
        observer.on_model_text_delta(text_delta);
        HookAction::cont()
    }

    async fn on_stream_completion_response_finish(
        &self,
        _prompt: &Message,
        response: &<M as CompletionModel>::StreamingResponse,
    ) -> HookAction {
        if self.cancel.is_cancelled() {
            return HookAction::terminate("run cancelled");
        }

        let usage = response
            .token_usage()
            .map(|usage| TokenUsage::new(usage.input_tokens as usize, usage.output_tokens as usize))
            .unwrap_or_default();
        let (iteration, tool_call_count, should_report) = {
            let mut state = self.state.lock().await;
            let should_report = !state.model_response_reported;
            state.model_response_reported = true;
            (
                state.current_iteration.saturating_sub(1),
                state.current_tool_call_count,
                should_report,
            )
        };

        let mut observer = self.observer.lock().await;
        if should_report {
            observer.on_model_response_ready(&ModelResponseContext {
                run_id: self.run_id.clone(),
                session_id: self.session_id.clone(),
                agent_id: self.agent_id.clone(),
                iteration,
                has_tool_calls: tool_call_count > 0,
                tool_call_count,
                usage: usage.clone(),
            });
        }
        observer.on_iteration_finish(&IterationFinishContext {
            run_id: self.run_id.clone(),
            session_id: self.session_id.clone(),
            agent_id: self.agent_id.clone(),
            iteration,
            message_count: 0,
            usage,
        });

        HookAction::cont()
    }
}

#[derive(Default)]
struct RigHookState {
    current_iteration: usize,
    current_tool_call_count: usize,
    model_response_reported: bool,
    active_tools: HashMap<String, ToolStart>,
    tool_events: Vec<ToolEvent>,
    last_tool_error: Option<String>,
}

struct ToolStart {
    name: String,
    tool_call_id: String,
    args: String,
    started_at: Instant,
}

#[derive(Debug)]
struct RunOutcome {
    final_text: String,
    usage: TokenUsage,
    messages: Vec<ChatMessage>,
    stop_reason: StopReason,
    error: Option<String>,
}

impl RunOutcome {
    fn completed(final_text: String, usage: TokenUsage, messages: Vec<ChatMessage>) -> Self {
        Self {
            final_text,
            usage,
            messages,
            stop_reason: StopReason::Completed,
            error: None,
        }
    }
}

#[derive(Debug)]
enum RunFailure {
    Cancelled,
    MaxTurns {
        messages: Vec<ChatMessage>,
    },
    ToolError {
        error: String,
        messages: Vec<ChatMessage>,
    },
    Provider(String),
}

fn is_tool_failure(result: &str) -> bool {
    let normalized = result.trim_start().to_ascii_lowercase();
    normalized.starts_with("[error]")
        || normalized.starts_with("toolset error")
        || normalized.starts_with("toolcallerror")
        || normalized.starts_with("toolservererror")
        || normalized.starts_with("jsonerror")
}

fn truncate_summary(value: &str) -> String {
    const MAX_LEN: usize = 500;
    if value.len() <= MAX_LEN {
        return value.to_string();
    }
    let end = value
        .char_indices()
        .map(|(idx, _)| idx)
        .take_while(|idx| *idx <= MAX_LEN)
        .last()
        .unwrap_or(0);
    format!("{}...", &value[..end])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::hooks::NoopRunHooks;
    use rig::completion::{
        CompletionError, CompletionRequest, CompletionResponse, ToolDefinition, Usage,
    };
    use rig::streaming::{RawStreamingChoice, RawStreamingToolCall, StreamingCompletionResponse};
    use serde::{Deserialize, Serialize};
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Clone, Debug, Deserialize, Serialize)]
    struct MockStreamingResponse {
        usage: Usage,
    }

    impl MockStreamingResponse {
        fn new(input_tokens: u64, output_tokens: u64) -> Self {
            let mut usage = Usage::new();
            usage.input_tokens = input_tokens;
            usage.output_tokens = output_tokens;
            usage.total_tokens = input_tokens + output_tokens;
            Self { usage }
        }
    }

    impl GetTokenUsage for MockStreamingResponse {
        fn token_usage(&self) -> Option<Usage> {
            Some(self.usage)
        }
    }

    #[derive(Clone)]
    struct TextModel;

    #[allow(refining_impl_trait)]
    impl CompletionModel for TextModel {
        type Response = ();
        type StreamingResponse = MockStreamingResponse;
        type Client = ();

        fn make(_: &Self::Client, _: impl Into<String>) -> Self {
            Self
        }

        async fn completion(
            &self,
            _request: CompletionRequest,
        ) -> Result<CompletionResponse<Self::Response>, CompletionError> {
            Err(CompletionError::ProviderError(
                "completion is unused in runner tests".to_string(),
            ))
        }

        async fn stream(
            &self,
            _request: CompletionRequest,
        ) -> Result<StreamingCompletionResponse<Self::StreamingResponse>, CompletionError> {
            let items = vec![
                Ok(RawStreamingChoice::Message("hello ".to_string())),
                Ok(RawStreamingChoice::Message("world".to_string())),
                Ok(RawStreamingChoice::FinalResponse(
                    MockStreamingResponse::new(3, 2),
                )),
            ];
            Ok(StreamingCompletionResponse::stream(Box::pin(
                futures_util::stream::iter(items),
            )))
        }
    }

    #[derive(Clone, Default)]
    struct ToolThenTextModel {
        turn: Arc<AtomicUsize>,
    }

    #[allow(refining_impl_trait)]
    impl CompletionModel for ToolThenTextModel {
        type Response = ();
        type StreamingResponse = MockStreamingResponse;
        type Client = ();

        fn make(_: &Self::Client, _: impl Into<String>) -> Self {
            Self::default()
        }

        async fn completion(
            &self,
            _request: CompletionRequest,
        ) -> Result<CompletionResponse<Self::Response>, CompletionError> {
            Err(CompletionError::ProviderError(
                "completion is unused in runner tests".to_string(),
            ))
        }

        async fn stream(
            &self,
            _request: CompletionRequest,
        ) -> Result<StreamingCompletionResponse<Self::StreamingResponse>, CompletionError> {
            let turn = self.turn.fetch_add(1, Ordering::SeqCst);
            let items = if turn == 0 {
                vec![
                    Ok(RawStreamingChoice::ToolCall(RawStreamingToolCall::new(
                        "tool_call_1".to_string(),
                        "echo_tool".to_string(),
                        serde_json::json!({"value": "ping"}),
                    ))),
                    Ok(RawStreamingChoice::FinalResponse(
                        MockStreamingResponse::new(4, 1),
                    )),
                ]
            } else {
                vec![
                    Ok(RawStreamingChoice::Message("tool done".to_string())),
                    Ok(RawStreamingChoice::FinalResponse(
                        MockStreamingResponse::new(5, 2),
                    )),
                ]
            };
            Ok(StreamingCompletionResponse::stream(Box::pin(
                futures_util::stream::iter(items),
            )))
        }
    }

    #[derive(Clone)]
    struct AlwaysToolModel;

    #[allow(refining_impl_trait)]
    impl CompletionModel for AlwaysToolModel {
        type Response = ();
        type StreamingResponse = MockStreamingResponse;
        type Client = ();

        fn make(_: &Self::Client, _: impl Into<String>) -> Self {
            Self
        }

        async fn completion(
            &self,
            _request: CompletionRequest,
        ) -> Result<CompletionResponse<Self::Response>, CompletionError> {
            Err(CompletionError::ProviderError(
                "completion is unused in runner tests".to_string(),
            ))
        }

        async fn stream(
            &self,
            _request: CompletionRequest,
        ) -> Result<StreamingCompletionResponse<Self::StreamingResponse>, CompletionError> {
            let items = vec![
                Ok(RawStreamingChoice::ToolCall(RawStreamingToolCall::new(
                    "tool_call_loop".to_string(),
                    "echo_tool".to_string(),
                    serde_json::json!({"value": "loop"}),
                ))),
                Ok(RawStreamingChoice::FinalResponse(
                    MockStreamingResponse::new(1, 1),
                )),
            ];
            Ok(StreamingCompletionResponse::stream(Box::pin(
                futures_util::stream::iter(items),
            )))
        }
    }

    #[derive(Clone)]
    struct EchoTool {
        fail: bool,
    }

    impl ToolDyn for EchoTool {
        fn name(&self) -> String {
            "echo_tool".to_string()
        }

        fn definition(
            &self,
            _prompt: String,
        ) -> Pin<Box<dyn Future<Output = ToolDefinition> + Send + '_>> {
            Box::pin(async {
                ToolDefinition {
                    name: "echo_tool".to_string(),
                    description: "Echo test tool".to_string(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "value": {"type": "string"}
                        }
                    }),
                }
            })
        }

        fn call(
            &self,
            _args: String,
        ) -> Pin<Box<dyn Future<Output = Result<String, rig::tool::ToolError>> + Send + '_>>
        {
            Box::pin(async move {
                if self.fail {
                    Err(rig::tool::ToolError::ToolCallError(Box::new(
                        std::io::Error::other("boom"),
                    )))
                } else {
                    Ok("echo ok".to_string())
                }
            })
        }
    }

    fn test_spec(max_iterations: usize, fail_on_tool_error: bool) -> AgentRunSpec {
        AgentRunSpec {
            model: "mock-model".to_string(),
            messages: vec![ChatMessage::user("hello")],
            max_iterations,
            fail_on_tool_error,
            ..AgentRunSpec::default()
        }
    }

    fn test_hook(spec: &AgentRunSpec, cancel: CancellationToken) -> RigPromptHook {
        RigPromptHook::new(spec, Arc::new(Mutex::new(Box::new(NoopRunHooks))), cancel)
    }

    #[tokio::test]
    async fn runner_streams_plain_text_response() {
        let spec = test_spec(3, false);
        let cancel = CancellationToken::new();
        let hook = test_hook(&spec, cancel.clone());

        let outcome = run_with_model(TextModel, &spec, vec![], hook, cancel)
            .await
            .expect("plain text run should succeed");

        assert_eq!(outcome.final_text, "hello world");
        assert_eq!(outcome.usage.prompt_tokens, 3);
        assert_eq!(outcome.usage.completion_tokens, 2);
        assert_eq!(outcome.messages.len(), 2);
    }

    #[tokio::test]
    async fn runner_records_successful_tool_call() {
        let spec = test_spec(3, false);
        let cancel = CancellationToken::new();
        let hook = test_hook(&spec, cancel.clone());

        let outcome = run_with_model(
            ToolThenTextModel::default(),
            &spec,
            vec![Box::new(EchoTool { fail: false })],
            hook.clone(),
            cancel,
        )
        .await
        .expect("tool run should succeed");

        assert_eq!(outcome.final_text, "tool done");
        let events = hook.tool_events().await;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].name, "echo_tool");
        assert_eq!(events[0].status, ToolStatus::Succeeded);
    }

    #[tokio::test]
    async fn runner_reports_max_turns() {
        let spec = test_spec(1, false);
        let cancel = CancellationToken::new();
        let hook = test_hook(&spec, cancel.clone());

        let failure = run_with_model(
            AlwaysToolModel,
            &spec,
            vec![Box::new(EchoTool { fail: false })],
            hook,
            cancel,
        )
        .await
        .expect_err("run should hit max turns");

        assert!(matches!(failure, RunFailure::MaxTurns { .. }));
    }

    #[tokio::test]
    async fn prompt_hook_terminates_when_cancelled() {
        let spec = test_spec(3, false);
        let cancel = CancellationToken::new();
        cancel.cancel();
        let hook = test_hook(&spec, cancel);

        let action = <RigPromptHook as PromptHook<TextModel>>::on_completion_call(
            &hook,
            &Message::user("hello"),
            &[],
        )
        .await;

        assert!(matches!(action, HookAction::Terminate { .. }));
    }

    #[tokio::test]
    async fn strict_tool_error_aborts_run() {
        let spec = test_spec(3, true);
        let cancel = CancellationToken::new();
        let hook = test_hook(&spec, cancel.clone());

        let failure = run_with_model(
            ToolThenTextModel::default(),
            &spec,
            vec![Box::new(EchoTool { fail: true })],
            hook,
            cancel,
        )
        .await
        .expect_err("strict tool failure should abort");

        assert!(matches!(failure, RunFailure::ToolError { .. }));
    }

    #[tokio::test]
    async fn lenient_tool_error_allows_model_retry() {
        let spec = test_spec(3, false);
        let cancel = CancellationToken::new();
        let hook = test_hook(&spec, cancel.clone());

        let outcome = run_with_model(
            ToolThenTextModel::default(),
            &spec,
            vec![Box::new(EchoTool { fail: true })],
            hook.clone(),
            cancel,
        )
        .await
        .expect("lenient tool failure should continue");

        assert_eq!(outcome.final_text, "tool done");
        let events = hook.tool_events().await;
        assert!(matches!(events[0].status, ToolStatus::Failed { .. }));
    }
}
