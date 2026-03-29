pub mod filesystem;
pub mod mcp_client;
pub mod operations;
pub mod shell;
pub mod skills;
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
