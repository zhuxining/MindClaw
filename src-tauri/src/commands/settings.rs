use crate::error::AppResult;
use crate::models::settings::AppSettings;

/// 设置：→ Storage (settings.json + keychain)
#[tauri::command]
pub async fn get_settings() -> AppResult<AppSettings> {
    todo!("实现获取设置命令")
}

#[tauri::command]
pub async fn save_settings(_settings: AppSettings) -> AppResult<()> {
    todo!("实现保存设置命令")
}

#[tauri::command]
pub async fn set_api_key(_key: String) -> AppResult<()> {
    todo!("实现设置 API Key 命令（写入 Stronghold）")
}
