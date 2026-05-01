//! AppRuntimeBuilder：Builder 模式构建 AppRuntime
//!
//! 从配置文件加载两层配置（UserConfig + VaultConfig），合并为运行时 AppConfig，
//! 打开双 DB（全局 DB + vault DB），初始化所有服务。

use crate::agent::{AgentLoop, AgentRegistry, AgentRunner, ModelRouter};
use crate::bus::MessageBus;
use crate::error::AppResult;
use crate::providers::ProviderRegistry;
use crate::runtime::config::{AppConfig, ConfigLoader};
use crate::runtime::services::ServiceContainer;
use crate::runtime::AppRuntime;
use crate::storage::database;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

/// AppRuntime 构建器
pub struct AppRuntimeBuilder {
    /// 可选覆盖：用户数据目录（默认 ~/.config/mindclaw）
    user_data_dir: Option<PathBuf>,
    /// 可选覆盖：vault 路径（优先于 UserConfig.active_vault_path）
    vault_path: Option<PathBuf>,
    /// 可选覆盖：provider id
    provider_id: Option<String>,
    /// 可选覆盖：model id
    model_id: Option<String>,
}

impl AppRuntimeBuilder {
    pub fn new() -> Self {
        Self {
            user_data_dir: None,
            vault_path: None,
            provider_id: None,
            model_id: None,
        }
    }

    pub fn user_data_dir(mut self, path: PathBuf) -> Self {
        self.user_data_dir = Some(path);
        self
    }

    /// 向后兼容：设置 data_dir
    pub fn data_dir(self, path: PathBuf) -> Self {
        self.user_data_dir(path)
    }

    pub fn vault_path(mut self, path: PathBuf) -> Self {
        self.vault_path = Some(path);
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

    /// 构建 AppRuntime（不启动后台任务，需调用 start()）
    pub async fn build(self) -> AppResult<AppRuntime> {
        // 1. 确定 user_data_dir
        let user_data_dir = self.user_data_dir.unwrap_or_else(|| {
            AppConfig::default_user_data_dir().unwrap_or_else(|_| PathBuf::from(".mindclaw-data"))
        });

        // 2. 加载 UserConfig（文件不存在时使用默认值）
        let mut user_config = ConfigLoader::load_user_config(&user_data_dir)?;

        // 3. 应用命令行覆盖
        if let Some(id) = self.provider_id {
            user_config.active_provider_id = id;
        }

        // 4. 确定当前 vault 路径
        let vault_path = self
            .vault_path
            .or_else(|| {
                // 从 active_vault_path 直接获取
                user_config.active_vault_path.clone()
            })
            .unwrap_or_else(|| user_data_dir.join("vault"));

        // 5. 加载 VaultConfig（文件不存在时使用默认值）
        let vault_config = ConfigLoader::load_vault_config(&vault_path)?;

        // 6. 构造临时 VaultEntry 用于 merge
        let vault_entry = crate::runtime::config::VaultEntry {
            name: vault_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("vault")
                .to_string(),
            path: vault_path,
            last_opened: None,
        };

        // 7. 合并为 AppConfig，应用 model 覆盖
        let mut config = ConfigLoader::merge(&user_config, &vault_entry, &vault_config);
        if let Some(model) = self.model_id {
            config.model_id = Some(model);
        }
        let config = Arc::new(config);

        // 8. 打开全局 DB（sessions/turns/messages）
        tracing::info!(path = %config.global_db_path.display(), "initializing_global_db");
        let global_db = database::open_global(&config.global_db_path)?;

        // 9. 打开 vault DB（tasks/notes/memories 索引）
        tracing::info!(path = %config.vault_db_path.display(), "initializing_vault_db");
        let vault_db = database::open_vault(&config.vault_db_path)?;

        // 10. 创建 ServiceContainer（使用 vault_db）
        let services = Arc::new(ServiceContainer::new(vault_db.clone(), &config)?);

        // 11. 创建 SessionManager（使用 global_db）
        let session_mgr = Arc::new(crate::agent::SessionManager::new(global_db.clone()));

        // 12. 创建 MessageBus
        let bus = Arc::new(MessageBus::new(config.bus_capacity));

        // 13. 初始化 ProviderRegistry（Adapter 层）
        let provider_registry = ProviderRegistry::new();
        let agent_models = provider_registry.create_agent_models_from_env(&config)?;
        let runner = Arc::new(AgentRunner::new(agent_models.clone()));

        let agent_registry = Arc::new(AgentRegistry::bootstrap(
            &config,
            &agent_models.main_model_id,
            &agent_models.light_model_id,
        ));
        let model_router = Arc::new(ModelRouter::new());

        // 14. 初始化 AgentLoop
        let agent = Arc::new(
            AgentLoop::init(
                config.clone(),
                bus.clone(),
                session_mgr.clone(),
                Arc::clone(&runner),
                agent_registry.clone(),
                model_router.clone(),
            )
            .await?,
        );

        let shutdown = CancellationToken::new();

        Ok(AppRuntime {
            global_db,
            vault_db,
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
