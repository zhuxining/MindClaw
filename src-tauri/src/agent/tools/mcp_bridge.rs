//! MCP Bridge：将外部 MCP server 的工具代理进 ToolRegistry
//!
//! 设计原则：Agent 不应看到黑盒的 "mcp_client"，而应看到具体 tool（如 `git_status`）。
//! McpBridgeManager 在启动时连接外部 server，为每个 tool 创建 McpProxyTool，
//! 批量注册到 ToolRegistry。
//!
//! MCP Server 用途（让外部 client 使用 MindClaw）在 gateway 层实现，不在此模块。

use super::traits::{Tool, ToolInput, ToolOutput};
use crate::error::{AppError, AppResult};
use rmcp::{
    model::CallToolRequestParams,
    service::{RoleClient, RunningService},
    transport::TokioChildProcess,
    ServiceExt,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;
use std::sync::Arc;
use tokio::process::Command;

// ── 配置 ──────────────────────────────────────────────

/// 单个 MCP server 的连接配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    pub transport: McpTransport,
}

/// mcp.toml 顶层结构
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct McpConfig {
    #[serde(default, rename = "server")]
    pub servers: Vec<McpServerConfig>,
}

impl McpConfig {
    /// 从 `data_dir/mcp.toml` 加载；文件不存在时返回空配置
    pub fn load_from_dir(data_dir: &Path) -> Self {
        let path = data_dir.join("mcp.toml");
        if !path.exists() {
            return Self::default();
        }
        match std::fs::read_to_string(&path) {
            Ok(content) => match toml::from_str(&content) {
                Ok(cfg) => {
                    tracing::info!(path = %path.display(), "loaded mcp.toml");
                    cfg
                }
                Err(e) => {
                    tracing::warn!(path = %path.display(), error = %e, "failed to parse mcp.toml, using empty config");
                    Self::default()
                }
            },
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %e, "failed to read mcp.toml");
                Self::default()
            }
        }
    }
}

/// MCP 传输方式
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum McpTransport {
    /// 启动子进程（如 `uvx mcp-server-git`、`npx @modelcontextprotocol/server-filesystem`）
    ChildProcess {
        command: String,
        #[serde(default)]
        args: Vec<String>,
        /// 额外注入的环境变量（如 API key）
        #[serde(default)]
        env: Vec<(String, String)>,
    },
}

// ── McpProxyTool ──────────────────────────────────────────────

/// 代理到外部 MCP server 某个具体 tool 的包装器
pub struct McpProxyTool {
    tool_name: String,
    tool_description: String,
    tool_schema: Value,
    server: Arc<McpServerConnection>,
}

impl McpProxyTool {
    fn new(
        tool_name: impl Into<String>,
        tool_description: impl Into<String>,
        tool_schema: Value,
        server: Arc<McpServerConnection>,
    ) -> Self {
        Self {
            tool_name: tool_name.into(),
            tool_description: tool_description.into(),
            tool_schema,
            server,
        }
    }
}

#[async_trait::async_trait]
impl Tool for McpProxyTool {
    fn name(&self) -> &str {
        &self.tool_name
    }

    fn description(&self) -> &str {
        &self.tool_description
    }

    fn input_schema(&self) -> Value {
        self.tool_schema.clone()
    }

    async fn execute(&self, input: ToolInput) -> AppResult<ToolOutput> {
        self.server
            .call_tool(&self.tool_name, input.parameters)
            .await
    }
}

// ── McpServerConnection ──────────────────────────────────────────────

/// 管理单个 MCP server 的连接生命周期
pub struct McpServerConnection {
    name: String,
    client: RunningService<RoleClient, ()>,
}

impl McpServerConnection {
    pub async fn connect(config: McpServerConfig) -> AppResult<Self> {
        tracing::info!(server = %config.name, "connecting to MCP server");

        let transport = match &config.transport {
            McpTransport::ChildProcess { command, args, env } => {
                let mut cmd = Command::new(command);
                cmd.args(args);
                for (k, v) in env {
                    cmd.env(k, v);
                }
                TokioChildProcess::new(cmd).map_err(|e| {
                    AppError::Internal(format!(
                        "MCP '{}': failed to start child process: {}",
                        config.name, e
                    ))
                })?
            }
        };

        let client = ().serve(transport).await.map_err(|e| {
            AppError::Internal(format!("MCP '{}': connection failed: {}", config.name, e))
        })?;

        tracing::info!(server = %config.name, "MCP server connected");
        Ok(Self {
            name: config.name,
            client,
        })
    }

    pub async fn list_tools(&self) -> AppResult<Vec<(String, String, Value)>> {
        let result = self.client.list_tools(None).await.map_err(|e| {
            AppError::Internal(format!("MCP '{}': list_tools failed: {}", self.name, e))
        })?;

        Ok(result
            .tools
            .into_iter()
            .map(|t| {
                let name = t.name.to_string();
                let desc = t.description.as_deref().unwrap_or("").to_string();
                let schema = serde_json::to_value(&t.input_schema).unwrap_or_default();
                (name, desc, schema)
            })
            .collect())
    }

    pub async fn call_tool(&self, tool_name: &str, params: Value) -> AppResult<ToolOutput> {
        let result = self
            .client
            .call_tool(CallToolRequestParams {
                name: tool_name.to_string().into(),
                arguments: params.as_object().cloned(),
                meta: None,
                task: None,
            })
            .await
            .map_err(|e| {
                AppError::Internal(format!(
                    "MCP '{}': call_tool '{}' failed: {}",
                    self.name, tool_name, e
                ))
            })?;

        let content = result
            .content
            .iter()
            .filter_map(|c| serde_json::to_string(c).ok())
            .collect::<Vec<_>>()
            .join("\n");

        Ok(ToolOutput {
            content,
            is_error: result.is_error.unwrap_or(false),
        })
    }
}

// ── McpBridgeManager ──────────────────────────────────────────────

/// 管理多个 MCP server 连接，批量注册 proxy tools 到 ToolRegistry
pub struct McpBridgeManager {
    connections: Vec<Arc<McpServerConnection>>,
}

impl McpBridgeManager {
    /// 从 `data_dir/mcp.toml` 加载配置并连接所有 MCP server
    /// 文件不存在或解析失败时返回空 bridge；单个 server 连接失败不阻断其他
    pub async fn from_file(data_dir: &Path) -> Self {
        let cfg = McpConfig::load_from_dir(data_dir);
        let mut connections = Vec::with_capacity(cfg.servers.len());
        for config in cfg.servers {
            match McpServerConnection::connect(config.clone()).await {
                Ok(conn) => connections.push(Arc::new(conn)),
                Err(e) => {
                    tracing::error!(
                        server = %config.name,
                        error = %e,
                        "failed to connect MCP server"
                    );
                }
            }
        }
        Self { connections }
    }

    /// 将所有 server 的 tools 注册到 ToolRegistry
    pub async fn inject_tools(&self, registry: &mut super::ToolRegistry) -> AppResult<()> {
        for server in &self.connections {
            let tools = match server.list_tools().await {
                Ok(t) => t,
                Err(e) => {
                    tracing::error!(
                        server = %server.name,
                        error = %e,
                        "failed to list MCP tools"
                    );
                    continue;
                }
            };
            for (name, description, schema) in tools {
                tracing::info!(
                    server = %server.name,
                    tool = %name,
                    "registering MCP proxy tool"
                );
                registry.register(Arc::new(McpProxyTool::new(
                    name,
                    description,
                    schema,
                    Arc::clone(server),
                )));
            }
        }
        Ok(())
    }

    pub fn server_count(&self) -> usize {
        self.connections.len()
    }
}
