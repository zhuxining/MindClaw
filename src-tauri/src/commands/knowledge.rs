use crate::error::{AppError, AppResult};
use crate::models::note::KnowledgeEntry;
use crate::runtime::AppRuntime;
use std::sync::Arc;

/// 搜索知识库
#[tauri::command]
pub async fn search_knowledge(
    _runtime: tauri::State<'_, Arc<AppRuntime>>,
    _query: String,
) -> AppResult<Vec<KnowledgeEntry>> {
    // TODO: 委托给 runtime.services().knowledge
    Ok(Vec::new())
}

/// 获取知识条目
#[tauri::command]
pub async fn get_knowledge(
    _runtime: tauri::State<'_, Arc<AppRuntime>>,
    _id: String,
) -> AppResult<KnowledgeEntry> {
    // TODO: 委托给 runtime.services().knowledge
    Err(AppError::NotFound(
        "knowledge service not yet implemented".into(),
    ))
}
