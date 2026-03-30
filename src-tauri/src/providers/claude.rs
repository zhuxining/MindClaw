use super::config::ProviderConfig;
use super::traits::{
    ChatMessage, ChatRequest, MessageContent, MessageRole, ModelTier, Provider, ProviderResponse,
    ToolChoice, ToolSchema,
};
use crate::agent::events::{ProviderEvent, UsageStats};
use crate::error::{AppError, AppResult};
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::pin::Pin;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

const API_VERSION: &str = "2023-06-01";

// ── Anthropic API request types ─────────────────────────────────

#[derive(Serialize)]
struct MessagesRequest<'a> {
    model: &'a str,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<&'a str>,
    messages: Vec<ApiMessage>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<ApiTool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<ApiToolChoice>,
    stream: bool,
}

#[derive(Serialize)]
struct ApiMessage {
    role: &'static str,
    content: Vec<ApiContent>,
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum ApiContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: String,
        #[serde(skip_serializing_if = "std::ops::Not::not")]
        is_error: bool,
    },
}

#[derive(Serialize)]
struct ApiTool {
    name: String,
    description: String,
    input_schema: Value,
}

#[derive(Serialize)]
struct ApiToolChoice {
    r#type: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
}

// ── Anthropic API response types (non-streaming) ────────────────

#[derive(Deserialize)]
struct MessagesResponse {
    content: Vec<ResponseContent>,
    stop_reason: Option<String>,
    usage: ApiUsage,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum ResponseContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
}

#[derive(Deserialize, Default)]
struct ApiUsage {
    #[serde(default)]
    input_tokens: u32,
    #[serde(default)]
    output_tokens: u32,
}

// ── Anthropic SSE event types ───────────────────────────────────

#[derive(Deserialize)]
struct SseMessageStart {
    message: SseMessageMeta,
}

