//! NoteService：Obsidian vault 笔记索引服务
//!
//! 真相源是 vault 内的 Markdown 文件，SQLite `notes_index` 是可重建的索引。
//! NoteService 不管理 tasks/ 和 memory/ 目录（由 TaskService/MemoryStore 负责）。

use crate::error::{AppError, AppResult};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

/// notes_index 行（轻量，不读文件内容）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteIndex {
    pub id: String,
    pub title: String,
    pub tags: Vec<String>,
    pub file_path: String,
    pub modified_at: String,
}

/// 增量同步结果统计
#[derive(Debug, Default)]
pub struct SyncResult {
    pub added: usize,
    pub updated: usize,
    pub deleted: usize,
}

pub struct NoteService {
    vault_db: Arc<Mutex<Connection>>,
    vault_path: PathBuf,
}

impl NoteService {
    pub fn new(vault_db: Arc<Mutex<Connection>>, vault_path: PathBuf) -> Self {
        Self {
            vault_db,
            vault_path,
        }
    }

    /// 按标题/tags 关键词搜索（查索引，不读文件）
    pub async fn search(&self, query: &str, limit: usize) -> AppResult<Vec<NoteIndex>> {
        let pattern = format!("%{query}%");
        let rows = {
            let conn = self.vault_db.lock().await;
            let mut stmt = conn
                .prepare(
                    "SELECT id, title, tags, file_path, modified_at FROM notes_index
                     WHERE title LIKE ?1 OR tags LIKE ?1
                     ORDER BY modified_at DESC LIMIT ?2",
                )
                .map_err(|e| AppError::Storage(format!("search notes: {e}")))?;
            let mapped = match stmt.query_map(rusqlite::params![pattern, limit as i64], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            }) {
                Ok(m) => m,
                Err(e) => return Err(AppError::Storage(format!("query notes: {e}"))),
            };
            let rows: Vec<_> = mapped
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|e| AppError::Storage(format!("collect notes: {e}")))?;
            rows
        };

        rows.into_iter()
            .map(|(id, title, tags_json, file_path, modified_at)| {
                let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
                Ok(NoteIndex {
                    id,
                    title,
                    tags,
                    file_path,
                    modified_at,
                })
            })
            .collect()
    }

    /// 按 vault 内相对路径读取笔记完整内容
    pub async fn read(&self, file_path: &str) -> AppResult<String> {
        let abs = self.vault_path.join(file_path);
        tokio::fs::read_to_string(&abs)
            .await
            .map_err(|e| AppError::Storage(format!("read note {file_path}: {e}")))
    }

    /// 增量 sync：对比文件 mtime，只更新有变化的文件
    pub async fn sync_index(&self, exclude_dirs: &[&str]) -> AppResult<SyncResult> {
        let mut result = SyncResult::default();
        let files = collect_md_files(&self.vault_path, exclude_dirs).await?;

        // 获取已有索引（file_path → modified_at）
        let existing = self.load_index_map().await?;

        for (rel_path, mtime) in &files {
            let key = rel_path.to_string_lossy().replace('\\', "/");
            match existing.get(&key) {
                Some(old_mtime) if old_mtime == mtime => {
                    // 未变化，跳过
                }
                Some(_) => {
                    // 已更改
                    let abs = self.vault_path.join(rel_path);
                    let (title, tags) = extract_title_tags(&abs).await;
                    let id = path_hash(&key);
                    self.upsert_index_row(&id, &title, &tags, &key, mtime)
                        .await?;
                    result.updated += 1;
                }
                None => {
                    // 新文件
                    let abs = self.vault_path.join(rel_path);
                    let (title, tags) = extract_title_tags(&abs).await;
                    let id = path_hash(&key);
                    self.upsert_index_row(&id, &title, &tags, &key, mtime)
                        .await?;
                    result.added += 1;
                }
            }
        }

        // 删除已不存在的文件的索引行
        let current_keys: std::collections::HashSet<String> = files
            .iter()
            .map(|(p, _)| p.to_string_lossy().replace('\\', "/"))
            .collect();
        for old_key in existing.keys() {
            if !current_keys.contains(old_key) {
                self.delete_index_row(old_key).await?;
                result.deleted += 1;
            }
        }

        Ok(result)
    }

    /// 全量重建索引
    pub async fn rebuild_index(&self, exclude_dirs: &[&str]) -> AppResult<usize> {
        // 清空现有索引
        {
            let conn = self.vault_db.lock().await;
            conn.execute("DELETE FROM notes_index", [])
                .map_err(|e| AppError::Storage(format!("clear notes_index: {e}")))?;
        }

        let files = collect_md_files(&self.vault_path, exclude_dirs).await?;
        let count = files.len();

        for (rel_path, mtime) in files {
            let key = rel_path.to_string_lossy().replace('\\', "/");
            let abs = self.vault_path.join(&rel_path);
            let (title, tags) = extract_title_tags(&abs).await;
            let id = path_hash(&key);
            self.upsert_index_row(&id, &title, &tags, &key, &mtime)
                .await?;
        }

        Ok(count)
    }

    // ── 内部辅助 ──────────────────────────────────────────────────────────────

    async fn upsert_index_row(
        &self,
        id: &str,
        title: &str,
        tags: &[String],
        file_path: &str,
        modified_at: &str,
    ) -> AppResult<()> {
        let tags_json = serde_json::to_string(tags).unwrap_or_else(|_| "[]".to_string());
        let conn = self.vault_db.lock().await;
        conn.execute(
            "INSERT INTO notes_index (id, title, tags, file_path, modified_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(file_path) DO UPDATE SET
               id          = excluded.id,
               title       = excluded.title,
               tags        = excluded.tags,
               modified_at = excluded.modified_at",
            rusqlite::params![id, title, tags_json, file_path, modified_at],
        )
        .map_err(|e| AppError::Storage(format!("upsert note index: {e}")))?;
        Ok(())
    }

    async fn delete_index_row(&self, file_path: &str) -> AppResult<()> {
        let conn = self.vault_db.lock().await;
        conn.execute(
            "DELETE FROM notes_index WHERE file_path = ?1",
            rusqlite::params![file_path],
        )
        .map_err(|e| AppError::Storage(format!("delete note index: {e}")))?;
        Ok(())
    }

    async fn load_index_map(&self) -> AppResult<std::collections::HashMap<String, String>> {
        let conn = self.vault_db.lock().await;
        let mut stmt = conn
            .prepare("SELECT file_path, modified_at FROM notes_index")
            .map_err(|e| AppError::Storage(format!("load index map: {e}")))?;
        let mapped = match stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        }) {
            Ok(m) => m,
            Err(e) => return Err(AppError::Storage(format!("query index map: {e}"))),
        };
        mapped
            .collect::<rusqlite::Result<std::collections::HashMap<_, _>>>()
            .map_err(|e| AppError::Storage(format!("collect index map: {e}")))
    }
}

