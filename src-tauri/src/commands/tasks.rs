use crate::error::{AppError, AppResult};
use crate::models::task::Task;
use crate::runtime::AppRuntime;
use std::sync::Arc;

/// 列出所有任务
#[tauri::command]
pub async fn list_tasks(_runtime: tauri::State<'_, Arc<AppRuntime>>) -> AppResult<Vec<Task>> {
    // TODO: 委托给 runtime.services().task
    Ok(Vec::new())
}

/// 创建任务
#[tauri::command]
pub async fn create_task(
    _runtime: tauri::State<'_, Arc<AppRuntime>>,
    _title: String,
    _description: Option<String>,
) -> AppResult<Task> {
    // TODO: 委托给 runtime.services().task
    Err(AppError::NotFound(
        "task service not yet implemented".into(),
    ))
}

/// 更新任务状态
#[tauri::command]
pub async fn update_task_status(
    _runtime: tauri::State<'_, Arc<AppRuntime>>,
    _id: String,
    _status: String,
) -> AppResult<()> {
    // TODO: 委托给 runtime.services().task
    Err(AppError::NotFound(
        "task service not yet implemented".into(),
    ))
}
