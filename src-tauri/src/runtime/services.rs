//! ServiceContainer：统一服务访问层
//!
//! 封装所有业务服务，由 AppRuntime 初始化后注入各入口点（Tauri/CLI/Gateway）

use crate::agent::memory::MemoryStore;
use crate::error::AppResult;
use crate::runtime::config::AppConfig;
use crate::services::{daily::DailyService, note::NoteService, task::TaskService};
use rusqlite::Connection;
use std::sync::Arc;
use tokio::sync::Mutex;

/// 统一服务容器
pub struct ServiceContainer {
    pub task: Arc<TaskService>,
    pub memory: Arc<MemoryStore>,
    pub note: Arc<NoteService>,
    pub daily: Arc<DailyService>,
}

impl ServiceContainer {
    /// 初始化所有服务（均使用 vault_db）
    pub fn new(vault_db: Arc<Mutex<Connection>>, config: &AppConfig) -> AppResult<Self> {
        let tasks_dir = config.vault_path.join(&config.folder_mappings.tasks);
        let memory_dir = config.vault_path.join(&config.folder_mappings.memory);

        Ok(Self {
            task: Arc::new(TaskService::new(vault_db.clone(), tasks_dir)),
            memory: Arc::new(MemoryStore::new(vault_db.clone(), memory_dir)),
            note: Arc::new(NoteService::new(
                vault_db.clone(),
                config.vault_path.clone(),
            )),
            daily: Arc::new(DailyService::new(vault_db, &config.vault_path)),
        })
    }
}
