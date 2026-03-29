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
pub trait Provider {
    fn name(&self) -> &str;
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
        on_token: impl Fn(&str) + Send + 'static,
    ) -> AppResult<ProviderResponse>;
}
