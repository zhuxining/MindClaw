//! TaskService：任务 CRUD
//!
//! 真相源是 `{vault}/tasks/*.md` 文件（YAML Frontmatter），
//! `tasks_index` 表是可重建的索引层。

use crate::error::{AppError, AppResult};
use crate::models::task::{Task, TaskFrontmatter, TaskPriority, TaskStatus};
use crate::storage::markdown;
use rusqlite::{Connection, OptionalExtension};
use serde_json;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct TaskService {
    vault_db: Arc<Mutex<Connection>>,
    tasks_dir: PathBuf,
}

// ─── 输入类型 ──────────────────────────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct TaskFilter {
    pub status: Option<TaskStatus>,
    pub priority: Option<TaskPriority>,
    /// YYYY-MM-DD：只返回 due_date <= 此值的任务
    pub due_before: Option<String>,
    pub tag: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug)]
pub struct CreateTaskInput {
    pub title: String,
    pub body: Option<String>,
    pub status: Option<TaskStatus>,
    pub priority: Option<TaskPriority>,
    pub due_date: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Default)]
pub struct UpdateTaskInput {
    pub title: Option<String>,
    pub body: Option<Option<String>>,
    pub status: Option<TaskStatus>,
    pub priority: Option<TaskPriority>,
    /// `Some(None)` = 清除 due_date
    pub due_date: Option<Option<String>>,
    pub tags: Option<Vec<String>>,
}

// ─── TaskService ───────────────────────────────────────────────────────────────

impl TaskService {
    pub fn new(vault_db: Arc<Mutex<Connection>>, tasks_dir: PathBuf) -> Self {
        Self {
            vault_db,
            tasks_dir,
        }
    }

    /// 列出任务（查索引，支持过滤）
    pub async fn list(&self, filter: TaskFilter) -> AppResult<Vec<Task>> {
        let rows = {
            let conn = self.vault_db.lock().await;
            let mut sql =
                "SELECT id, title, status, priority, due_date, tags, file_path, created, updated \
                           FROM tasks_index WHERE 1=1"
                    .to_string();
            let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

            if let Some(ref s) = filter.status {
                let sv = status_str(s);
                sql.push_str(&format!(" AND status = ?{}", params.len() + 1));
                params.push(Box::new(sv.to_string()));
            }
            if let Some(ref p) = filter.priority {
                let pv = priority_str(p);
                sql.push_str(&format!(" AND priority = ?{}", params.len() + 1));
                params.push(Box::new(pv.to_string()));
            }
            if let Some(ref d) = filter.due_before {
                sql.push_str(&format!(
                    " AND due_date IS NOT NULL AND due_date <= ?{}",
                    params.len() + 1
                ));
                params.push(Box::new(d.clone()));
            }
            if let Some(ref t) = filter.tag {
                let pattern = format!("%\"{t}\"%");
                sql.push_str(&format!(" AND tags LIKE ?{}", params.len() + 1));
                params.push(Box::new(pattern));
            }
            sql.push_str(" ORDER BY updated DESC");
            if let Some(limit) = filter.limit {
                sql.push_str(&format!(" LIMIT {limit}"));
            }

            let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

            let mut stmt = conn
                .prepare(&sql)
                .map_err(|e| AppError::Storage(format!("list tasks: {e}")))?;

            let mapped = match stmt.query_map(param_refs.as_slice(), row_to_index_row) {
                Ok(m) => m,
                Err(e) => return Err(AppError::Storage(format!("query tasks: {e}"))),
            };
            let rows: Vec<IndexRow> = mapped
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|e| AppError::Storage(format!("collect tasks: {e}")))?;
            rows
        };

