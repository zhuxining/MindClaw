use crate::error::AppResult;
use crate::models::note::KnowledgeEntry;

/// 知识库：→ KnowledgeService
#[tauri::command]
pub async fn search_knowledge(_query: String) -> AppResult<Vec<KnowledgeEntry>> {
    todo!("实现知识搜索命令")
}

#[tauri::command]
pub async fn get_knowledge(_id: String) -> AppResult<KnowledgeEntry> {
    todo!("实现获取知识条目命令")
}
