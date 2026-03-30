use super::config::ProviderConfig;
use super::traits::{
    ChatMessage, ChatRequest, MessageContent, MessageRole, ModelTier, Provider, ProviderResponse,
};
use crate::agent::events::{ProviderEvent, UsageStats};
use crate::error::{AppError, AppResult};
use async_openai::config::OpenAIConfig;
use async_openai::types::chat::{
    ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage,
    ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestToolMessageArgs,
    ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs,
};
use async_openai::Client;
use futures_util::StreamExt;
use std::pin::Pin;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

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

    fn convert_messages(
        &self,
        messages: &[ChatMessage],
        system: Option<&str>,
    ) -> AppResult<Vec<ChatCompletionRequestMessage>> {
        let mut request_messages = Vec::new();

        if let Some(sys) = system {
            request_messages.push(
                ChatCompletionRequestSystemMessageArgs::default()
                    .content(sys)
                    .build()
                    .map_err(|e| AppError::Provider(format!("system message build: {e}")))?
                    .into(),
            );
        }

        for msg in messages {
            match msg.role {
                MessageRole::System => {
                    for part in &msg.content {
                        match part {
                            MessageContent::Text { text } => request_messages.push(
                                ChatCompletionRequestSystemMessageArgs::default()
                                    .content(text.clone())
                                    .build()
                                    .map_err(|e| {
                                        AppError::Provider(format!(
                                            "system message build from content: {e}"
                                        ))
                                    })?
                                    .into(),
                            ),
                            MessageContent::ToolUse { .. } | MessageContent::ToolResult { .. } => {
                                return Err(AppError::Provider(
                                    "system messages only support text content".to_string(),
                                ));
                            }
                        }
                    }
                }
                MessageRole::User => {
                    for part in &msg.content {
                        match part {
                            MessageContent::Text { text } => request_messages.push(
                                ChatCompletionRequestUserMessageArgs::default()
                                    .content(text.clone())
                                    .build()
                                    .map_err(|e| {
                                        AppError::Provider(format!("user message build: {e}"))
                                    })?
                                    .into(),
                            ),
                            MessageContent::ToolResult {
                                tool_use_id,
                                content,
                                ..
                            } => request_messages.push(
                                ChatCompletionRequestToolMessageArgs::default()
                                    .tool_call_id(tool_use_id)
                                    .content(content.clone())
                                    .build()
                                    .map_err(|e| {
                                        AppError::Provider(format!("tool message build: {e}"))
                                    })?
                                    .into(),
                            ),
                            MessageContent::ToolUse { .. } => {
                                return Err(AppError::Provider(
                                    "user messages cannot contain tool use blocks".to_string(),
                                ));
                            }
                        }
                    }
                }
                MessageRole::Assistant => {
                    let text = msg.text_content();
                    if text.is_empty() {
                        return Err(AppError::Provider(
                            "assistant messages without text are not supported yet".to_string(),
                        ));
                    }

                    request_messages.push(
                        ChatCompletionRequestAssistantMessageArgs::default()
                            .content(text)
                            .build()
                            .map_err(|e| {
                                AppError::Provider(format!("assistant message build: {e}"))
                            })?
                            .into(),
                    );
                }
            }
        }

        Ok(request_messages)
    }

    fn build_request(
        &self,
        request: &ChatRequest<'_>,
    ) -> AppResult<CreateChatCompletionRequestArgs> {
        let request_messages = self.convert_messages(request.messages, request.system)?;
        let max_tokens = request.max_tokens.unwrap_or(4_000);

        let mut builder = CreateChatCompletionRequestArgs::default();
        builder
            .model(request.model)
            .messages(request_messages)
            .max_tokens(max_tokens);
        Ok(builder)
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

    async fn chat(&self, request: ChatRequest<'_>) -> AppResult<ProviderResponse> {
        let request = self
            .build_request(&request)?
            .build()
            .map_err(|e| AppError::Provider(format!("request build: {e}")))?;

        let response = self
            .client
            .chat()
            .create(request)
            .await
            .map_err(|e| AppError::Provider(format!("API error: {e}")))?;

        let choice = response
            .choices
            .first()
            .ok_or_else(|| AppError::Provider("no response choices".to_string()))?;

        let usage = response.usage.as_ref();

        Ok(ProviderResponse {
            message: ChatMessage::assistant_text(
                choice.message.content.clone().unwrap_or_default(),
            ),
            usage: UsageStats {
                input_tokens: usage.map(|u| u.prompt_tokens).unwrap_or(0),
                output_tokens: usage.map(|u| u.completion_tokens).unwrap_or(0),
            },
            stop_reason: choice
                .finish_reason
                .map(|reason| format!("{reason:?}"))
                .unwrap_or_else(|| "complete".to_string()),
        })
    }

    async fn chat_stream(
        &self,
        request: ChatRequest<'_>,
    ) -> AppResult<Pin<Box<dyn futures_util::stream::Stream<Item = AppResult<ProviderEvent>> + Send>>>
    {
        let cancel = request.cancel.clone();
        let request = self
            .build_request(&request)?
            .stream(true)
            .build()
            .map_err(|e| AppError::Provider(format!("request build: {e}")))?;

        let mut upstream = self
            .client
            .chat()
            .create_stream(request)
            .await
            .map_err(|e| AppError::Provider(format!("stream creation: {e}")))?;

        let (tx, rx) = mpsc::channel(32);

        tokio::spawn(async move {
            let mut usage = UsageStats {
                input_tokens: 0,
                output_tokens: 0,
            };
            let mut finish_reason: Option<String> = None;

            loop {
                tokio::select! {
                    _ = cancel.cancelled() => {
                        break;
                    }
                    maybe_chunk = upstream.next() => {
                        match maybe_chunk {
                            Some(Ok(chunk)) => {
                                if let Some(chunk_usage) = chunk.usage {
                                    usage.input_tokens = chunk_usage.prompt_tokens;
                                    usage.output_tokens = chunk_usage.completion_tokens;
                                }

                                for choice in chunk.choices {
                                    if let Some(reason) = choice.finish_reason {
                                        finish_reason = Some(format!("{reason:?}"));
                                    }

                                    if let Some(content) = choice.delta.content {
                                        if !content.is_empty()
                                            && tx.send(Ok(ProviderEvent::TextDelta { text: content })).await.is_err()
                                        {
                                            return;
                                        }
                                    }
                                }
                            }
                            Some(Err(error)) => {
                                let _ = tx
                                    .send(Err(AppError::Provider(format!("stream error: {error}"))))
                                    .await;
                                return;
                            }
                            None => {
                                let _ = tx
                                    .send(Ok(ProviderEvent::Finished {
                                        stop_reason: finish_reason
                                            .unwrap_or_else(|| "complete".to_string()),
                                        usage,
                                    }))
                                    .await;
                                return;
                            }
                        }
                    }
                }
            }
        });

        Ok(Box::pin(ReceiverStream::new(rx)))
    }
}
