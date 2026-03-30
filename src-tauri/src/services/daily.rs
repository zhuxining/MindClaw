// DailyService：日记读写、模板创建、条目追加

use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct DailyService {
    #[allow(dead_code)]
    db: Arc<Mutex<Connection>>,
    #[allow(dead_code)]
    vault_path: PathBuf,
}

impl DailyService {
    pub fn new(db: Arc<Mutex<Connection>>, vault_path: &Path) -> Self {
        Self {
            db,
            vault_path: vault_path.to_path_buf(),
        }
    }
}