#[derive(Deserialize)]
struct SseMessageMeta {
    usage: ApiUsage,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum SseContentBlock {
    #[serde(rename = "text")]
    Text { _text: String },
    #[serde(rename = "tool_use")]
    ToolUse { id: String, name: String },
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum SseContentDelta {
    #[serde(rename = "text_delta")]
    TextDelta { text: String },
    #[serde(rename = "input_json_delta")]
    InputJsonDelta { partial_json: String },
}

#[derive(Deserialize)]
struct SseMessageDelta {
    delta: SseMessageDeltaInner,
    usage: Option<ApiUsage>,
}

#[derive(Deserialize)]
struct SseMessageDeltaInner {
    stop_reason: Option<String>,
}

#[derive(Deserialize)]
struct ApiErrorResponse {
    error: ApiErrorDetail,
}

#[derive(Deserialize)]
struct ApiErrorDetail {
    message: String,
}

// ── ClaudeProvider ──────────────────────────────────────────────

pub struct ClaudeProvider {
    client: Client,
    api_base: String,
    api_key: String,
    model_id: String,
    tier: ModelTier,
}

impl ClaudeProvider {
    pub fn new(config: &ProviderConfig, api_key: String, model_id: Option<&str>) -> Self {
        let selected = model_id.unwrap_or(&config.default_model);
        let tier = config
            .models
            .iter()
            .find(|m| m.id == selected)
            .map(|m| m.tier.clone())
            .unwrap_or(ModelTier::Smart);
        Self {
            client: Client::new(),
            api_base: config.api_base.clone(),
            api_key,
            model_id: selected.to_string(),
            tier,
        }
    }

    pub fn from_env(config: &ProviderConfig, model_id: Option<&str>) -> AppResult<Self> {
        let env_var = config.api_key_env.as_deref().ok_or_else(|| {
            AppError::Provider(format!(
                "provider '{}' has no api_key_env configured",
                config.name
            ))
        })?;
        let api_key =
            std::env::var(env_var).map_err(|_| AppError::Provider(format!("{env_var} not set")))?;
        Ok(Self::new(config, api_key, model_id))
    }

    fn convert_messages(messages: &[ChatMessage]) -> Vec<ApiMessage> {
        let mut api_messages: Vec<ApiMessage> = Vec::new();

        for msg in messages {
            let role = match msg.role {
                MessageRole::User | MessageRole::System => "user",
                MessageRole::Assistant => "assistant",
            };

            let content: Vec<ApiContent> = msg
                .content
                .iter()
                .map(|part| match part {
                    MessageContent::Text { text } => ApiContent::Text { text: text.clone() },
                    MessageContent::ToolUse { id, name, input } => ApiContent::ToolUse {
                        id: id.clone(),
                        name: name.clone(),
                        input: input.clone(),
                    },
                    MessageContent::ToolResult {
                        tool_use_id,
                        content,
                        is_error,
                    } => ApiContent::ToolResult {
                        tool_use_id: tool_use_id.clone(),
                        content: content.clone(),
                        is_error: *is_error,
                    },
                })
                .collect();

            // Anthropic API 要求交替 user/assistant，合并相邻同角色消息
            if let Some(last) = api_messages.last_mut() {
                if last.role == role {
                    last.content.extend(content);
                    continue;
                }
            }

            api_messages.push(ApiMessage { role, content });
        }

        api_messages
    }

    fn convert_tools(tools: &[ToolSchema]) -> Vec<ApiTool> {
        tools
            .iter()
            .map(|t| ApiTool {
                name: t.name.clone(),
                description: t.description.clone(),
                input_schema: t.parameters.clone(),
            })
            .collect()
    }

    fn convert_tool_choice(choice: &ToolChoice) -> Option<ApiToolChoice> {
        match choice {
            ToolChoice::Auto => Some(ApiToolChoice {
                r#type: "auto",
                name: None,
            }),
            ToolChoice::None => Some(ApiToolChoice {
                r#type: "none",
                name: None,
            }),
            ToolChoice::Specific(name) => Some(ApiToolChoice {
                r#type: "tool",
                name: Some(name.clone()),
            }),
        }
    }

    fn build_request_body<'a>(
        &'a self,
        request: &'a ChatRequest<'_>,
        stream: bool,
    ) -> MessagesRequest<'a> {
        let max_tokens = request.max_tokens.unwrap_or(8_192);
        let api_tools = Self::convert_tools(request.tools);
        let tool_choice = if api_tools.is_empty() {
            None
        } else {
            Self::convert_tool_choice(&request.tool_choice)
        };

        MessagesRequest {
            model: request.model,
            max_tokens,
            system: request.system,
            messages: Self::convert_messages(request.messages),
            tools: api_tools,
            tool_choice,
            stream,
        }
    }

    fn messages_url(&self) -> String {
        format!("{}/v1/messages", self.api_base)
    }
}

#[async_trait::async_trait]
impl Provider for ClaudeProvider {
    fn name(&self) -> &str {
        "claude"
    }

    fn model_id(&self) -> &str {
        &self.model_id
    }

    fn tier(&self) -> ModelTier {
        self.tier.clone()
    }

