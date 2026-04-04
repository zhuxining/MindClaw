//! MCP Bridge：基于 rmcp 的 Model Context Protocol 集成
//!
//! 设计原则：
//! - 延迟连接：首次调用时建立连接
//! - 透明封装：MCP 工具与原生工具无差别
//! - 命名空间隔离：使用 `mcp__tool` 格式避免冲突
//! - 生命周期管理：优雅连接、错误恢复、干净关闭

use crate::agent::tools::traits::{Tool, ToolInput, ToolOutput};
use crate::error::{AppError, AppResult};
use async_trait::async_trait;
use rmcp::{
    model::CallToolRequestParams,
    service::{RoleClient, RunningService},
    transport::streamable_http_client::StreamableHttpClientTransportConfig,
    transport::{StreamableHttpClientTransport, TokioChildProcess},
    ServiceExt,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Command;
use tokio::sync::RwLock;

// ============================================================================
// 配置
// ============================================================================

/// MCP 服务器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPServerConfig {
    /// 服务器名称（用于标识）
    pub name: String,
    /// 传输类型
    #[serde(default)]
    pub transport: MCPTransport,
    /// stdio: 命令
    pub command: Option<String>,
    /// stdio: 参数
    #[serde(default)]
    pub args: Vec<String>,
    /// stdio: 环境变量
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// streamable-http: URL
    pub url: Option<String>,
    /// streamable-http: 认证 Token
    pub auth_token: Option<String>,
    /// streamable-http: 自定义请求头
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// 工具调用超时（秒）
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    /// 工具过滤（空表示全部）
    #[serde(default)]
    pub enabled_tools: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MCPTransport {
    #[default]
    Stdio,
    StreamableHttp,
}

fn default_timeout() -> u64 {
    30
}

/// mcp.toml 顶层结构
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct MCPConfig {
    #[serde(default, rename = "server")]
    pub servers: Vec<MCPServerConfig>,
}

impl MCPConfig {
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

// ============================================================================
// MCP Manager
// ============================================================================

/// MCP 管理器
///
/// 管理多个 MCP server 连接，延迟加载，批量注册 proxy tools 到 ToolRegistry
pub struct MCPManager {
    configs: Vec<MCPServerConfig>,
    /// 已连接的客户端
    clients: RwLock<HashMap<String, Arc<MCPClient>>>,
    /// 连接状态
    connected: std::sync::atomic::AtomicBool,
}

/// MCP 客户端包装
pub struct MCPClient {
    inner: RunningService<RoleClient, ()>,
    config: MCPServerConfig,
}

impl MCPManager {
    /// 创建新的 MCP Manager
    pub fn new(configs: Vec<MCPServerConfig>) -> Self {
        Self {
            configs,
            clients: RwLock::new(HashMap::new()),
            connected: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// 从 `data_dir/mcp.toml` 加载配置
    pub fn from_file(data_dir: &Path) -> Self {
        let cfg = MCPConfig::load_from_dir(data_dir);
        Self::new(cfg.servers)
    }

    /// server 数量
    pub fn server_count(&self) -> usize {
        self.configs.len()
    }

    /// 确保已连接（延迟连接）
    pub async fn ensure_connected(&self) -> AppResult<()> {
        // 已连接则直接返回
        if self.connected.load(std::sync::atomic::Ordering::SeqCst) {
            return Ok(());
        }

        // 连接所有服务器
        for config in &self.configs {
            match self.connect_server(config).await {
                Ok(client) => {
                    let mut clients = self.clients.write().await;
                    clients.insert(config.name.clone(), Arc::new(client));
                }
                Err(e) => {
                    tracing::error!(
                        server = %config.name,
                        error = %e,
                        "failed to connect MCP server"
                    );
                    // 继续连接其他服务器
                }
            }
        }

        self.connected
            .store(true, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    /// 连接单个服务器
    async fn connect_server(&self, config: &MCPServerConfig) -> AppResult<MCPClient> {
        let client = match config.transport {
            MCPTransport::Stdio => self.connect_stdio(config).await?,
            MCPTransport::StreamableHttp => self.connect_streamable_http(config).await?,
        };

        Ok(MCPClient {
            inner: client,
            config: config.clone(),
        })
    }

    /// stdio 连接
    async fn connect_stdio(
        &self,
        config: &MCPServerConfig,
    ) -> AppResult<RunningService<RoleClient, ()>> {
        let cmd = config.command.as_ref().ok_or_else(|| {
            AppError::Validation("Command required for stdio transport".to_string())
        })?;

        let mut command = Command::new(cmd);
        command.args(&config.args);
        command.envs(&config.env);

        let transport = TokioChildProcess::new(command).map_err(|e| {
            AppError::Internal(format!(
                "MCP '{}': failed to start child process: {}",
                config.name, e
            ))
        })?;

        let client = ().serve(transport).await.map_err(|e| {
            AppError::Internal(format!("MCP '{}': connection failed: {}", config.name, e))
        })?;

        tracing::info!(server = %config.name, "MCP server connected (stdio)");
        Ok(client)
    }

    /// streamable-http 连接
    async fn connect_streamable_http(
        &self,
        config: &MCPServerConfig,
    ) -> AppResult<RunningService<RoleClient, ()>> {
        let url = config.url.as_ref().ok_or_else(|| {
            AppError::Validation("URL required for streamable-http transport".to_string())
        })?;

        let mut transport_config = StreamableHttpClientTransportConfig::with_uri(url.clone());

        // 配置认证
        if let Some(token) = &config.auth_token {
            transport_config = transport_config.auth_header(token);
        }

        // 配置自定义请求头
        if !config.headers.is_empty() {
            let mut custom_headers = std::collections::HashMap::new();
            for (key, value) in &config.headers {
                if let (Ok(header_name), Ok(header_value)) = (
                    key.parse::<http::HeaderName>(),
                    value.parse::<http::HeaderValue>(),
                ) {
                    custom_headers.insert(header_name, header_value);
                } else {
                    tracing::warn!(key = %key, value = %value, "invalid header format, skipping");
                }
            }
            if !custom_headers.is_empty() {
                transport_config = transport_config.custom_headers(custom_headers);
            }
        }

        // 创建 reqwest client
        let http_client = reqwest::Client::new();
        let transport = StreamableHttpClientTransport::with_client(http_client, transport_config);

        let client = ().serve(transport).await.map_err(|e| {
            AppError::Internal(format!("MCP '{}': connection failed: {}", config.name, e))
        })?;

        tracing::info!(server = %config.name, url = %url, "MCP server connected (streamable-http)");
        Ok(client)
    }

    /// 获取所有 MCP 工具
    pub async fn get_tools(&self) -> Vec<MCPTool> {
        let mut tools = Vec::new();
        let clients = self.clients.read().await;

        for (server_name, client) in clients.iter() {
            match client.inner.list_all_tools().await {
                Ok(mcp_tools) => {
                    for mcp_tool in mcp_tools {
                        // 工具过滤
                        if !self.is_tool_enabled(&client.config, &mcp_tool.name) {
                            continue;
                        }

                        let tool = MCPTool::new(server_name.clone(), mcp_tool, Arc::clone(client));
                        tools.push(tool);
                    }
                }
                Err(e) => {
                    tracing::error!(server = %server_name, error = %e, "failed to list MCP tools");
                }
            }
        }

        tools
    }

    /// 检查工具是否启用
    fn is_tool_enabled(&self, config: &MCPServerConfig, tool_name: &str) -> bool {
        if config.enabled_tools.is_empty() {
            return true;
        }

        let prefixed_name = format!("mcp__{}", tool_name);
        config.enabled_tools.contains(&tool_name.to_string())
            || config.enabled_tools.contains(&prefixed_name)
    }

    /// 关闭所有连接
    pub async fn close_all(&self) {
        let mut clients = self.clients.write().await;
        for (name, _client) in clients.drain() {
            tracing::info!(server = %name, "MCP server connection closed");
        }
        self.connected
            .store(false, std::sync::atomic::Ordering::SeqCst);
    }
}

// ============================================================================
// MCP Tool 封装
// ============================================================================

/// MCP 工具包装
pub struct MCPTool {
    /// 服务器名称
    #[allow(dead_code)]
    server_name: String,
    /// MCP 原始名称
    original_name: String,
    /// 封装后的名称（mcp__tool）
    prefixed_name: String,
    /// 描述
    description: String,
    /// 参数 Schema
    schema: Value,
    /// MCP 客户端
    client: Arc<MCPClient>,
    /// 超时
    timeout: Duration,
}

impl MCPTool {
    pub fn new(server_name: String, mcp_tool: rmcp::model::Tool, client: Arc<MCPClient>) -> Self {
        let original_name = mcp_tool.name.to_string();
        let prefixed_name = format!("mcp__{}", original_name);
        let timeout = Duration::from_secs(client.config.timeout_secs);

        Self {
            server_name,
            original_name,
            prefixed_name,
            description: mcp_tool.description.unwrap_or_default().to_string(),
            schema: normalize_schema(&mcp_tool.input_schema),
            client,
            timeout,
        }
    }
}

#[async_trait]
impl Tool for MCPTool {
    fn name(&self) -> &str {
        &self.prefixed_name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn input_schema(&self) -> Value {
        self.schema.clone()
    }

    async fn execute(&self, input: ToolInput) -> AppResult<ToolOutput> {
        let args = input
            .parameters
            .as_object()
            .map(|m| serde_json::Map::from_iter(m.iter().map(|(k, v)| (k.clone(), v.clone()))))
            .unwrap_or_default();

        let request = CallToolRequestParams {
            name: self.original_name.clone().into(),
            arguments: Some(args),
            meta: None,
            task: None,
        };

        // 带超时执行
        let result =
            match tokio::time::timeout(self.timeout, self.client.inner.call_tool(request)).await {
                Ok(Ok(result)) => result,
                Ok(Err(e)) => {
                    return Ok(ToolOutput {
                        content: format!("MCP error: {}", e),
                        is_error: true,
                    });
                }
                Err(_) => {
                    return Ok(ToolOutput {
                        content: format!("Timeout after {:?}", self.timeout),
                        is_error: true,
                    });
                }
            };

        // 转换结果为字符串
        let content = format_mcp_result(&result);
        Ok(ToolOutput {
            content,
            is_error: false,
        })
    }
}

/// 格式化 MCP 结果为字符串
fn format_mcp_result(result: &rmcp::model::CallToolResult) -> String {
    use rmcp::model::RawContent;

    let mut outputs = Vec::new();

    for annotated_content in &result.content {
        // annotated_content 是 Annotated<RawContent> 类型
        let text = match &annotated_content.raw {
            RawContent::Text(t) => t.text.clone(),
            RawContent::Image(img) => {
                format!("![image](data:{};base64,{})", img.mime_type, img.data)
            }
            RawContent::Resource(r) => {
                // 从资源中提取文本
                serde_json::to_string(r).unwrap_or_default()
            }
            RawContent::Audio(a) => {
                format!("[Audio: {}]", a.mime_type)
            }
            RawContent::ResourceLink(r) => {
                format!("[Resource: {} - {}]", r.uri, r.name)
            }
        };
        outputs.push(text);
    }

    if outputs.is_empty() {
        "(no output)".to_string()
    } else {
        outputs.join("\n\n")
    }
}

/// Schema 标准化
fn normalize_schema(schema: &std::sync::Arc<serde_json::Map<String, Value>>) -> Value {
    Value::Object(schema.as_ref().clone())
}

// ============================================================================
// 与 ToolRegistry 集成
// ============================================================================

/// 扩展 ToolRegistry 以支持 MCP
pub async fn register_mcp_tools(
    registry: &mut super::ToolRegistry,
    mcp_manager: &MCPManager,
) -> AppResult<()> {
    // 确保已连接
    mcp_manager.ensure_connected().await?;

    // 获取所有 MCP 工具
    let tools = mcp_manager.get_tools().await;

    // 注册到 registry
    for tool in tools {
        tracing::info!(tool_name = %tool.name(), "registering MCP tool");
        registry.register(Arc::new(tool));
    }

    Ok(())
}
