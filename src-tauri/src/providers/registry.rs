//! Rig-based Provider Registry
//!
//! 完全使用 rig 内置 providers。
//! - LLMClient 持有 rig provider client
//! - LLMCompletionModel 持有 rig completion model
//! - 配置管理保留（ProviderConfig）
//!
//! ProviderRegistry 只解析主模型/轻量模型，不封装 complete()/stream()。
//! Agent 构建、工具执行、streaming 和消息转换均属于 AgentRunner。

use std::collections::HashMap;

use super::config::{builtin_configs, ProviderConfig};
use crate::error::AppResult;
use crate::runtime::config::AppConfig;
use rig::client::CompletionClient;
use rig::providers::{anthropic, deepseek, openai};

// ============================================================================
// LLMClient - rig provider client enum
// ============================================================================

pub enum LLMClient {
    Anthropic(anthropic::Client),
    OpenAI(openai::Client),
    DeepSeek(deepseek::Client),
}

impl LLMClient {
    /// 创建 completion model
    pub fn completion_model(&self, model_id: &str) -> AppResult<LLMCompletionModel> {
        match self {
            LLMClient::Anthropic(client) => Ok(LLMCompletionModel::Anthropic(
                client.completion_model(model_id),
            )),
            LLMClient::OpenAI(client) => Ok(LLMCompletionModel::OpenAI(
                client.clone().completions_api().completion_model(model_id),
            )),
            LLMClient::DeepSeek(client) => Ok(LLMCompletionModel::DeepSeek(
                client.completion_model(model_id),
            )),
        }
    }
}

// ============================================================================
// LLMCompletionModel - rig completion model enum
// ============================================================================
// 注意：不同 provider 的 CompletionModel 类型路径不同：
// - anthropic: 需要通过 completion_model() 方法返回，类型不直接公开
// - openai: openai::CompletionModel 可用（通过 pub use completion::*）
// - deepseek: deepseek::CompletionModel 直接定义在模块中

#[derive(Clone)]
pub enum LLMCompletionModel {
    Anthropic(anthropic::completion::CompletionModel),
    OpenAI(openai::CompletionModel),
    DeepSeek(deepseek::CompletionModel),
}

// ============================================================================
// AgentModelSet - AgentRunner 使用的模型集合
// ============================================================================

/// AgentRunner 使用的模型集合。
///
/// ProviderRegistry 在这里完成 provider/model 解析；Runner 只看到主模型和轻量模型。
#[derive(Clone)]
pub struct AgentModelSet {
    pub main: LLMCompletionModel,
    pub light: LLMCompletionModel,
    pub main_model_id: String,
    pub light_model_id: String,
}

impl AgentModelSet {
    pub fn model_for(&self, model_id: &str) -> LLMCompletionModel {
        if model_id == self.light_model_id {
            self.light.clone()
        } else {
            self.main.clone()
        }
    }
}

// ============================================================================
// ProviderRegistry - 配置管理和 client 创建
// ============================================================================

pub struct ProviderRegistry {
    configs: HashMap<String, ProviderConfig>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        let mut configs = HashMap::new();
        for config in builtin_configs() {
            configs.insert(config.name.clone(), config);
        }
        Self { configs }
    }

    pub fn register(&mut self, config: ProviderConfig) {
        self.configs.insert(config.name.clone(), config);
    }

    pub fn available_providers(&self) -> Vec<&str> {
        self.configs.keys().map(|s| s.as_str()).collect()
    }

    fn create_client(&self, provider_name: &str, api_key: &str) -> AppResult<LLMClient> {
        match provider_name {
            "anthropic" | "claude" => {
                let client = anthropic::Client::new(api_key).map_err(|e| {
                    crate::error::AppError::Provider(format!("anthropic client error: {}", e))
                })?;
                Ok(LLMClient::Anthropic(client))
            }
            "openai" => {
                let client = openai::Client::new(api_key).map_err(|e| {
                    crate::error::AppError::Provider(format!("openai client error: {}", e))
                })?;
                Ok(LLMClient::OpenAI(client))
            }
            "deepseek" => {
                let client = deepseek::Client::new(api_key).map_err(|e| {
                    crate::error::AppError::Provider(format!("deepseek client error: {}", e))
                })?;
                Ok(LLMClient::DeepSeek(client))
            }
            other => Err(crate::error::AppError::Provider(format!(
                "unsupported provider: {}",
                other
            ))),
        }
    }

    pub fn create_client_from_env(&self, provider_name: &str) -> AppResult<LLMClient> {
        let config = self.configs.get(provider_name).ok_or_else(|| {
            crate::error::AppError::Provider(format!("unknown provider: {}", provider_name))
        })?;

        let api_key = std::env::var(&config.api_key_env).map_err(|_| {
            crate::error::AppError::Provider(format!("{} not set", config.api_key_env))
        })?;

        self.create_client(provider_name, &api_key)
    }

    pub fn create_agent_models_from_env(&self, config: &AppConfig) -> AppResult<AgentModelSet> {
        let provider_config = self.configs.get(&config.provider_id).ok_or_else(|| {
            crate::error::AppError::Provider(format!("unknown provider: {}", config.provider_id))
        })?;

        let main_model_id = config
            .model_id
            .clone()
            .unwrap_or_else(|| provider_config.default_model.clone());
        let light_model_id = config
            .light_model_id
            .clone()
            .unwrap_or_else(|| main_model_id.clone());

        let client = self.create_client_from_env(&config.provider_id)?;
        let main = client.completion_model(&main_model_id)?;
        let light = if light_model_id == main_model_id {
            main.clone()
        } else {
            client.completion_model(&light_model_id)?
        };

        Ok(AgentModelSet {
            main,
            light,
            main_model_id,
            light_model_id,
        })
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}