    async fn chat(&self, request: ChatRequest<'_>) -> AppResult<ProviderResponse> {
        let body = self.build_request_body(&request, false);

        let resp = self
            .client
            .post(self.messages_url())
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", API_VERSION)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::Provider(format!("HTTP error: {e}")))?;

        let status = resp.status();
        let resp_text = resp
            .text()
            .await
            .map_err(|e| AppError::Provider(format!("read body: {e}")))?;

        if !status.is_success() {
            let detail = serde_json::from_str::<ApiErrorResponse>(&resp_text)
                .map(|e| e.error.message)
                .unwrap_or(resp_text);
            return Err(AppError::Provider(format!("API {status}: {detail}")));
        }

        let api_resp: MessagesResponse = serde_json::from_str(&resp_text)
            .map_err(|e| AppError::Provider(format!("parse response: {e}")))?;

        let mut content_parts = Vec::new();
        for block in &api_resp.content {
            match block {
                ResponseContent::Text { text } => {
                    content_parts.push(MessageContent::Text { text: text.clone() });
                }
                ResponseContent::ToolUse { id, name, input } => {
                    content_parts.push(MessageContent::ToolUse {
                        id: id.clone(),
                        name: name.clone(),
                        input: input.clone(),
                    });
                }
            }
        }

        Ok(ProviderResponse {
            message: ChatMessage::assistant_parts(content_parts),
            usage: UsageStats {
                input_tokens: api_resp.usage.input_tokens,
                output_tokens: api_resp.usage.output_tokens,
            },
            stop_reason: api_resp.stop_reason.unwrap_or_else(|| "end_turn".into()),
        })
    }