        // 转换为 Task（从索引数据构建，不读文件 body）
        rows.into_iter()
            .map(|r| index_row_to_task(r, &self.tasks_dir))
            .collect()
    }

    /// 按 ID 获取完整任务（从文件读取 body）
    pub async fn get(&self, id: &str) -> AppResult<Task> {
        let file_path = self.file_path_from_index(id).await?;
        let abs = self
            .tasks_dir
            .parent()
            .unwrap_or(&self.tasks_dir)
            .join(&file_path);
        let (fm, body): (TaskFrontmatter, String) = markdown::read_file(&abs).await?;
        Ok(frontmatter_to_task(fm, Some(body), file_path))
    }

    /// 创建任务（写文件 + 更新索引）
    pub async fn create(&self, input: CreateTaskInput) -> AppResult<Task> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let fm = TaskFrontmatter {
            id: id.clone(),
            title: input.title.clone(),
            status: input.status.unwrap_or(TaskStatus::Todo),
            priority: input.priority.unwrap_or(TaskPriority::Medium),
            due_date: input.due_date,
            tags: input.tags,
            created: now.clone(),
            updated: now.clone(),
        };
        let body = input.body.unwrap_or_default();
        let abs_path = markdown::task_file_path(&self.tasks_dir, &input.title, &now, &id);
        markdown::write_file(&abs_path, &fm, &body).await?;

        let rel = self.relative_path(&abs_path);
        self.upsert_index(&fm, &path_str(&rel)).await?;

        Ok(frontmatter_to_task(fm, Some(body), rel))
    }

    /// 更新任务（读-改-写文件 + 更新索引）
    pub async fn update(&self, id: &str, input: UpdateTaskInput) -> AppResult<Task> {
        let file_path = self.file_path_from_index(id).await?;
        let abs = self
            .tasks_dir
            .parent()
            .unwrap_or(&self.tasks_dir)
            .join(&file_path);
        let (mut fm, mut body): (TaskFrontmatter, String) = markdown::read_file(&abs).await?;

        if let Some(title) = input.title {
            fm.title = title;
        }
        if let Some(status) = input.status {
            fm.status = status;
        }
        if let Some(priority) = input.priority {
            fm.priority = priority;
        }
        if let Some(due_date) = input.due_date {
            fm.due_date = due_date;
        }
        if let Some(tags) = input.tags {
            fm.tags = tags;
        }
        if let Some(b) = input.body {
            body = b.unwrap_or_default();
        }
        fm.updated = chrono::Utc::now().to_rfc3339();

        markdown::write_file(&abs, &fm, &body).await?;
        self.upsert_index(&fm, &path_str(&file_path)).await?;

        Ok(frontmatter_to_task(fm, Some(body), file_path))
    }

    /// 删除任务（删除文件 + 删除索引行）
    pub async fn delete(&self, id: &str) -> AppResult<()> {
        let file_path = self.file_path_from_index(id).await?;
        let abs = self
            .tasks_dir
            .parent()
            .unwrap_or(&self.tasks_dir)
            .join(&file_path);
        if abs.exists() {
            tokio::fs::remove_file(&abs)
                .await
                .map_err(|e| AppError::Storage(format!("delete task file: {e}")))?;
        }
        let conn = self.vault_db.lock().await;
        conn.execute(
            "DELETE FROM tasks_index WHERE id = ?1",
            rusqlite::params![id],
        )
        .map_err(|e| AppError::Storage(format!("delete task index: {e}")))?;
        Ok(())
    }

    /// 全量重建 tasks_index（扫描 tasks_dir 下所有 .md 文件）
    pub async fn rebuild_index(&self) -> AppResult<usize> {
        if !self.tasks_dir.exists() {
            return Ok(0);
        }
        let mut count = 0usize;
        let mut entries = tokio::fs::read_dir(&self.tasks_dir)
            .await
            .map_err(|e| AppError::Storage(format!("read tasks dir: {e}")))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| AppError::Storage(format!("iterate tasks dir: {e}")))?
        {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("md") {
                continue;
            }
            if let Ok((fm, _)) = markdown::read_file::<TaskFrontmatter>(&path).await {
                let rel = self.relative_path(&path);
                self.upsert_index(&fm, &path_str(&rel)).await?;
                count += 1;
            }
        }
        Ok(count)
    }

    // ── 内部辅助 ──────────────────────────────────────────────────────────────

    async fn upsert_index(&self, fm: &TaskFrontmatter, file_path: &str) -> AppResult<()> {
        let tags = serde_json::to_string(&fm.tags).unwrap_or_else(|_| "[]".to_string());
        let conn = self.vault_db.lock().await;
        conn.execute(
            "INSERT INTO tasks_index
               (id, title, status, priority, due_date, tags, file_path, created, updated)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)
             ON CONFLICT(id) DO UPDATE SET
               title     = excluded.title,
               status    = excluded.status,
               priority  = excluded.priority,
               due_date  = excluded.due_date,
               tags      = excluded.tags,
               file_path = excluded.file_path,
               updated   = excluded.updated",
            rusqlite::params![
                fm.id,
                fm.title,
                status_str(&fm.status),
                priority_str(&fm.priority),
                fm.due_date,
                tags,
                file_path,
                fm.created,
                fm.updated,
            ],
        )
        .map_err(|e| AppError::Storage(format!("upsert task index: {e}")))?;
        Ok(())
    }

    async fn file_path_from_index(&self, id: &str) -> AppResult<PathBuf> {
        let conn = self.vault_db.lock().await;
        let path: String = conn
            .query_row(
                "SELECT file_path FROM tasks_index WHERE id = ?1",
                rusqlite::params![id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| AppError::Storage(format!("lookup task: {e}")))?
            .ok_or_else(|| AppError::NotFound(format!("task {id} not found")))?;
        Ok(PathBuf::from(path))
    }

    fn relative_path(&self, abs_path: &std::path::Path) -> PathBuf {
        let vault_root = self.tasks_dir.parent().unwrap_or(&self.tasks_dir);
        abs_path
            .strip_prefix(vault_root)
            .unwrap_or(abs_path)
            .to_path_buf()
    }
}

