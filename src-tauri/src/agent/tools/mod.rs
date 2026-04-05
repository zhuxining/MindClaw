//! Tools runtime support
//!
//! 当前 `tools/` 是 Runtime 中唯一保留目录的模块，原因有两点：
//! - 具体工具实现数量已经明显多于其他 agent 子模块
//! - MCP bridge 虽然语义上属于 external tool bridge，但最终仍注册为 `Tool`
//!
//! 当前目录内按语义大致分为三类：
//! - builtin tools：`read_file`、`write_file`、`edit_file`、`find_files`、`search_content`、`shell`
//! - orchestration tools：`delegate_to_agent`、`spawn_background_agent`
//! - external tool bridge：`mcp__*`
//!
//! 未来如果三类工具继续增长，再考虑演进为 `builtin/`、`orchestration/`、`bridge/`
//! 等子目录；在此之前保持扁平目录更有利于控制重构成本。

pub mod agent_spawn;
pub mod file_ops;
pub mod find_files;
pub mod mcp;
pub mod path_guard;
pub mod search_content;
pub mod shell;
pub mod traits;

use crate::error::{AppError, AppResult};
use crate::providers::ToolSchema;
use futures_util::future::join_all;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;
use traits::{Tool, ToolInput, ToolOutput};

/// 工具调用请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// 工具调用 ID（由 Provider 生成）
    pub id: String,
    /// 工具名称
    pub name: String,
    /// 工具参数（JSON 对象）
    pub arguments: Value,
}

/// 工具结果消息
#[derive(Debug, Clone)]
pub struct ToolResultMessage {
    /// 对应的工具调用 ID
    pub tool_call_id: String,
    /// 结果内容
    pub content: String,
    /// 是否出错
    pub is_error: bool,
}

/// 工具注册表：注册、查找、执行工具
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool + Send + Sync>>,
    concurrency: Arc<Semaphore>,
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolRegistry {
    /// 创建新的 ToolRegistry
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            concurrency: Arc::new(Semaphore::new(4)),
        }
    }

    /// 设置并发限制
    pub fn with_concurrency_limit(mut self, limit: usize) -> Self {
        self.concurrency = Arc::new(Semaphore::new(limit.max(1)));
        self
    }

    /// 注册工具
    pub fn register(&mut self, tool: Arc<dyn Tool + Send + Sync>) {
        tracing::info!(tool_name = %tool.name(), "tool_registered");
        self.tools.insert(tool.name().to_string(), tool);
    }

    /// 获取工具
    pub fn get(&self, name: &str) -> Option<&(dyn Tool + Send + Sync)> {
        self.tools.get(name).map(|tool| tool.as_ref())
    }

    /// 获取所有工具的 JSON Schema 定义（供 Provider 使用）
    pub fn schemas(&self) -> Vec<ToolSchema> {
        self.tools
            .values()
            .map(|tool| ToolSchema {
                name: tool.name().to_string(),
                description: tool.description().to_string(),
                parameters: tool.input_schema(),
            })
            .collect()
    }

    /// 并行执行多个工具调用
    pub async fn execute_calls(
        &self,
        calls: Vec<ToolCall>,
        cancel: CancellationToken,
    ) -> AppResult<Vec<ToolResultMessage>> {
        let tasks = calls.into_iter().map(|call| {
            let tool = self.tools.get(&call.name).cloned();
            let semaphore = Arc::clone(&self.concurrency);
            let cancel = cancel.clone();

            tokio::spawn(async move {
                let tool_call_id = call.id.clone();

                let Some(tool) = tool else {
                    return ToolResultMessage {
                        tool_call_id,
                        content: format!("Tool not found: {}", call.name),
                        is_error: true,
                    };
                };

                let permit = match semaphore.acquire_owned().await {
                    Ok(permit) => permit,
                    Err(_) => {
                        return ToolResultMessage {
                            tool_call_id,
                            content: "Tool concurrency limiter closed".to_string(),
                            is_error: true,
                        };
                    }
                };

                if cancel.is_cancelled() {
                    drop(permit);
                    return ToolResultMessage {
                        tool_call_id,
                        content: "Tool execution cancelled".to_string(),
                        is_error: true,
                    };
                }

                let result = tool
                    .execute(ToolInput {
                        parameters: call.arguments,
                    })
                    .await;

                drop(permit);

                match result {
                    Ok(output) => ToolResultMessage {
                        tool_call_id,
                        content: output.content,
                        is_error: output.is_error,
                    },
                    Err(error) => ToolResultMessage {
                        tool_call_id,
                        content: format!("Error: {error}"),
                        is_error: true,
                    },
                }
            })
        });

        Ok(join_all(tasks)
            .await
            .into_iter()
            .filter_map(|result| result.ok())
            .collect())
    }

    /// 执行单个工具
    pub async fn execute(&self, name: &str, input: ToolInput) -> AppResult<ToolOutput> {
        let tool = self
            .tools
            .get(name)
            .ok_or_else(|| AppError::NotFound(format!("tool: {name}")))?;
        tool.execute(input).await
    }

    /// 获取已注册的工具数量
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// 是否没有注册任何工具
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    /// 获取所有工具名称
    pub fn tool_names(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }
}

// ============================================================================
// 便捷构建方法
// ============================================================================

impl ToolRegistry {
    /// 初始化默认工具集
    ///
    /// 包含以下内置工具：
    /// - ShellTool - Shell 命令执行
    /// - FileReadTool - 读取文件
    /// - FileWriteTool - 写入文件
    /// - FileEditTool - 编辑文件（内容匹配替换）
    /// - FindFilesTool - 文件搜索
    /// - SearchContentTool - 内容搜索
    /// - MCP - MCP 桥接（如果 mcp.toml 存在）
    pub async fn init_default(
        config: &crate::runtime::config::AppConfig,
        extra: Vec<Arc<dyn Tool + Send + Sync>>,
    ) -> AppResult<Arc<Self>> {
        use crate::agent::tools::file_ops::{FileEditTool, FileReadTool, FileWriteTool};
        use crate::agent::tools::find_files::FindFilesTool;
        use crate::agent::tools::mcp::{register_mcp_tools, MCPManager};
        use crate::agent::tools::path_guard::PathGuard;
        use crate::agent::tools::search_content::SearchContentTool;
        use crate::agent::tools::shell::ShellTool;

        let mut tools = Self::new();

        // 内置工具
        let guard =
            Arc::new(PathGuard::vault_only(config.vault_path.clone()).with_denied("private"));
        tools.register(Arc::new(ShellTool::new(config.data_dir.clone())));
        tools.register(Arc::new(FileReadTool::with_guard(Arc::clone(&guard))));
        tools.register(Arc::new(FileWriteTool::with_guard(Arc::clone(&guard))));
        tools.register(Arc::new(FileEditTool::with_guard(Arc::clone(&guard))));
        tools.register(Arc::new(FindFilesTool::new(Arc::clone(&guard))));
        tools.register(Arc::new(SearchContentTool::new(Arc::clone(&guard))));

        // MCP proxy tools：从 data_dir/mcp.toml 加载外部 server 配置
        let mcp_manager = MCPManager::from_file(&config.data_dir);
        if mcp_manager.server_count() > 0 {
            register_mcp_tools(&mut tools, &mcp_manager).await?;
            tracing::info!(servers = %mcp_manager.server_count(), "MCP bridge initialized");
        }

        // 额外工具
        for tool in extra {
            tools.register(tool);
        }

        tracing::info!(tool_count = %tools.len(), "tools_initialized");
        Ok(Arc::new(tools))
    }
}
