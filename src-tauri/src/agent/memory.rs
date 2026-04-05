//! Memory 子系统
//!
//! 当前文件负责：
//! - `Memory` / `MemoryCategory`
//! - 关键词召回与向量召回入口
//! - 混合召回入口 `recall`
//!
//! 说明：
//! - 这是从原 `memory/` 目录收敛而来的单文件版本
//! - 目前只完成模块收敛，实际召回逻辑仍是占位实现

use crate::error::AppResult;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryCategory {
    UserFact,
    Preference,
    WorkContext,
    Relationship,
    Goal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub key: String,
    pub category: MemoryCategory,
    pub content: String,
    pub importance: f32,
    pub embedding: Option<Vec<f32>>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// 记忆召回：关键词 + 向量检索，importance 排序
pub async fn recall_by_keyword(_query: &str, _limit: usize) -> AppResult<Vec<Memory>> {
    todo!("实现关键词记忆召回（FTS5）")
}

pub async fn recall_by_vector(_embedding: &[f32], _limit: usize) -> AppResult<Vec<Memory>> {
    todo!("实现向量记忆召回（sqlite-vss，Phase 2）")
}

pub async fn recall(query: &str, limit: usize) -> AppResult<Vec<Memory>> {
    // 混合召回：关键词 + 向量，按 importance 排序
    recall_by_keyword(query, limit).await
}
