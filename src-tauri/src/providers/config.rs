//! Provider 配置：数据驱动的提供商和模型定义

use super::traits::ModelTier;
use serde::{Deserialize, Serialize};

/// 单个模型的配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// 发送给 API 的模型 ID（如 "gpt-4o", "deepseek-chat"）
    pub id: String,
    /// UI 展示名称
    pub display_name: String,
    /// 模型对应的能力层级
    pub tier: ModelTier,
    /// 最大输出 token 数
    pub max_output_tokens: u32,
    /// 上下文窗口大小（tokens）
    pub context_window: u32,
}

/// OpenAI 兼容提供商的配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// 唯一标识（如 "openai", "deepseek", "claude"）
    pub name: String,
    /// API 基础 URL
    pub api_base: String,
    /// API Key 对应的环境变量名
    pub api_key_env: Option<String>,
    /// 该提供商可用的模型列表
    pub models: Vec<ModelConfig>,
    /// 未指定模型时的默认模型 ID
    pub default_model: String,
}

/// 上下文窗口裁剪阈值（tokens）
pub const CONTEXT_TRIM_THRESHOLD: u32 = 180_000;

/// 内置提供商配置
pub fn builtin_configs() -> Vec<ProviderConfig> {
    vec![
        ProviderConfig {
            name: "openai".into(),
            api_base: "https://api.openai.com/v1".into(),
            api_key_env: Some("OPENAI_API_KEY".into()),
            models: vec![
                ModelConfig {
                    id: "gpt-4o-mini".into(),
                    display_name: "GPT-4o Mini".into(),
                    tier: ModelTier::Fast,
                    max_output_tokens: 16_384,
                    context_window: 128_000,
                },
                ModelConfig {
                    id: "gpt-4o".into(),
                    display_name: "GPT-4o".into(),
                    tier: ModelTier::Smart,
                    max_output_tokens: 16_384,
                    context_window: 128_000,
                },
            ],
            default_model: "gpt-4o".into(),
        },
        ProviderConfig {
            name: "deepseek".into(),
            api_base: "https://api.deepseek.com".into(),
            api_key_env: Some("DEEPSEEK_API_KEY".into()),
            models: vec![
                ModelConfig {
                    id: "deepseek-chat".into(),
                    display_name: "DeepSeek V3".into(),
                    tier: ModelTier::Fast,
                    max_output_tokens: 8_192,
                    context_window: 64_000,
                },
                ModelConfig {
                    id: "deepseek-reasoner".into(),
                    display_name: "DeepSeek R1".into(),
                    tier: ModelTier::Smart,
                    max_output_tokens: 8_192,
                    context_window: 64_000,
                },
            ],
            default_model: "deepseek-chat".into(),
        },
        ProviderConfig {
            name: "claude".into(),
            api_base: "https://api.anthropic.com".into(),
            api_key_env: Some("ANTHROPIC_API_KEY".into()),
            models: vec![
                ModelConfig {
                    id: "claude-haiku-4-5-20251001".into(),
                    display_name: "Claude Haiku 4.5".into(),
                    tier: ModelTier::Fast,
                    max_output_tokens: 8_192,
                    context_window: 200_000,
                },
                ModelConfig {
                    id: "claude-sonnet-4-6".into(),
                    display_name: "Claude Sonnet 4.6".into(),
                    tier: ModelTier::Smart,
                    max_output_tokens: 64_000,
                    context_window: 200_000,
                },
            ],
            default_model: "claude-sonnet-4-6".into(),
        },
    ]
}
