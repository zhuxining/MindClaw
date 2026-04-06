use crate::agent::events::{ProviderEvent, ProviderUsage};
use crate::error::AppResult;
use futures_util::stream::Stream;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::pin::Pin;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ModelTier {
    /// 快速轻量（Claude Haiku）
    Fast,
    /// 高质量推理（Claude Sonnet）
    Smart,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MessageContent {
    Text {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
    ToolResult {
        tool_use_id: String,
        content: String,
        is_error: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: Vec<MessageContent>,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: vec![MessageContent::Text {
                text: content.into(),
            }],
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: vec![MessageContent::Text {
                text: content.into(),
            }],
        }
    }

    pub fn assistant_text(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: vec![MessageContent::Text {
                text: content.into(),
            }],
        }
    }

    pub fn assistant_parts(content: Vec<MessageContent>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content,
        }
    }

    pub fn tool_result(
        content: impl Into<String>,
        tool_call_id: impl Into<String>,
        is_error: bool,
    ) -> Self {
        Self {
            role: MessageRole::User,
            content: vec![MessageContent::ToolResult {
                tool_use_id: tool_call_id.into(),
                content: content.into(),
                is_error,
            }],
        }
    }

    pub fn text_content(&self) -> String {
        self.content
            .iter()
            .filter_map(|part| match part {
                MessageContent::Text { text } => Some(text.as_str()),
                MessageContent::ToolUse { .. } | MessageContent::ToolResult { .. } => None,
            })
            .collect::<Vec<_>>()
            .join("")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderResponse {
    pub message: ChatMessage,
    pub usage: ProviderUsage,
    pub stop_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolChoice {
    Auto,
    None,
    Specific(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchema {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

pub struct ChatRequest<'a> {
    pub model: &'a str,
    pub messages: &'a [ChatMessage],
    pub system: Option<&'a str>,
    pub tools: &'a [ToolSchema],
    pub tool_choice: ToolChoice,
    pub max_tokens: Option<u32>,
    pub cancel: CancellationToken,
}

/// AI Provider trait：所有 LLM 接入实现此接口
#[async_trait::async_trait]
pub trait Provider: Send + Sync {
    /// Provider 名称（如 "openai", "deepseek", "claude"）
    fn name(&self) -> &str;
    /// 当前使用的模型 ID（如 "gpt-4o", "deepseek-chat"）
    fn model_id(&self) -> &str;
    fn tier(&self) -> ModelTier;

    async fn chat(&self, request: ChatRequest<'_>) -> AppResult<ProviderResponse>;

    async fn chat_stream(
        &self,
        request: ChatRequest<'_>,
    ) -> AppResult<Pin<Box<dyn Stream<Item = AppResult<ProviderEvent>> + Send>>>;

    fn supports_streaming(&self) -> bool {
        true
    }
}
