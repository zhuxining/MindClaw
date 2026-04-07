//! Vault 级 SQLite DB 管理
//!
//! 存储 vault 内的索引数据：tasks_index、notes_index、memories_index。
//! 索引可丢弃，随时可从 Markdown 文件重建。

use crate::error::{AppError, AppResult};
use rusqlite::Connection;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

const SCHEMA_VERSION: u32 = 1;

/// 打开（或创建）vault DB 并执行 Schema 迁移
pub fn open(path: &Path) -> AppResult<Arc<Mutex<Connection>>> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let conn =
        Connection::open(path).map_err(|e| AppError::Storage(format!("open vault db: {e}")))?;

    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA busy_timeout = 5000;
         PRAGMA foreign_keys = ON;",
    )
    .map_err(|e| AppError::Storage(format!("vault db pragma: {e}")))?;

    migrate(&conn)?;

    Ok(Arc::new(Mutex::new(conn)))
}

/// 打开内存 vault DB（测试用）
#[cfg(test)]
pub fn open_memory() -> AppResult<Arc<Mutex<Connection>>> {
    let conn = Connection::open_in_memory()
        .map_err(|e| AppError::Storage(format!("open vault memory db: {e}")))?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(|e| AppError::Storage(format!("vault memory db pragma: {e}")))?;
    migrate(&conn)?;
    Ok(Arc::new(Mutex::new(conn)))
}

fn migrate(conn: &Connection) -> AppResult<()> {
    let current: u32 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|e| AppError::Storage(format!("read vault user_version: {e}")))?;

    if current < 1 {
        conn.execute_batch(include_str!("../migrations/vault_001_init.sql"))
            .map_err(|e| AppError::Storage(format!("vault migration 001: {e}")))?;
    }

    if current < SCHEMA_VERSION {
        conn.pragma_update(None, "user_version", SCHEMA_VERSION)
            .map_err(|e| AppError::Storage(format!("set vault user_version: {e}")))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_memory_and_migrate() {
        let db = open_memory().expect("should open vault memory db");
        let conn = db.blocking_lock();
        let version: u32 = conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);

        for table in &["tasks_index", "notes_index", "memories_index"] {
            let count: u32 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                    rusqlite::params![table],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(count, 1, "table {table} should exist");
        }
    }
}
