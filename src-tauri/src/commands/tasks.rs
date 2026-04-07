use crate::error::AppResult;
use crate::models::task::{Task, TaskStatus};
use crate::runtime::AppRuntime;
use crate::services::task::{CreateTaskInput, TaskFilter, UpdateTaskInput};
use std::sync::Arc;

/// 列出任务（可选状态过滤）
#[tauri::command]
pub async fn list_tasks(
    runtime: tauri::State<'_, Arc<AppRuntime>>,
    status: Option<String>,
) -> AppResult<Vec<Task>> {
    let filter = TaskFilter {
        status: status.as_deref().map(parse_status),
        ..Default::default()
    };
    runtime.services().task.list(filter).await
}

/// 创建任务
#[tauri::command]
pub async fn create_task(
    runtime: tauri::State<'_, Arc<AppRuntime>>,
    title: String,
    body: Option<String>,
    priority: Option<String>,
    due_date: Option<String>,
    tags: Option<Vec<String>>,
) -> AppResult<Task> {
    let input = CreateTaskInput {
        title,
        body,
        status: None,
        priority: priority
            .as_deref()
            .map(crate::services::task::parse_priority_pub),
        due_date,
        tags: tags.unwrap_or_default(),
    };
    runtime.services().task.create(input).await
}

/// 更新任务状态
#[tauri::command]
pub async fn update_task_status(
    runtime: tauri::State<'_, Arc<AppRuntime>>,
    id: String,
    status: String,
) -> AppResult<()> {
    let input = UpdateTaskInput {
        status: Some(parse_status(&status)),
        ..Default::default()
    };
    runtime.services().task.update(&id, input).await?;
    Ok(())
}

/// 重建任务索引（vault 被外部编辑后调用）
#[tauri::command]
pub async fn rebuild_tasks_index(runtime: tauri::State<'_, Arc<AppRuntime>>) -> AppResult<usize> {
    runtime.services().task.rebuild_index().await
}

fn parse_status(s: &str) -> TaskStatus {
    match s {
        "in_progress" => TaskStatus::InProgress,
        "done" => TaskStatus::Done,
        "cancelled" => TaskStatus::Cancelled,
        _ => TaskStatus::Todo,
    }
}
