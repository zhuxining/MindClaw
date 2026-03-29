use super::config::ProviderConfig;
use super::traits::{ChatMessage, ModelTier, Provider, ProviderResponse};
use crate::error::AppResult;

#[allow(dead_code)]
pub struct ClaudeProvider {
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
            api_key,
            model_id: selected.to_string(),
            tier,
        }
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
        _on_token: Box<dyn Fn(String) + Send + Sync>,
    ) -> AppResult<ProviderResponse> {
        todo!("实现 Claude 流式 API 调用")
    }
}