// ─── 辅助函数 ──────────────────────────────────────────────────────────────────

/// 迭代式收集 vault 下所有 .md 文件（用栈模拟递归），排除指定顶级目录
async fn collect_md_files(
    vault: &Path,
    exclude_dirs: &[&str],
) -> AppResult<Vec<(PathBuf, String)>> {
    let mut result = Vec::new();
    // 用栈存待处理目录
    let mut stack: Vec<PathBuf> = vec![vault.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let mut entries = match tokio::fs::read_dir(&dir).await {
            Ok(e) => e,
            Err(_) => continue, // 跳过不可访问的目录
        };

        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            if path.is_dir() {
                // 只在 vault 根目录下排除 exclude_dirs
                let is_vault_root_child = dir == vault;
                if is_vault_root_child && exclude_dirs.contains(&name_str.as_ref()) {
                    continue;
                }
                stack.push(path);
            } else if path.extension().and_then(|s| s.to_str()) == Some("md") {
                let mtime = entry
                    .metadata()
                    .await
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs().to_string())
                    .unwrap_or_default();
                let rel = path.strip_prefix(vault).unwrap_or(&path).to_path_buf();
                result.push((rel, mtime));
            }
        }
    }

    Ok(result)
}

/// 从 Markdown 文件提取标题和标签（尝试解析 Frontmatter，失败则用文件名）
async fn extract_title_tags(path: &Path) -> (String, Vec<String>) {
    #[derive(serde::Deserialize)]
    struct MinFm {
        #[serde(default)]
        title: Option<String>,
        #[serde(default)]
        tags: Vec<String>,
    }

    if let Ok(content) = tokio::fs::read_to_string(path).await {
        if let Ok((fm, _)) = crate::storage::markdown::parse_frontmatter::<MinFm>(&content) {
            let title = fm.title.unwrap_or_else(|| {
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("untitled")
                    .to_string()
            });
            return (title, fm.tags);
        }
    }

    // 无 Frontmatter：用文件名作标题
    let title = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("untitled")
        .to_string();
    (title, Vec::new())
}

/// 用 file_path 生成稳定 id（sha256 前 16 hex 字符）
fn path_hash(path: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    path.hash(&mut hasher);
    let h = hasher.finish();
    format!("{h:016x}")
}
