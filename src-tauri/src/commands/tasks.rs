use crate::error::AppResult;
use crate::models::task::Task;

/// 任务：→ TaskService
#[tauri::command]
pub async fn list_tasks() -> AppResult<Vec<Task>> {
    todo!("实现列出任务命令")
}

#[tauri::command]
pub async fn create_task(_title: String, _description: Option<String>) -> AppResult<Task> {
    todo!("实现创建任务命令")
}

#[tauri::command]
pub async fn update_task_status(_id: String, _status: String) -> AppResult<()> {
    todo!("实现更新任务状态命令")
}
