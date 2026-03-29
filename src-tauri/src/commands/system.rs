use crate::error::AppResult;

/// 系统：→ Heartbeat + Gateway + Cron 状态
#[tauri::command]
pub async fn get_system_status() -> AppResult<String> {
    todo!("实现获取系统状态命令")
}
