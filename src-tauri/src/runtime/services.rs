//! ServiceContainer：统一服务访问层
//!
//! 封装所有业务服务，由 AppRuntime 初始化后注入各入口点（Tauri/CLI/Gateway）

use crate::error::AppResult;
use crate::runtime::config::AppConfig;
use crate::services::{daily::DailyService, knowledge::KnowledgeService, task::TaskService};
use rusqlite::Connection;
use std::sync::Arc;
use tokio::sync::Mutex;

/// 统一服务容器
pub struct ServiceContainer {
    pub knowledge: Arc<KnowledgeService>,
    pub daily: Arc<DailyService>,
    pub task: Arc<TaskService>,
}

impl ServiceContainer {
    /// 初始化所有服务
    pub fn new(db: Arc<Mutex<Connection>>, config: &AppConfig) -> AppResult<Self> {
        Ok(Self {
            knowledge: Arc::new(KnowledgeService::new(db.clone(), &config.vault_path)),
            daily: Arc::new(DailyService::new(db.clone(), &config.vault_path)),
            task: Arc::new(TaskService::new(db)),
        })
    }
}
