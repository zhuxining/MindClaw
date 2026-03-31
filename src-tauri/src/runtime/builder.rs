//! AppRuntimeBuilder：Builder 模式构建 AppRuntime
//!
//! 从 CliRuntime::new_with_agent() 提取的通用初始化逻辑

use crate::agent::AgentBuilder;
use crate::bus::MessageBus;
use crate::error::AppResult;
use crate::runtime::config::AppConfig;
use crate::runtime::services::ServiceContainer;
use crate::runtime::AppRuntime;
use crate::storage::database;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

/// AppRuntime 构建器
pub struct AppRuntimeBuilder {
    data_dir: Option<PathBuf>,
    provider_id: Option<String>,
    model_id: Option<String>,
    bus_capacity: Option<usize>,
    context_token_limit: Option<usize>,
    system_prompt: Option<String>,
}

impl AppRuntimeBuilder {
    pub fn new() -> Self {
        Self {
            data_dir: None,
            provider_id: None,
            model_id: None,
            bus_capacity: None,
            context_token_limit: None,
            system_prompt: None,
        }
    }

    pub fn data_dir(mut self, path: PathBuf) -> Self {
        self.data_dir = Some(path);
        self
    }

    pub fn provider(mut self, id: impl Into<String>) -> Self {
        self.provider_id = Some(id.into());
        self
    }

    pub fn model(mut self, id: impl Into<String>) -> Self {
        self.model_id = Some(id.into());
        self
    }

    pub fn bus_capacity(mut self, cap: usize) -> Self {
        self.bus_capacity = Some(cap);
        self
    }

    pub fn context_token_limit(mut self, limit: usize) -> Self {
        self.context_token_limit = Some(limit);
        self
    }

    pub fn system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// 构建 AppRuntime（不启动后台任务，需调用 start()）
    pub async fn build(self) -> AppResult<AppRuntime> {
        // 1. 组装配置
        let mut config = AppConfig::default();
        if let Some(dir) = self.data_dir {
            config.db_path = dir.join("mindclaw.db");
            config.vault_path = dir.join("vault");
            config.data_dir = dir;
        }
        if let Some(id) = self.provider_id {
            config.provider_id = id;
        }
        if let Some(id) = self.model_id {
            config.model_id = Some(id);
        }
        if let Some(cap) = self.bus_capacity {
            config.bus_capacity = cap;
        }
        if let Some(limit) = self.context_token_limit {
            config.context_token_limit = limit;
        }
        if let Some(prompt) = self.system_prompt {
            config.system_prompt = prompt;
        }
        let config = Arc::new(config);

        // 2. 打开数据库
        tracing::info!(db_path = %config.db_path.display(), "initializing_database");
        let db = database::open(&config.db_path)?;

        // 3. 创建 ServiceContainer
        let services = Arc::new(ServiceContainer::new(db.clone(), &config)?);

        // 4. 创建 SessionManager
        let session_mgr = Arc::new(crate::agent::SessionManager::new(db.clone()));

        // 5. 创建 MessageBus
        let bus = Arc::new(MessageBus::new(config.bus_capacity));

        // 6. 使用 AgentBuilder 构建 AgentLoop
        let agent_builder = AgentBuilder::new(config.clone(), bus.clone(), session_mgr.clone());
        let agent = Arc::new(agent_builder.build().await?);

        let shutdown = CancellationToken::new();

        Ok(AppRuntime {
            db,
            services,
            bus,
            agent,
            session_mgr,
            config,
            shutdown,
            tasks: Mutex::new(Vec::new()),
        })
    }
}

impl Default for AppRuntimeBuilder {
    fn default() -> Self {
        Self::new()
    }
}
