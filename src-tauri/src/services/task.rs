// TaskService：任务 CRUD、状态管理

use rusqlite::Connection;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct TaskService {
    #[allow(dead_code)]
    db: Arc<Mutex<Connection>>,
}

impl TaskService {
    pub fn new(db: Arc<Mutex<Connection>>) -> Self {
        Self { db }
    }
}
