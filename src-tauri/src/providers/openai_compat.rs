use super::config::ProviderConfig;
use super::traits::{ChatMessage, ModelTier, Provider, ProviderResponse};
use crate::error::{AppError, AppResult};
use async_openai::config::OpenAIConfig;
use async_openai::types::chat::{
    ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage,
    ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestToolMessageArgs,
    ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs,
};
use async_openai::Client;

/// 通用 OpenAI 兼容 Provider，支持所有 OpenAI 兼容 API（OpenAI、DeepSeek、Moonshot 等）
pub struct OpenAICompatProvider {
    client: Client<OpenAIConfig>,
    provider_name: String,
    model_id: String,
    tier: ModelTier,
}

impl OpenAICompatProvider {
    /// 从 ProviderConfig 创建，指定 API key 和可选的模型 ID
    pub fn new(
        config: &ProviderConfig,
        api_key: String,
        model_id: Option<&str>,
    ) -> AppResult<Self> {
        let selected_model = model_id.unwrap_or(&config.default_model);
        let tier = config
            .models
            .iter()
            .find(|m| m.id == selected_model)
            .map(|m| m.tier.clone())
            .unwrap_or(ModelTier::Smart);

        let openai_config = OpenAIConfig::new()
            .with_api_key(api_key)
            .with_api_base(&config.api_base);
        let client = Client::with_config(openai_config);

        Ok(Self {
            client,
            provider_name: config.name.clone(),
            model_id: selected_model.to_string(),
            tier,
        })
    }

    /// 从环境变量读取 API key 创建
    pub fn from_env(config: &ProviderConfig, model_id: Option<&str>) -> AppResult<Self> {
        let env_var = config.api_key_env.as_deref().ok_or_else(|| {
            AppError::Provider(format!(
                "provider '{}' has no api_key_env configured",
                config.name
            ))
        })?;
        let api_key = std::env::var(env_var)
            .map_err(|_| AppError::Provider(format!("{} not set", env_var)))?;
        Self::new(config, api_key, model_id)
    }

    /// 将内部 ChatMessage 转换为 async-openai 请求消息
    fn convert_messages(
        &self,
        messages: Vec<ChatMessage>,
        system: Option<&str>,
    ) -> AppResult<Vec<ChatCompletionRequestMessage>> {
        let mut request_messages: Vec<ChatCompletionRequestMessage> = Vec::new();

        if let Some(sys) = system {
            request_messages.push(
                ChatCompletionRequestSystemMessageArgs::default()
                    .content(sys)
                    .build()
                    .map_err(|e| AppError::Provider(format!("system message build: {}", e)))?
                    .into(),
            );
        }

        for msg in messages {
            let req_msg = match msg.role.as_str() {
                "user" => ChatCompletionRequestUserMessageArgs::default()
                    .content(msg.content)
                    .build()
                    .map_err(|e| AppError::Provider(format!("user message build: {}", e)))?
                    .into(),
                "assistant" => ChatCompletionRequestAssistantMessageArgs::default()
                    .content(msg.content)
                    .build()
                    .map_err(|e| AppError::Provider(format!("assistant message build: {}", e)))?
                    .into(),
                "tool" => {
                    let tool_call_id = msg.tool_call_id.unwrap_or_default();
                    ChatCompletionRequestToolMessageArgs::default()
                        .content(msg.content)
                        .tool_call_id(tool_call_id)
                        .build()
                        .map_err(|e| AppError::Provider(format!("tool message build: {}", e)))?
                        .into()
                }
                role => return Err(AppError::Provider(format!("unknown role: {}", role))),
            };
            request_messages.push(req_msg);
        }

        Ok(request_messages)
    }
}

#[async_trait::async_trait]
impl Provider for OpenAICompatProvider {
    fn name(&self) -> &str {
        &self.provider_name
    }

    fn model_id(&self) -> &str {
        &self.model_id
    }

    fn tier(&self) -> ModelTier {
        self.tier.clone()
    }

    async fn chat(
        &self,
        messages: Vec<ChatMessage>,
        system: Option<&str>,
        max_tokens: u32,
    ) -> AppResult<ProviderResponse> {
        let request_messages = self.convert_messages(messages, system)?;

        let request = CreateChatCompletionRequestArgs::default()
            .model(&*self.model_id)
            .messages(request_messages)
            .max_tokens(max_tokens)
            .build()
            .map_err(|e| AppError::Provider(format!("request build: {}", e)))?;

        let response = self
            .client
            .chat()
            .create(request)
            .await
            .map_err(|e| AppError::Provider(format!("API error: {}", e)))?;

        let choice = response
            .choices
            .first()
            .ok_or_else(|| AppError::Provider("no response choices".to_string()))?;

        let usage = response.usage.as_ref();
        let input_tokens = usage.map(|u| u.prompt_tokens).unwrap_or(0);
        let output_tokens = usage.map(|u| u.completion_tokens).unwrap_or(0);

        Ok(ProviderResponse {
            content: choice.message.content.clone().unwrap_or_default(),
            input_tokens,
            output_tokens,
            stop_reason: choice
                .finish_reason
                .map(|r| format!("{:?}", r))
                .unwrap_or_else(|| "complete".to_string()),
        })
    }

    async fn chat_stream(
        &self,
        messages: Vec<ChatMessage>,
        system: Option<&str>,
        max_tokens: u32,
        on_token: Box<dyn Fn(String) + Send + Sync>,
    ) -> AppResult<ProviderResponse> {
        use futures_util::StreamExt;

        let request_messages = self.convert_messages(messages, system)?;

        let request = CreateChatCompletionRequestArgs::default()
            .model(&*self.model_id)
            .messages(request_messages)
            .max_tokens(max_tokens)
            .stream(true)
            .build()
            .map_err(|e| AppError::Provider(format!("request build: {}", e)))?;

        let mut stream = self
            .client
            .chat()
            .create_stream(request)
            .await
            .map_err(|e| AppError::Provider(format!("stream creation: {}", e)))?;

        let mut text_accumulator = String::new();
        let mut input_tokens = 0u32;
        let mut output_tokens = 0u32;
        let mut finish_reason: Option<String> = None;

        while let Some(chunk_result) = stream.next().await {
            let chunk =
                chunk_result.map_err(|e| AppError::Provider(format!("stream error: {}", e)))?;

            if let Some(usage) = chunk.usage {
                input_tokens = usage.prompt_tokens;
                output_tokens = usage.completion_tokens;
            }

            for choice in &chunk.choices {
                if let Some(reason) = &choice.finish_reason {
                    finish_reason = Some(format!("{:?}", reason));
                }

                let delta = &choice.delta;
                if let Some(content) = &delta.content {
                    if !content.is_empty() {
                        on_token(content.clone());
                        text_accumulator.push_str(content);
                    }
                }
            }
        }

        Ok(ProviderResponse {
            content: text_accumulator,
            input_tokens,
            output_tokens,
            stop_reason: finish_reason.unwrap_or_else(|| "complete".to_string()),
        })
    }
}
