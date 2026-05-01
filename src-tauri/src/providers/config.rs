//! Provider 配置：简化的提供商定义
//!
//! rig 框架内置了 20+ 个供应商，每个供应商内部已硬编码 base URL。
//! 此模块只需定义环境变量名和默认模型。

/// 简化的提供商配置
///
/// rig 框架的每个供应商（anthropic, openai, deepseek 等）都有独立的 Client::new(&api_key)
/// 构造函数，内部已硬编码了 API base URL，无需外部配置。
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    /// provider 标识符（"anthropic", "openai", "deepseek" 等）
    pub name: String,
    /// API key 环境变量名
    pub api_key_env: String,
    /// 默认模型 ID
    pub default_model: String,
}

/// 内置提供商配置
///
/// 添加新供应商只需在此处添加一行，然后在使用 `define_providers!` 宏的地方
/// 添加对应的定义即可。
pub fn builtin_configs() -> Vec<ProviderConfig> {
    vec![
        ProviderConfig {
            name: "anthropic".into(),
            api_key_env: "ANTHROPIC_API_KEY".into(),
            default_model: "claude-sonnet-4-6".into(),
        },
        ProviderConfig {
            name: "openai".into(),
            api_key_env: "OPENAI_API_KEY".into(),
            default_model: "gpt-4o".into(),
        },
        ProviderConfig {
            name: "deepseek".into(),
            api_key_env: "DEEPSEEK_API_KEY".into(),
            default_model: "deepseek-chat".into(),
        },
    ]
}
