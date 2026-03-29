use super::types::Memory;
use crate::error::AppResult;

/// 记忆召回：关键词 + 向量检索，importance 排序
pub async fn recall_by_keyword(_query: &str, _limit: usize) -> AppResult<Vec<Memory>> {
    todo!("实现关键词记忆召回（FTS5）")
}

pub async fn recall_by_vector(_embedding: &[f32], _limit: usize) -> AppResult<Vec<Memory>> {
    todo!("实现向量记忆召回（sqlite-vss，Phase 2）")
}

pub async fn recall(_query: &str, _limit: usize) -> AppResult<Vec<Memory>> {
    // 混合召回：关键词 + 向量，按 importance 排序
    recall_by_keyword(_query, _limit).await
}
