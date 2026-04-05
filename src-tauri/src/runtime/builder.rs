//! AppRuntimeBuilder：Builder 模式构建 AppRuntime
//!
//! 从 CliRuntime::new_with_agent() 提取的通用初始化逻辑

use crate::agent::{AgentLoop, AgentRegistry, ModelRouter};
use crate::bus::MessageBus;
use crate::error::AppResult;
use crate::providers::config::builtin_configs;
use crate::providers::openai_compat::OpenAICompatProvider;
use crate::providers::Provider;
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

        // 6. 初始化 Provider / AgentRegistry / ModelRouter
        let provider = init_provider(&config)?;
        let agent_registry = Arc::new(AgentRegistry::bootstrap(&config, provider.model_id()));
        let model_router = Arc::new(ModelRouter::new());

        // 7. 初始化 AgentLoop（含 AgentSpawnDispatcher 和 SpawnBackgroundAgentTool）
        let agent = Arc::new(
            AgentLoop::init(
                config.clone(),
                bus.clone(),
                session_mgr.clone(),
                provider,
                agent_registry.clone(),
                model_router.clone(),
            )
            .await?,
        );

        let shutdown = CancellationToken::new();

        Ok(AppRuntime {
            db,
            services,
            bus,
            agent,
            agent_registry,
            model_router,
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

fn init_provider(config: &AppConfig) -> AppResult<Arc<dyn Provider>> {
    let configs = builtin_configs();
    let provider_config = configs
        .iter()
        .find(|c| c.name == config.provider_id)
        .ok_or_else(|| {
            crate::error::AppError::Provider(format!(
                "provider '{}' not found in builtin configs",
                config.provider_id
            ))
        })?;

    let provider = OpenAICompatProvider::from_env(provider_config, config.model_id.as_deref())
        .map_err(|e| {
            crate::error::AppError::Provider(format!(
                "failed to initialize {} provider: {}",
                config.provider_id, e
            ))
        })?;

    tracing::info!(
        provider = %config.provider_id,
        model = %provider.model_id(),
        "provider_initialized"
    );

    Ok(Arc::new(provider))
}
