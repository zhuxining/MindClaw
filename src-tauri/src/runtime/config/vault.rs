//! Vault 级配置（{vault}/.mindclaw/config.json）
//!
//! 与特定 vault 绑定，跟着 vault 走（可纳入 Obsidian git sync）。
//! 所有字段均有 Default，缺失字段不报错。

use crate::models::task::TaskPriority;
use serde::{Deserialize, Serialize};

/// Vault 级配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct VaultConfig {
    /// Agent 行为覆盖（None = 继承 UserConfig.agent_defaults）
    pub agent: VaultAgentConfig,
    /// Vault 内文件夹映射
    pub folder_mappings: FolderMappings,
    /// 任务创建默认值
    pub task_defaults: TaskDefaults,
    /// 日记设置
    pub daily: DailyConfig,
    /// 索引设置
    pub index: IndexConfig,
}

/// Vault 级 Agent 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct VaultAgentConfig {
    /// None = 继承全局 provider
    pub provider_id: Option<String>,
    pub model_id: Option<String>,
    /// None = 使用内置默认提示词
    pub system_prompt: Option<String>,
    pub max_iterations: Option<usize>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<usize>,
    /// 是否为此 vault 开启记忆提取
    pub enable_memory: bool,
    /// 记忆提取子配置
    pub memory: MemoryConfig,
}

impl Default for VaultAgentConfig {
    fn default() -> Self {
        Self {
            provider_id: None,
            model_id: None,
            system_prompt: None,
            max_iterations: None,
            temperature: None,
            max_tokens: None,
            enable_memory: true,
            memory: MemoryConfig::default(),
        }
    }
}

/// 记忆提取设置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MemoryConfig {
    /// turn 结束后自动后台提取
    pub auto_extract: bool,
    /// 低于此重要性的记忆不写入
    pub min_importance: f32,
    /// 超出时按 importance 淘汰旧条目
    pub max_entries: usize,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            auto_extract: true,
            min_importance: 0.5,
            max_entries: 500,
        }
    }
}

/// Vault 内文件夹路径映射（相对于 vault 根目录）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FolderMappings {
    pub tasks: String,
    pub memory: String,
    pub daily: String,
    /// notes_index 扫描时排除的目录
    pub index_exclude: Vec<String>,
}

impl Default for FolderMappings {
    fn default() -> Self {
        Self {
            tasks: "tasks".to_string(),
            memory: "memory".to_string(),
            daily: "daily".to_string(),
            index_exclude: vec![
                ".obsidian".to_string(),
                ".mindclaw".to_string(),
                "templates".to_string(),
                "attachments".to_string(),
            ],
        }
    }
}

/// 任务创建默认值
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TaskDefaults {
    pub priority: TaskPriority,
    pub tags: Vec<String>,
}

impl Default for TaskDefaults {
    fn default() -> Self {
        Self {
            priority: TaskPriority::Medium,
            tags: Vec::new(),
        }
    }
}

/// 日记设置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DailyConfig {
    /// 日记文件名日期格式（目前固定为 YYYY-MM-DD）
    pub date_format: String,
    /// 日记模板内容（None = 无模板）
    pub template: Option<String>,
}

impl Default for DailyConfig {
    fn default() -> Self {
        Self {
            date_format: "YYYY-MM-DD".to_string(),
            template: None,
        }
    }
}

/// 笔记索引设置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct IndexConfig {
    /// 后台增量 sync 间隔（秒），0 = 禁用
    pub auto_sync_interval_secs: u64,
}

impl Default for IndexConfig {
    fn default() -> Self {
        Self {
            auto_sync_interval_secs: 300,
        }
    }
}
