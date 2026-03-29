use crate::error::AppResult;

/// 日记：→ DailyService
#[tauri::command]
pub async fn get_daily(_date: String) -> AppResult<String> {
    todo!("实现获取日记命令")
}

#[tauri::command]
pub async fn save_daily(_date: String, _content: String) -> AppResult<()> {
    todo!("实现保存日记命令")
}
