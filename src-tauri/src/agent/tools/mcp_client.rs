use super::traits::{Tool, ToolInput, ToolOutput};
use crate::error::AppResult;
use serde_json::{json, Value};

/// MCP Client：接入外部工具服务（Model Context Protocol）
#[allow(dead_code)]
pub struct McpClientTool {
    server_url: String,
}

impl McpClientTool {
    pub fn new(server_url: impl Into<String>) -> Self {
        Self {
            server_url: server_url.into(),
        }
    }
}

#[async_trait::async_trait]
impl Tool for McpClientTool {
    fn name(&self) -> &str {
        "mcp_client"
    }

    fn description(&self) -> &str {
        "Connect to external MCP tool servers and invoke their tools."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "tool_name": { "type": "string" },
                "parameters": { "type": "object" }
            },
            "required": ["tool_name"]
        })
    }

    async fn execute(&self, _input: ToolInput) -> AppResult<ToolOutput> {
        todo!("实现 MCP Client 工具")
    }
}
