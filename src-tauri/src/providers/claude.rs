use super::config::*;
use super::traits::{ChatMessage, ModelTier, Provider, ProviderResponse};
use crate::error::AppResult;

#[allow(dead_code)]
pub struct ClaudeProvider {
    api_key: String,
    tier: ModelTier,
}

impl ClaudeProvider {
    pub fn new(api_key: String, tier: ModelTier) -> Self {
        Self { api_key, tier }
    }

    #[allow(dead_code)]
    fn model_id(&self) -> &str {
        match self.tier {
            ModelTier::Fast => MODEL_HAIKU,
            ModelTier::Smart => MODEL_SONNET,
        }
    }
}

#[async_trait::async_trait]
impl Provider for ClaudeProvider {
    fn name(&self) -> &str {
        "claude"
    }

    fn tier(&self) -> ModelTier {
        self.tier.clone()
    }

    async fn chat(
        &self,
        _messages: Vec<ChatMessage>,
        _system: Option<&str>,
        _max_tokens: u32,
    ) -> AppResult<ProviderResponse> {
        todo!("实现 Claude API 调用")
    }

    async fn chat_stream(
        &self,
        _messages: Vec<ChatMessage>,
        _system: Option<&str>,
        _max_tokens: u32,
        _on_token: impl Fn(&str) + Send + 'static,
    ) -> AppResult<ProviderResponse> {
        todo!("实现 Claude 流式 API 调用")
    }
}
