//! 应用配置：双层配置体系（用户级 + vault 级）
//!
//! - `UserConfig`：跨 vault 的全局偏好，存于 `~/.config/mindclaw/config.json`
//! - `VaultConfig`：与 vault 绑定的配置，存于 `{vault}/.mindclaw/config.json`
//! - `AppConfig`：两层合并后的运行时配置（内存中使用）

pub mod loader;
pub mod user;
pub mod vault;

pub use loader::ConfigLoader;
pub use user::{GlobalAgentDefaults, ProviderConfig, StartupConfig, UserConfig, VaultEntry};
pub use vault::{
    DailyConfig, FolderMappings, IndexConfig, MemoryConfig, TaskDefaults, VaultAgentConfig,
    VaultConfig,
};

use crate::error::{AppError, AppResult};
use std::path::PathBuf;

/// 合并后的运行时配置（内存中使用，不直接写磁盘）
#[derive(Debug, Clone)]
pub struct AppConfig {
    // ── 路径 ──────────────────────────────────────────────────────────────
    /// 用户数据根目录（~/.config/mindclaw/）
    pub user_data_dir: PathBuf,
    /// 全局 SQLite DB 路径（sessions/turns，跨 vault）
    pub global_db_path: PathBuf,
    /// 当前 Obsidian vault 根目录
    pub vault_path: PathBuf,
    /// vault 配置目录（{vault}/.mindclaw/）
    pub vault_config_dir: PathBuf,
    /// vault 级 SQLite DB 路径（tasks/notes/memories 索引）
    pub vault_db_path: PathBuf,

    // ── LLM / Agent 运行时参数 ──────────────────────────────────────────
    pub provider_id: String,
    /// 主模型 ID；为空时使用当前 provider 的主模型 / 默认模型。
    pub model_id: Option<String>,
    /// 轻量模型 ID；为空时运行时回退到主模型。
    pub light_model_id: Option<String>,
    pub system_prompt: String,
    pub bus_capacity: usize,
    pub context_token_limit: usize,
    pub agent_temperature: Option<f32>,
    pub agent_max_tokens: Option<usize>,
    pub agent_max_iterations: usize,
    pub tool_concurrency: usize,
    pub llm_concurrency: usize,

    // ── 功能开关与子配置 ────────────────────────────────────────────────
    pub enable_memory: bool,
    pub memory: MemoryConfig,
    pub folder_mappings: FolderMappings,
    pub task_defaults: TaskDefaults,
    pub daily: DailyConfig,
    pub index: IndexConfig,
    pub startup: StartupConfig,
}

impl AppConfig {
    /// 获取平台标准用户数据目录（~/.config/mindclaw）
    pub fn default_user_data_dir() -> AppResult<PathBuf> {
        use directories::BaseDirs;
        let base = BaseDirs::new()
            .ok_or_else(|| AppError::Internal("cannot determine base directories".into()))?;
        Ok(base.config_dir().join("mindclaw"))
    }

    /// 向后兼容：返回 user_data_dir（原 data_dir 字段）
    pub fn data_dir(&self) -> &PathBuf {
        &self.user_data_dir
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        let user_data_dir =
            Self::default_user_data_dir().unwrap_or_else(|_| PathBuf::from(".mindclaw-data"));
        let vault_path = user_data_dir.join("vault");
        let vault_config_dir = vault_path.join(".mindclaw");
        Self {
            global_db_path: user_data_dir.join("mindclaw.db"),
            vault_db_path: vault_config_dir.join("mindclaw.db"),
            vault_config_dir,
            vault_path,
            user_data_dir,
            provider_id: "deepseek".to_string(),
            model_id: None,
            light_model_id: None,
            bus_capacity: 100,
            context_token_limit: 128_000,
            system_prompt: "你是一个智能助手，可以帮助用户完成各种任务。".to_string(),
            agent_temperature: None,
            agent_max_tokens: None,
            agent_max_iterations: 8,
            tool_concurrency: 4,
            llm_concurrency: 3,
            enable_memory: true,
            memory: MemoryConfig::default(),
            folder_mappings: FolderMappings::default(),
            task_defaults: TaskDefaults::default(),
            daily: DailyConfig::default(),
            index: IndexConfig::default(),
            startup: StartupConfig::default(),
        }
    }
}
