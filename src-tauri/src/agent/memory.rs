//! Memory 子系统
//!
//! - `Memory` / `MemoryCategory`：数据模型
//! - `MemoryStore`：CRUD 服务，以 Markdown 文件为真相源，SQLite 只存索引

use crate::error::{AppError, AppResult};
use crate::storage::markdown;
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

// ─── 数据模型 ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryCategory {
    UserFact,
    Preference,
    WorkContext,
    Relationship,
    Goal,
}

/// Memory Frontmatter（用于 serde_yaml 读写）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryFrontmatter {
    pub id: String,
    pub key: String,
    pub category: MemoryCategory,
    pub importance: f32,
    /// ISO8601
    pub created: String,
    /// ISO8601
    pub updated: String,
}

/// 完整的 Memory 条目（含正文和运行时路径）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub id: String,
    pub key: String,
    pub category: MemoryCategory,
    /// 文件正文
    pub content: String,
    pub importance: f32,
    /// ISO8601
    pub created: String,
    /// ISO8601
    pub updated: String,
    /// vault 内相对路径（运行时填充，不写入 Frontmatter）
    #[serde(skip)]
    pub file_path: PathBuf,
}

impl Memory {
    pub fn to_frontmatter(&self) -> MemoryFrontmatter {
        MemoryFrontmatter {
            id: self.id.clone(),
            key: self.key.clone(),
            category: self.category.clone(),
            importance: self.importance,
            created: self.created.clone(),
            updated: self.updated.clone(),
        }
    }
}

/// 写入/更新 Memory 的输入
#[derive(Debug)]
pub struct UpsertMemoryInput {
    pub category: MemoryCategory,
    pub content: String,
    pub importance: f32,
}

// ─── MemoryStore ───────────────────────────────────────────────────────────────

/// Memory 存储服务
///
/// 以 `{vault}/memory/` 下的 Markdown 文件为真相源，SQLite `memories_index` 为索引。
pub struct MemoryStore {
    vault_db: Arc<Mutex<Connection>>,
    memory_dir: PathBuf,
}

impl MemoryStore {
    pub fn new(vault_db: Arc<Mutex<Connection>>, memory_dir: PathBuf) -> Self {
        Self {
            vault_db,
            memory_dir,
        }
    }

    /// 按 key 精确查询
    pub async fn get(&self, key: &str) -> AppResult<Option<Memory>> {
        let file_path = {
            let conn = self.vault_db.lock().await;
            conn.query_row(
                "SELECT file_path FROM memories_index WHERE key = ?1",
                rusqlite::params![key],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|e| AppError::Storage(format!("get memory: {e}")))?
        };

        match file_path {
            None => Ok(None),
            Some(rel_path) => {
                let abs = self.vault_root().join(&rel_path);
                let (fm, content): (MemoryFrontmatter, String) = markdown::read_file(&abs).await?;
                Ok(Some(memory_from(fm, content, rel_path)))
            }
        }
    }

    /// 按 category 列出（按 importance 降序）
    pub async fn list_by_category(
        &self,
        category: &MemoryCategory,
        limit: usize,
    ) -> AppResult<Vec<Memory>> {
        let cat = cat_str(category);
        let rows = self
            .query_index(
                "SELECT key, file_path FROM memories_index
             WHERE category = ?1 ORDER BY importance DESC LIMIT ?2",
                rusqlite::params![cat, limit as i64],
            )
            .await?;
        self.load_memories(rows).await
    }

    /// 取 top-N 重要记忆（跨 category）
    pub async fn top_important(&self, limit: usize) -> AppResult<Vec<Memory>> {
        let rows = self
            .query_index(
                "SELECT key, file_path FROM memories_index
             ORDER BY importance DESC LIMIT ?1",
                rusqlite::params![limit as i64],
            )
            .await?;
        self.load_memories(rows).await
    }

    /// 关键词召回：在 `memories_index.key` 上做 LIKE 查询
    pub async fn recall(&self, query: &str, limit: usize) -> AppResult<Vec<Memory>> {
        let pattern = format!("%{query}%");
        let rows = self
            .query_index(
                "SELECT key, file_path FROM memories_index
             WHERE key LIKE ?1 ORDER BY importance DESC LIMIT ?2",
                rusqlite::params![pattern, limit as i64],
            )
            .await?;
        self.load_memories(rows).await
    }

    /// 写入或更新 Memory（key 已存在则覆盖）
    pub async fn upsert(&self, key: &str, input: UpsertMemoryInput) -> AppResult<Memory> {
        let now = chrono::Utc::now().to_rfc3339();
        let existing = self.get(key).await?;

        let (id, created, rel_path) = if let Some(ref m) = existing {
            (m.id.clone(), m.created.clone(), m.file_path.clone())
        } else {
            let id = uuid::Uuid::new_v4().to_string();
            let abs = markdown::memory_file_path(&self.memory_dir, &input.category, key);
            let rel = self.to_rel(&abs);
            (id, now.clone(), rel)
        };

        let fm = MemoryFrontmatter {
            id: id.clone(),
            key: key.to_string(),
            category: input.category.clone(),
            importance: input.importance,
            created: created.clone(),
            updated: now.clone(),
        };

        let abs = self.vault_root().join(&rel_path);
        markdown::write_file(&abs, &fm, &input.content).await?;
        self.upsert_index_row(&fm, &path_str(&rel_path)).await?;

        Ok(Memory {
            id,
            key: key.to_string(),
            category: input.category,
            content: input.content,
            importance: input.importance,
            created,
            updated: now,
            file_path: rel_path,
        })
    }

