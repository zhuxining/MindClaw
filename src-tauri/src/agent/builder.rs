//! AgentBuilder：Builder 模式构建 AgentLoop
//!
//! 封装 Agent 内部组件的初始化逻辑（Tools、ContextPipeline、Commands、Observer）

use crate::agent::commands::AgentCommandRegistry;
use crate::agent::context::ContextPipeline;
use crate::agent::observer::{AgentObserver, TracingObserver};
use crate::agent::tools::mcp::{MCPManager, register_mcp_tools};
use crate::agent::tools::traits::Tool;
use crate::agent::tools::{
    file_edit::FileEditTool, file_read::FileReadTool, file_write::FileWriteTool,
    find_files::FindFilesTool, path_guard::PathGuard, search_content::SearchContentTool,
    shell::ShellTool, ToolRegistry,
};
use crate::agent::AgentLoop;
use crate::bus::MessageBus;
use crate::error::{AppError, AppResult};
use crate::providers::config::builtin_configs;
use crate::providers::openai_compat::OpenAICompatProvider;
use crate::providers::Provider;
use crate::runtime::config::AppConfig;
use std::sync::Arc;

/// Agent 构建器
pub struct AgentBuilder {
    config: Arc<AppConfig>,
    bus: Arc<MessageBus>,
    session_mgr: Arc<crate::agent::SessionManager>,
    extra_tools: Vec<Arc<dyn Tool + Send + Sync>>,
    observer: Option<Arc<dyn AgentObserver>>,
}

impl AgentBuilder {
    /// 创建新的 AgentBuilder
    pub fn new(
        config: Arc<AppConfig>,
        bus: Arc<MessageBus>,
        session_mgr: Arc<crate::agent::SessionManager>,
    ) -> Self {
        Self {
            config,
            bus,
            session_mgr,
            extra_tools: Vec::new(),
            observer: None,
        }
    }

    /// 添加额外工具
    pub fn add_tool(mut self, tool: Arc<dyn Tool + Send + Sync>) -> Self {
        self.extra_tools.push(tool);
        self
    }

    /// 设置观测器
    pub fn observer(mut self, obs: Arc<dyn AgentObserver>) -> Self {
        self.observer = Some(obs);
        self
    }

    /// 构建 AgentLoop
    pub async fn build(self) -> AppResult<AgentLoop> {
        // 1. 初始化 Provider
        let provider = init_provider(&self.config)?;

        // 2. 初始化 ToolRegistry
        let tools = init_tools(&self.config, self.extra_tools).await?;

        // 3. 创建 AgentCommandRegistry
        let agent_commands = Arc::new(AgentCommandRegistry::with_builtins());

        // 4. 构建 ContextPipeline
        let context_pipeline = ContextPipeline::build_default(&self.config);

        // 5. 创建 Observer
        let observer = self
            .observer
            .unwrap_or_else(|| Arc::new(TracingObserver::new()));

        // 6. 创建 AgentLoop
        Ok(AgentLoop::new(
            self.bus,
            self.session_mgr,
            context_pipeline,
            provider,
            tools,
            agent_commands,
            observer,
            self.config,
        ))
    }
}

/// 根据配置初始化 LLM Provider
fn init_provider(config: &AppConfig) -> AppResult<Arc<dyn Provider>> {
    let configs = builtin_configs();
    let provider_config = configs
        .iter()
        .find(|c| c.name == config.provider_id)
        .ok_or_else(|| {
            AppError::Provider(format!(
                "provider '{}' not found in builtin configs",
                config.provider_id
            ))
        })?;

    let provider = OpenAICompatProvider::from_env(provider_config, config.model_id.as_deref())
        .map_err(|e| {
            AppError::Provider(format!(
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

/// 初始化默认工具 + 额外工具
async fn init_tools(
    config: &AppConfig,
    extra: Vec<Arc<dyn Tool + Send + Sync>>,
) -> AppResult<Arc<ToolRegistry>> {
    let mut tools = ToolRegistry::new().with_concurrency_limit(config.tool_concurrency);

    // 内置工具
    let guard = Arc::new(PathGuard::vault_only(config.vault_path.clone()).with_denied("private"));
    tools.register(Arc::new(ShellTool::new(config.data_dir.clone())));
    tools.register(Arc::new(FileReadTool::with_guard(Arc::clone(&guard))));
    tools.register(Arc::new(FileWriteTool::with_guard(Arc::clone(&guard))));
    tools.register(Arc::new(FileEditTool::with_guard(Arc::clone(&guard))));
    tools.register(Arc::new(FindFilesTool::new(Arc::clone(&guard))));
    tools.register(Arc::new(SearchContentTool::new(Arc::clone(&guard))));

    // MCP proxy tools：从 data_dir/mcp.toml 加载外部 server 配置
    let mcp_manager = MCPManager::from_file(&config.data_dir);
    if mcp_manager.server_count() > 0 {
        if let Err(e) = register_mcp_tools(&mut tools, &mcp_manager).await {
            tracing::warn!(error = %e, "MCP registration failed, continuing without MCP tools");
        } else {
            tracing::info!(servers = %mcp_manager.server_count(), "MCP bridge initialized");
        }
    }

    // 额外工具
    for tool in extra {
        tools.register(tool);
    }

    tracing::info!(tool_count = %tools.len(), "tools_initialized");
    Ok(Arc::new(tools))
}
