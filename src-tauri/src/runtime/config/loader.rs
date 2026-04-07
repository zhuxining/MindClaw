//! ConfigLoader：从 JSON 文件加载、保存、合并两层配置

use super::user::{UserConfig, VaultEntry};
use super::vault::VaultConfig;
use super::AppConfig;
use crate::error::{AppError, AppResult};
use std::path::{Path, PathBuf};

pub struct ConfigLoader;

impl ConfigLoader {
    /// 加载用户级配置，文件不存在则返回默认值（不写入磁盘）
    pub fn load_user_config(user_data_dir: &Path) -> AppResult<UserConfig> {
        let path = user_data_dir.join("config.json");
        if !path.exists() {
            return Ok(UserConfig::default());
        }
        let content = std::fs::read_to_string(&path)
            .map_err(|e| AppError::Storage(format!("read user config: {e}")))?;
        serde_json::from_str(&content)
            .map_err(|e| AppError::Storage(format!("parse user config: {e}")))
    }

    /// 保存用户级配置到磁盘（原子写）
    pub fn save_user_config(user_data_dir: &Path, cfg: &UserConfig) -> AppResult<()> {
        std::fs::create_dir_all(user_data_dir)
            .map_err(|e| AppError::Storage(format!("create user config dir: {e}")))?;
        let content = serde_json::to_string_pretty(cfg)
            .map_err(|e| AppError::Storage(format!("serialize user config: {e}")))?;
        atomic_write(&user_data_dir.join("config.json"), &content)
    }

    /// 加载 vault 级配置，文件不存在则返回默认值（不写入磁盘）
    pub fn load_vault_config(vault_path: &Path) -> AppResult<VaultConfig> {
        let path = vault_path.join(".mindclaw").join("config.json");
        if !path.exists() {
            return Ok(VaultConfig::default());
        }
        let content = std::fs::read_to_string(&path)
            .map_err(|e| AppError::Storage(format!("read vault config: {e}")))?;
        serde_json::from_str(&content)
            .map_err(|e| AppError::Storage(format!("parse vault config: {e}")))
    }

    /// 保存 vault 级配置到磁盘（原子写）
    pub fn save_vault_config(vault_path: &Path, cfg: &VaultConfig) -> AppResult<()> {
        let dir = vault_path.join(".mindclaw");
        std::fs::create_dir_all(&dir)
            .map_err(|e| AppError::Storage(format!("create vault config dir: {e}")))?;
        let content = serde_json::to_string_pretty(cfg)
            .map_err(|e| AppError::Storage(format!("serialize vault config: {e}")))?;
        atomic_write(&dir.join("config.json"), &content)
    }

    /// 合并两层配置，生成运行时 AppConfig。
    /// VaultConfig 中 Some 的字段覆盖 UserConfig agent_defaults。
    pub fn merge(user: &UserConfig, entry: &VaultEntry, vault: &VaultConfig) -> AppConfig {
        let user_data_dir =
            AppConfig::default_user_data_dir().unwrap_or_else(|_| PathBuf::from(".mindclaw-data"));
        let global_db_path = user_data_dir.join("mindclaw.db");
        let vault_config_dir = entry.path.join(".mindclaw");
        let vault_db_path = vault_config_dir.join("mindclaw.db");

        let a = &vault.agent;
        let d = &user.agent_defaults;

        AppConfig {
            user_data_dir,
            global_db_path,
            vault_path: entry.path.clone(),
            vault_config_dir,
            vault_db_path,
            provider_id: a
                .provider_id
                .clone()
                .unwrap_or_else(|| user.active_provider_id.clone()),
            model_id: a.model_id.clone(),
            system_prompt: a
                .system_prompt
                .clone()
                .unwrap_or_else(|| "你是一个智能助手，可以帮助用户完成各种任务。".to_string()),
            bus_capacity: user.bus_capacity,
            context_token_limit: d.context_token_limit,
            agent_temperature: a.temperature.or(d.temperature),
            agent_max_tokens: a.max_tokens.or(d.max_tokens),
            agent_max_iterations: a.max_iterations.unwrap_or(d.max_iterations),
            tool_concurrency: d.tool_concurrency,
            llm_concurrency: d.llm_concurrency,
            enable_memory: a.enable_memory,
            memory: a.memory.clone(),
            folder_mappings: vault.folder_mappings.clone(),
            task_defaults: vault.task_defaults.clone(),
            daily: vault.daily.clone(),
            index: vault.index.clone(),
            startup: user.startup.clone(),
        }
    }
}

/// 原子写：写临时文件后 rename
fn atomic_write(path: &Path, content: &str) -> AppResult<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, content)
        .map_err(|e| AppError::Storage(format!("write tmp config: {e}")))?;
    std::fs::rename(&tmp, path).map_err(|e| AppError::Storage(format!("rename config: {e}")))
}
