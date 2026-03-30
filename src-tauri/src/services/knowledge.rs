// KnowledgeService：知识笔记 CRUD、wikilink 提取、索引同步

use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct KnowledgeService {
    #[allow(dead_code)]
    db: Arc<Mutex<Connection>>,
    #[allow(dead_code)]
    vault_path: PathBuf,
}

impl KnowledgeService {
    pub fn new(db: Arc<Mutex<Connection>>, vault_path: &Path) -> Self {
        Self {
            db,
            vault_path: vault_path.to_path_buf(),
        }
    }
}