// ─── 辅助转换 ──────────────────────────────────────────────────────────────────

fn status_str(s: &TaskStatus) -> &'static str {
    match s {
        TaskStatus::Todo => "todo",
        TaskStatus::InProgress => "in_progress",
        TaskStatus::Done => "done",
        TaskStatus::Cancelled => "cancelled",
    }
}

fn priority_str(p: &TaskPriority) -> &'static str {
    match p {
        TaskPriority::Low => "low",
        TaskPriority::Medium => "medium",
        TaskPriority::High => "high",
    }
}

fn parse_status(s: &str) -> TaskStatus {
    match s {
        "in_progress" => TaskStatus::InProgress,
        "done" => TaskStatus::Done,
        "cancelled" => TaskStatus::Cancelled,
        _ => TaskStatus::Todo,
    }
}

fn parse_priority(s: &str) -> TaskPriority {
    match s {
        "low" => TaskPriority::Low,
        "high" => TaskPriority::High,
        _ => TaskPriority::Medium,
    }
}

/// 公开的优先级解析（供 commands 使用）
pub fn parse_priority_pub(s: &str) -> TaskPriority {
    parse_priority(s)
}

// (id, title, status, priority, due_date, tags_json, file_path, created, updated)
type IndexRow = (
    String,
    String,
    String,
    String,
    Option<String>,
    String,
    String,
    String,
    String,
);

fn row_to_index_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<IndexRow> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
        row.get(6)?,
        row.get(7)?,
        row.get(8)?,
    ))
}

fn index_row_to_task(r: IndexRow, tasks_dir: &std::path::Path) -> AppResult<Task> {
    let (id, title, status_s, priority_s, due_date, tags_json, file_path, created, updated) = r;
    let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
    let vault_root = tasks_dir.parent().unwrap_or(tasks_dir);
    Ok(Task {
        id,
        title,
        status: parse_status(&status_s),
        priority: parse_priority(&priority_s),
        due_date,
        tags,
        created,
        updated,
        body: None, // 列表不读文件 body
        file_path: vault_root.join(&file_path),
    })
}

fn frontmatter_to_task(fm: TaskFrontmatter, body: Option<String>, rel: PathBuf) -> Task {
    Task {
        id: fm.id,
        title: fm.title,
        status: fm.status,
        priority: fm.priority,
        due_date: fm.due_date,
        tags: fm.tags,
        created: fm.created,
        updated: fm.updated,
        body,
        file_path: rel,
    }
}

fn path_str(p: &std::path::Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}
