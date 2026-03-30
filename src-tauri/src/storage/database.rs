//! SQLite 连接管理与 Schema 初始化

use crate::error::{AppError, AppResult};
use rusqlite::Connection;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Schema 版本号。增加此值时在 `migrate` 中追加对应的迁移逻辑。
const SCHEMA_VERSION: u32 = 1;

/// 打开（或创建）数据库并执行 Schema 迁移，返回 `Arc<Mutex<Connection>>`
pub fn open(path: &Path) -> AppResult<Arc<Mutex<Connection>>> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let conn = Connection::open(path).map_err(|e| AppError::Storage(format!("open db: {e}")))?;

    // 基础 pragma
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA busy_timeout = 5000;
         PRAGMA foreign_keys = ON;",
    )
    .map_err(|e| AppError::Storage(format!("pragma: {e}")))?;

    migrate(&conn)?;

    Ok(Arc::new(Mutex::new(conn)))
}

/// 打开内存数据库（测试用）
#[cfg(test)]
pub fn open_memory() -> AppResult<Arc<Mutex<Connection>>> {
    let conn = Connection::open_in_memory()
        .map_err(|e| AppError::Storage(format!("open memory db: {e}")))?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(|e| AppError::Storage(format!("pragma: {e}")))?;
    migrate(&conn)?;
    Ok(Arc::new(Mutex::new(conn)))
}

fn migrate(conn: &Connection) -> AppResult<()> {
    let current: u32 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|e| AppError::Storage(format!("read user_version: {e}")))?;

    if current < 1 {
        conn.execute_batch(include_str!("migrations/001_init.sql"))
            .map_err(|e| AppError::Storage(format!("migration 001: {e}")))?;
    }

    if current < SCHEMA_VERSION {
        conn.pragma_update(None, "user_version", SCHEMA_VERSION)
            .map_err(|e| AppError::Storage(format!("set user_version: {e}")))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_memory_and_migrate() {
        let db = open_memory().expect("should open memory db");
        let conn = db.blocking_lock();
        let version: u32 = conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);

        // 验证 sessions 表存在
        let count: u32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='sessions'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }
}