    /// 删除 Memory（删除文件和索引行）
    pub async fn delete(&self, key: &str) -> AppResult<()> {
        if let Some(m) = self.get(key).await? {
            let abs = self.vault_root().join(&m.file_path);
            if abs.exists() {
                tokio::fs::remove_file(&abs)
                    .await
                    .map_err(|e| AppError::Storage(format!("delete memory file: {e}")))?;
            }
        }
        let conn = self.vault_db.lock().await;
        conn.execute(
            "DELETE FROM memories_index WHERE key = ?1",
            rusqlite::params![key],
        )
        .map_err(|e| AppError::Storage(format!("delete memory index: {e}")))?;
        Ok(())
    }

    /// 重建 memories_index（扫描 memory_dir 下所有 .md 文件）
    pub async fn rebuild_index(&self) -> AppResult<usize> {
        if !self.memory_dir.exists() {
            return Ok(0);
        }
        let mut count = 0usize;
        let mut entries = tokio::fs::read_dir(&self.memory_dir)
            .await
            .map_err(|e| AppError::Storage(format!("read memory dir: {e}")))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| AppError::Storage(format!("iterate memory dir: {e}")))?
        {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("md") {
                continue;
            }
            if let Ok((fm, _)) = markdown::read_file::<MemoryFrontmatter>(&path).await {
                let rel = self.to_rel(&path);
                self.upsert_index_row(&fm, &path_str(&rel)).await?;
                count += 1;
            }
        }
        Ok(count)
    }

    // ── 内部辅助 ──────────────────────────────────────────────────────────────

    async fn query_index(
        &self,
        sql: &str,
        params: impl rusqlite::Params,
    ) -> AppResult<Vec<(String, String)>> {
        let conn = self.vault_db.lock().await;
        let mut stmt = conn
            .prepare(sql)
            .map_err(|e| AppError::Storage(format!("prepare: {e}")))?;
        let mapped = match stmt.query_map(params, |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        }) {
            Ok(m) => m,
            Err(e) => return Err(AppError::Storage(format!("query_map: {e}"))),
        };
        mapped
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| AppError::Storage(format!("collect: {e}")))
    }

    async fn upsert_index_row(&self, fm: &MemoryFrontmatter, file_path: &str) -> AppResult<()> {
        let cat = cat_str(&fm.category);
        let conn = self.vault_db.lock().await;
        conn.execute(
            "INSERT INTO memories_index (id, key, category, importance, file_path, updated)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(key) DO UPDATE SET
               category   = excluded.category,
               importance = excluded.importance,
               file_path  = excluded.file_path,
               updated    = excluded.updated",
            rusqlite::params![fm.id, fm.key, cat, fm.importance, file_path, fm.updated],
        )
        .map_err(|e| AppError::Storage(format!("upsert memory index: {e}")))?;
        Ok(())
    }

    async fn load_memories(&self, rows: Vec<(String, String)>) -> AppResult<Vec<Memory>> {
        let mut result = Vec::with_capacity(rows.len());
        for (_key, rel_path) in rows {
            let abs = self.vault_root().join(&rel_path);
            if let Ok((fm, content)) = markdown::read_file::<MemoryFrontmatter>(&abs).await {
                result.push(memory_from(fm, content, rel_path));
            }
        }
        Ok(result)
    }

    fn vault_root(&self) -> &Path {
        self.memory_dir.parent().unwrap_or(&self.memory_dir)
    }

    fn to_rel(&self, abs: &Path) -> PathBuf {
        abs.strip_prefix(self.vault_root())
            .unwrap_or(abs)
            .to_path_buf()
    }
}

// ─── 辅助函数 ──────────────────────────────────────────────────────────────────

fn cat_str(c: &MemoryCategory) -> &'static str {
    match c {
        MemoryCategory::UserFact => "user_fact",
        MemoryCategory::Preference => "preference",
        MemoryCategory::WorkContext => "work_context",
        MemoryCategory::Relationship => "relationship",
        MemoryCategory::Goal => "goal",
    }
}

fn path_str(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

fn memory_from(fm: MemoryFrontmatter, content: String, rel_path: impl Into<PathBuf>) -> Memory {
    Memory {
        id: fm.id,
        key: fm.key,
        category: fm.category,
        content,
        importance: fm.importance,
        created: fm.created,
        updated: fm.updated,
        file_path: rel_path.into(),
    }
}
