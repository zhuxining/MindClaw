use crate::error::AppResult;
use crate::runtime::AppRuntime;
use std::sync::Arc;

/// 获取系统状态
#[tauri::command]
pub async fn get_system_status(_runtime: tauri::State<'_, Arc<AppRuntime>>) -> AppResult<String> {
    // TODO: 聚合 Heartbeat + Gateway + Cron 状态
    Ok("ok".to_string())
}
