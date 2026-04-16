//! 用户级配置（~/.config/mindclaw/config.json）
//!
//! 跨 vault 的全局偏好，随用户账号存在。

use crate::models::settings::WorkspacePrefs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 用户级全局配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UserConfig {
    /// 界面语言："zh-CN" | "en-US"
    pub language: String,
    /// 主题："light" | "dark" | "system"
    pub theme: String,
    /// 当前激活的 vault 路径
    pub active_vault_path: Option<PathBuf>,
    /// 已注册的 vault 列表（最近打开过的 vault 记录）
    pub vaults: Vec<VaultEntry>,
    /// 当前激活的 provider id
    pub active_provider_id: String,
    /// LLM provider 配置列表
    pub providers: Vec<ProviderConfig>,
    /// 全局 Agent 默认参数（可被 VaultConfig 覆盖）
    pub agent_defaults: GlobalAgentDefaults,
    /// MessageBus 通道容量
    pub bus_capacity: usize,
    /// 启动行为配置
    pub startup: StartupConfig,
    /// 桌面工作区偏好
    pub workspace: WorkspacePrefs,
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            language: "zh-CN".to_string(),
            theme: "system".to_string(),
            active_vault_path: None,
            vaults: Vec::new(),
            active_provider_id: "deepseek".to_string(),
            providers: vec![
                ProviderConfig {
                    id: "deepseek".to_string(),
                    display_name: "DeepSeek".to_string(),
                    base_url: "https://api.deepseek.com".to_string(),
                    default_model: Some("deepseek-chat".to_string()),
                    available_models: vec![
                        "deepseek-chat".to_string(),
                        "deepseek-reasoner".to_string(),
                    ],
                },
                ProviderConfig {
                    id: "openai".to_string(),
                    display_name: "OpenAI".to_string(),
                    base_url: "https://api.openai.com".to_string(),
                    default_model: Some("gpt-4o".to_string()),
                    available_models: vec![
                        "gpt-4o".to_string(),
                        "gpt-4o-mini".to_string(),
                        "o3-mini".to_string(),
                    ],
                },
                ProviderConfig {
                    id: "claude".to_string(),
                    display_name: "Claude".to_string(),
                    base_url: "https://api.anthropic.com".to_string(),
                    default_model: Some("claude-sonnet-4-6".to_string()),
                    available_models: vec![
                        "claude-opus-4-6".to_string(),
                        "claude-sonnet-4-6".to_string(),
                        "claude-haiku-4-5-20251001".to_string(),
                    ],
                },
            ],
            agent_defaults: GlobalAgentDefaults::default(),
            bus_capacity: 100,
            startup: StartupConfig::default(),
            workspace: WorkspacePrefs::default(),
        }
    }
}

/// 已注册的 vault 条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultEntry {
    /// 显示名称
    pub name: String,
    /// vault 根目录绝对路径（作为唯一标识）
    pub path: PathBuf,
    /// 上次打开时间（ISO8601）
    pub last_opened: Option<String>,
}

/// LLM Provider 配置（API key 不存这里，存 Stronghold）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// provider 标识："deepseek" | "openai" | "claude"
    pub id: String,
    /// 显示名称
    pub display_name: String,
    /// API base URL
    pub base_url: String,
    /// 默认模型 id
    pub default_model: Option<String>,
    /// 可选的模型列表（用于 UI 下拉）
    pub available_models: Vec<String>,
}

/// 全局 Agent 默认参数
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GlobalAgentDefaults {
    pub max_iterations: usize,
    pub temperature: Option<f32>,
    pub max_tokens: Option<usize>,
    pub context_token_limit: usize,
    pub tool_concurrency: usize,
    pub llm_concurrency: usize,
    pub enable_memory: bool,
}

impl Default for GlobalAgentDefaults {
    fn default() -> Self {
        Self {
            max_iterations: 8,
            temperature: None,
            max_tokens: None,
            context_token_limit: 128_000,
            tool_concurrency: 4,
            llm_concurrency: 3,
            enable_memory: true,
        }
    }
}

/// 启动行为配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct StartupConfig {
    /// 启动时自动打开上次的 vault
    pub open_last_vault: bool,
    /// 打开 vault 时自动增量 sync 索引
    pub sync_index_on_open: bool,
}

impl Default for StartupConfig {
    fn default() -> Self {
        Self {
            open_last_vault: true,
            sync_index_on_open: true,
        }
    }
}
