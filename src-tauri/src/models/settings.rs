use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserRole {
    Professional,
    Student,
    Researcher,
    Creator,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelTierPreference {
    /// 优先省钱（Haiku）
    Economy,
    /// 优先质量（Sonnet）
    Quality,
    /// 自动按任务选择
    Auto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPreference {
    /// 提供商名称（如 "openai", "deepseek", "claude"）
    pub provider: String,
    /// 具体模型 ID，None 则使用提供商的默认模型
    pub model_id: Option<String>,
    pub model_tier: ModelTierPreference,
    pub max_tokens_per_turn: u32,
    pub enable_memory: bool,
    pub enable_tools: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub vault_path: String,
    pub user_role: Option<UserRole>,
    pub agent: AgentPreference,
    pub language: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            vault_path: "~/MindClaw/vault".to_string(),
            user_role: None,
            agent: AgentPreference {
                provider: "openai".to_string(),
                model_id: None,
                model_tier: ModelTierPreference::Auto,
                max_tokens_per_turn: 8192,
                enable_memory: true,
                enable_tools: true,
            },
            language: "zh-CN".to_string(),
        }
    }
}
