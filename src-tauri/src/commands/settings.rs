use crate::error::{AppError, AppResult};
use crate::models::settings::AppSettings;
use crate::runtime::config::{ConfigLoader, VaultEntry};
use crate::runtime::AppRuntime;
use std::path::PathBuf;
use std::sync::Arc;

fn user_data_dir() -> AppResult<PathBuf> {
    use directories::BaseDirs;
    let base = BaseDirs::new()
        .ok_or_else(|| AppError::Internal("cannot determine base directories".into()))?;
    Ok(base.config_dir().join("mindclaw"))
}

/// 获取应用设置（从磁盘读取 UserConfig）
#[tauri::command]
pub async fn get_settings(_runtime: tauri::State<'_, Arc<AppRuntime>>) -> AppResult<AppSettings> {
    let dir = user_data_dir()?;
    let cfg = ConfigLoader::load_user_config(&dir)?;
    Ok(AppSettings {
        vault_path: cfg
            .active_vault_path
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default(),
        user_role: None,
        agent: crate::models::settings::AgentPreference {
            provider: cfg.active_provider_id,
            model_id: None,
            model_tier: crate::models::settings::ModelTierPreference::Auto,
            max_tokens_per_turn: cfg.agent_defaults.max_tokens.unwrap_or(8192) as u32,
            enable_memory: cfg.agent_defaults.enable_memory,
            enable_tools: true,
        },
        language: cfg.language,
    })
}

/// 设置当前 vault 路径，创建必要目录结构，保存到 UserConfig
#[tauri::command]
pub async fn set_vault(path: String) -> AppResult<()> {
    // 拒绝空路径
    if path.is_empty() {
        return Err(AppError::Internal("vault path cannot be empty".into()));
    }

    let vault_path = PathBuf::from(&path);

    // 必须是绝对路径
    if !vault_path.is_absolute() {
        return Err(AppError::Internal(
            "vault path must be an absolute path".into(),
        ));
    }

    // 拒绝危险根目录
    let dangerous_roots = [
        PathBuf::from("/"),
        PathBuf::from("/Users"),
        PathBuf::from("/home"),
        PathBuf::from("/etc"),
        PathBuf::from("/var"),
    ];
    for root in &dangerous_roots {
        if vault_path.starts_with(root) && vault_path != *root {
            // 允许 /Users/xxx/mindclaw-vault 但不允许 /Users 本身
        } else if vault_path == *root {
            return Err(AppError::Internal(format!(
                "vault path cannot be a system root: {}",
                path
            )));
        }
    }

    // 如果路径已存在，验证它是目录
    if vault_path.exists() && !vault_path.is_dir() {
        return Err(AppError::Internal(
            "vault path already exists but is not a directory".into(),
        ));
    }

    // 先保存配置（失败时不留下孤儿目录）
    let dir = user_data_dir()?;
    let mut cfg = ConfigLoader::load_user_config(&dir)?;

    // 更新 vaults 列表（去重后追加）
    let name = vault_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.clone());
    let now = chrono::Utc::now().to_rfc3339();
    if let Some(existing) = cfg.vaults.iter_mut().find(|v| v.path == vault_path) {
        existing.last_opened = Some(now);
    } else {
        cfg.vaults.push(VaultEntry {
            name,
            path: vault_path.clone(),
            last_opened: Some(now),
        });
    }

    cfg.active_vault_path = Some(vault_path.clone());
    ConfigLoader::save_user_config(&dir, &cfg)?;

    // 配置保存成功后再创建目录结构
    for subdir in &["daily", "tasks", "private", "source", ".mindclaw"] {
        std::fs::create_dir_all(vault_path.join(subdir))
            .map_err(|e| AppError::Storage(format!("create vault subdir {subdir}: {e}")))?;
    }

    Ok(())
}

/// 保存应用设置
#[tauri::command]
pub async fn save_settings(
    _runtime: tauri::State<'_, Arc<AppRuntime>>,
    settings: AppSettings,
) -> AppResult<()> {
    let dir = user_data_dir()?;
    let mut cfg = ConfigLoader::load_user_config(&dir)?;

    if !settings.vault_path.is_empty() {
        cfg.active_vault_path = Some(PathBuf::from(&settings.vault_path));
    }
    cfg.active_provider_id = settings.agent.provider;
    cfg.language = settings.language;
    cfg.agent_defaults.max_tokens = Some(settings.agent.max_tokens_per_turn as usize);
    cfg.agent_defaults.enable_memory = settings.agent.enable_memory;

    ConfigLoader::save_user_config(&dir, &cfg)
}

/// 设置 API Key（写入 Stronghold）
#[tauri::command]
pub async fn set_api_key(
    _runtime: tauri::State<'_, Arc<AppRuntime>>,
    _key: String,
) -> AppResult<()> {
    Err(AppError::NotFound(
        "stronghold integration not yet implemented".into(),
    ))
}
