use std::collections::HashMap;
use std::sync::Arc;

use super::claude::ClaudeProvider;
use super::config::{builtin_configs, ModelConfig, ProviderConfig};
use super::openai_compat::OpenAICompatProvider;
use super::traits::{ModelTier, Provider};
use crate::error::{AppError, AppResult};

/// Provider 注册表：管理所有提供商配置，提供工厂方法
pub struct ProviderRegistry {
    configs: HashMap<String, ProviderConfig>,
}

impl ProviderRegistry {
    /// 创建注册表并加载内置提供商配置
    pub fn new() -> Self {
        let mut configs = HashMap::new();
        for config in builtin_configs() {
            configs.insert(config.name.clone(), config);
        }
        Self { configs }
    }

    /// 注册自定义提供商配置
    pub fn register(&mut self, config: ProviderConfig) {
        self.configs.insert(config.name.clone(), config);
    }

    /// 列出所有可用的提供商名称
    pub fn available_providers(&self) -> Vec<&str> {
        self.configs.keys().map(|s| s.as_str()).collect()
    }

    /// 获取指定提供商的模型列表
    pub fn models_for(&self, provider_name: &str) -> Option<&[ModelConfig]> {
        self.configs.get(provider_name).map(|c| c.models.as_slice())
    }

    /// 获取指定提供商的配置
    pub fn get_config(&self, provider_name: &str) -> Option<&ProviderConfig> {
        self.configs.get(provider_name)
    }

    /// 按 tier 查找指定提供商的模型
    pub fn resolve_model_by_tier(
        &self,
        provider_name: &str,
        tier: ModelTier,
    ) -> Option<&ModelConfig> {
        self.configs
            .get(provider_name)?
            .models
            .iter()
            .find(|m| m.tier == tier)
    }

    /// 创建 Provider 实例（显式指定 API key）
    pub fn create_provider(
        &self,
        provider_name: &str,
        api_key: String,
        model_id: Option<&str>,
    ) -> AppResult<Arc<dyn Provider>> {
        let config = self
            .configs
            .get(provider_name)
            .ok_or_else(|| AppError::Provider(format!("unknown provider: {}", provider_name)))?;

        if provider_name == "claude" {
            Ok(Arc::new(ClaudeProvider::new(config, api_key, model_id)))
        } else {
            Ok(Arc::new(OpenAICompatProvider::new(
                config, api_key, model_id,
            )?))
        }
    }

    /// 创建 Provider 实例（从环境变量读取 API key）
    pub fn create_provider_from_env(
        &self,
        provider_name: &str,
        model_id: Option<&str>,
    ) -> AppResult<Arc<dyn Provider>> {
        let config = self
            .configs
            .get(provider_name)
            .ok_or_else(|| AppError::Provider(format!("unknown provider: {}", provider_name)))?;

        let env_var = config.api_key_env.as_deref().ok_or_else(|| {
            AppError::Provider(format!(
                "provider '{}' has no api_key_env configured",
                provider_name
            ))
        })?;
        let api_key = std::env::var(env_var)
            .map_err(|_| AppError::Provider(format!("{} not set", env_var)))?;

        self.create_provider(provider_name, api_key, model_id)
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}
