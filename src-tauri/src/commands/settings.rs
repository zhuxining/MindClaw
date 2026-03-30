use crate::error::{AppError, AppResult};
use crate::models::settings::AppSettings;
use crate::runtime::AppRuntime;
use std::sync::Arc;

/// 获取应用设置
#[tauri::command]
pub async fn get_settings(_runtime: tauri::State<'_, Arc<AppRuntime>>) -> AppResult<AppSettings> {
    // TODO: 从 Storage 读取 settings.json
    Ok(AppSettings::default())
}

/// 保存应用设置
#[tauri::command]
pub async fn save_settings(
    _runtime: tauri::State<'_, Arc<AppRuntime>>,
    _settings: AppSettings,
) -> AppResult<()> {
    // TODO: 写入 Storage
    Err(AppError::NotFound(
        "settings persistence not yet implemented".into(),
    ))
}

/// 设置 API Key（写入 Stronghold）
#[tauri::command]
pub async fn set_api_key(
    _runtime: tauri::State<'_, Arc<AppRuntime>>,
    _key: String,
) -> AppResult<()> {
    // TODO: 写入 Stronghold
    Err(AppError::NotFound(
        "stronghold integration not yet implemented".into(),
    ))
}
