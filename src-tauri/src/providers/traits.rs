use crate::error::AppResult;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ModelTier {
    /// 快速轻量（Claude Haiku）
    Fast,
    /// 高质量推理（Claude Sonnet）
    Smart,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    /// Tool call ID for tool result messages (role="tool")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl ChatMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
            tool_call_id: None,
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
            tool_call_id: None,
        }
    }

    pub fn tool_result(content: impl Into<String>, tool_call_id: impl Into<String>) -> Self {
        Self {
            role: "tool".to_string(),
            content: content.into(),
            tool_call_id: Some(tool_call_id.into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderResponse {
    pub content: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub stop_reason: String,
}

/// AI Provider trait：所有 LLM 接入实现此接口
#[async_trait::async_trait]
pub trait Provider: Send + Sync {
    /// Provider 名称（如 "openai", "deepseek", "claude"）
    fn name(&self) -> &str;
    /// 当前使用的模型 ID（如 "gpt-4o", "deepseek-chat"）
    fn model_id(&self) -> &str;
    fn tier(&self) -> ModelTier;

    async fn chat(
        &self,
        messages: Vec<ChatMessage>,
        system: Option<&str>,
        max_tokens: u32,
    ) -> AppResult<ProviderResponse>;

    async fn chat_stream(
        &self,
        messages: Vec<ChatMessage>,
        system: Option<&str>,
        max_tokens: u32,
        on_token: Box<dyn Fn(String) + Send + Sync>,
    ) -> AppResult<ProviderResponse>;
}
