use crate::error::{AppError, AppResult};
use crate::models::note::KnowledgeEntry;
use crate::runtime::AppRuntime;
use crate::services::note::NoteIndex;
use std::sync::Arc;

fn note_to_knowledge(n: NoteIndex) -> KnowledgeEntry {
    KnowledgeEntry {
        id: n.id,
        title: n.title,
        topic: n.file_path.clone(),
        content: String::new(), // 搜索结果不含正文，避免大量 IO
        wikilinks: Vec::new(),
        tags: n.tags,
        source_url: None,
        created_at: 0,
        updated_at: 0,
    }
}

/// 搜索知识库（按标题/标签关键词）
#[tauri::command]
pub async fn search_knowledge(
    runtime: tauri::State<'_, Arc<AppRuntime>>,
    query: String,
) -> AppResult<Vec<KnowledgeEntry>> {
    let results = runtime.services().note.search(&query, 20).await?;
    Ok(results.into_iter().map(note_to_knowledge).collect())
}

/// 获取知识条目（含正文）
#[tauri::command]
pub async fn get_knowledge(
    runtime: tauri::State<'_, Arc<AppRuntime>>,
    id: String,
) -> AppResult<KnowledgeEntry> {
    // 先按 id 从索引查 file_path
    let all = runtime.services().note.search("", 10000).await?;
    let entry = all
        .into_iter()
        .find(|n| n.id == id)
        .ok_or_else(|| AppError::NotFound(format!("knowledge entry not found: {id}")))?;

    let content = runtime.services().note.read(&entry.file_path).await?;
    Ok(KnowledgeEntry {
        id: entry.id,
        title: entry.title,
        topic: entry.file_path,
        content,
        wikilinks: Vec::new(),
        tags: entry.tags,
        source_url: None,
        created_at: 0,
        updated_at: 0,
    })
}