    async fn chat_stream(
        &self,
        request: ChatRequest<'_>,
    ) -> AppResult<Pin<Box<dyn futures_util::stream::Stream<Item = AppResult<ProviderEvent>> + Send>>>
    {
        let cancel = request.cancel.clone();
        let body = self.build_request_body(&request, true);

        let resp = self
            .client
            .post(self.messages_url())
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", API_VERSION)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::Provider(format!("HTTP error: {e}")))?;

        let status = resp.status();
        if !status.is_success() {
            let resp_text = resp
                .text()
                .await
                .map_err(|e| AppError::Provider(format!("read error body: {e}")))?;
            let detail = serde_json::from_str::<ApiErrorResponse>(&resp_text)
                .map(|e| e.error.message)
                .unwrap_or(resp_text);
            return Err(AppError::Provider(format!("API {status}: {detail}")));
        }

        let mut byte_stream = resp.bytes_stream();
        let (tx, rx) = mpsc::channel(32);

        tokio::spawn(async move {
            let mut usage = UsageStats {
                input_tokens: 0,
                output_tokens: 0,
            };
            let mut stop_reason = String::from("end_turn");

            // Tool call buffering: accumulate partial JSON per content block index
            let mut tool_blocks: Vec<(String, String, String)> = Vec::new(); // (id, name, json_buf)
            let mut current_block_index: Option<usize> = None;
            let mut line_buf = String::new();

            loop {
                tokio::select! {
                    _ = cancel.cancelled() => break,
                    maybe_chunk = byte_stream.next() => {
                        let chunk = match maybe_chunk {
                            Some(Ok(bytes)) => bytes,
                            Some(Err(e)) => {
                                let _ = tx.send(Err(AppError::Provider(format!("stream read: {e}")))).await;
                                return;
                            }
                            None => {
                                // Stream ended — emit any buffered tool calls + Finished
                                for (id, name, json_buf) in &tool_blocks {
                                    let arguments_json = serde_json::from_str(json_buf)
                                        .unwrap_or(Value::Object(Default::default()));
                                    if tx.send(Ok(ProviderEvent::ToolCall {
                                        id: id.clone(),
                                        name: name.clone(),
                                        arguments_json,
                                    })).await.is_err() {
                                        return;
                                    }
                                }
                                let _ = tx.send(Ok(ProviderEvent::Finished { stop_reason, usage })).await;
                                return;
                            }
                        };

                        let text = String::from_utf8_lossy(&chunk);
                        line_buf.push_str(&text);

                        // Process complete SSE lines
                        while let Some(newline_pos) = line_buf.find('\n') {
                            let line = line_buf[..newline_pos].trim_end_matches('\r').to_string();
                            line_buf = line_buf[newline_pos + 1..].to_string();

                            if line.is_empty() || line.starts_with(':') {
                                continue;
                            }

                            if let Some(event_type) = line.strip_prefix("event: ") {
                                current_block_index = match event_type {
                                    "content_block_start" | "content_block_delta" | "content_block_stop" => current_block_index,
                                    _ => None,
                                };
                                // Event type is used contextually below when processing data
                                continue;
                            }

                            let Some(data) = line.strip_prefix("data: ") else {
                                continue;
                            };

                            let Ok(json) = serde_json::from_str::<Value>(data) else {
                                continue;
                            };

                            let event_type = json.get("type").and_then(|v| v.as_str()).unwrap_or("");

                            match event_type {
                                "message_start" => {
                                    if let Ok(msg) = serde_json::from_value::<SseMessageStart>(json) {
                                        usage.input_tokens = msg.message.usage.input_tokens;
                                        usage.output_tokens = msg.message.usage.output_tokens;
                                    }
                                }
                                "content_block_start" => {
                                    let index = json.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                                    current_block_index = Some(index);

                                    if let Some(cb) = json.get("content_block") {
                                        if let Ok(block) = serde_json::from_value::<SseContentBlock>(cb.clone()) {
                                            match block {
                                                SseContentBlock::Text { .. } => {}
                                                SseContentBlock::ToolUse { id, name } => {
                                                    // Ensure tool_blocks has capacity
                                                    while tool_blocks.len() <= index {
                                                        tool_blocks.push((String::new(), String::new(), String::new()));
                                                    }
                                                    tool_blocks[index] = (id, name, String::new());
                                                }
                                            }
                                        }
                                    }
                                }
                                "content_block_delta" => {
                                    let index = json.get("index").and_then(|v| v.as_u64())
                                        .or_else(|| current_block_index.map(|i| i as u64))
                                        .unwrap_or(0) as usize;

                                    if let Some(delta) = json.get("delta") {
                                        if let Ok(d) = serde_json::from_value::<SseContentDelta>(delta.clone()) {
                                            match d {
                                                SseContentDelta::TextDelta { text } => {
                                                    if tx.send(Ok(ProviderEvent::TextDelta { text })).await.is_err() {
                                                        return;
                                                    }
                                                }
                                                SseContentDelta::InputJsonDelta { partial_json } => {
                                                    if let Some(tb) = tool_blocks.get_mut(index) {
                                                        tb.2.push_str(&partial_json);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                "content_block_stop" => {
                                    // Tool call will be emitted at stream end
                                    let index = json.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

                                    // Emit completed tool call immediately
                                    if let Some(tb) = tool_blocks.get(index) {
                                        if !tb.0.is_empty() {
                                            let arguments_json = serde_json::from_str(&tb.2)
                                                .unwrap_or(Value::Object(Default::default()));
                                            if tx.send(Ok(ProviderEvent::ToolCall {
                                                id: tb.0.clone(),
                                                name: tb.1.clone(),
                                                arguments_json,
                                            })).await.is_err() {
                                                return;
                                            }
                                        }
                                    }
                                }
                                "message_delta" => {
                                    if let Ok(md) = serde_json::from_value::<SseMessageDelta>(json) {
                                        if let Some(reason) = md.delta.stop_reason {
                                            stop_reason = reason;
                                        }
                                        if let Some(u) = md.usage {
                                            usage.output_tokens = u.output_tokens;
                                        }
                                    }
                                }
                                "message_stop" => {
                                    // Finished — no more tool calls to buffer since we emit on content_block_stop
                                    let _ = tx.send(Ok(ProviderEvent::Finished { stop_reason, usage })).await;
                                    return;
                                }
                                "error" => {
                                    let msg = json.get("error")
                                        .and_then(|e| e.get("message"))
                                        .and_then(|m| m.as_str())
                                        .unwrap_or("unknown stream error");
                                    let _ = tx.send(Err(AppError::Provider(msg.to_string()))).await;
                                    return;
                                }
                                _ => {} // ping, etc.
                            }
                        }
                    }
                }
            }
        });

        Ok(Box::pin(ReceiverStream::new(rx)))
    }
}
