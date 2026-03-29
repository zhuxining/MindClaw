pub mod filesystem;
pub mod mcp_client;
pub mod operations;
pub mod shell;
pub mod traits;

use crate::error::AppResult;
use std::collections::HashMap;
use traits::{Tool, ToolInput, ToolOutput};

/// 工具注册表：注册、查找、执行工具
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool + Send + Sync>>,
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn register(&mut self, tool: Box<dyn Tool + Send + Sync>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    pub fn get(&self, name: &str) -> Option<&(dyn Tool + Send + Sync)> {
        self.tools.get(name).map(|t| t.as_ref())
    }

    pub async fn execute(&self, name: &str, input: ToolInput) -> AppResult<ToolOutput> {
        let tool = self
            .tools
            .get(name)
            .ok_or_else(|| crate::error::AppError::NotFound(format!("tool: {}", name)))?;
        tool.execute(input).await
    }
}
