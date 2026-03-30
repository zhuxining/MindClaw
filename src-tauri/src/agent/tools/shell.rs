use super::traits::{Tool, ToolInput, ToolOutput};
use crate::error::AppResult;
use serde_json::{json, Value};

/// 白名单受限 Shell（沙箱执行，只允许预定义命令集）
#[allow(dead_code)]
const ALLOWED_COMMANDS: &[&str] = &["echo", "date", "pwd"];

pub struct ShellTool;

#[async_trait::async_trait]
impl Tool for ShellTool {
    fn name(&self) -> &str {
        "shell"
    }

    fn description(&self) -> &str {
        "Execute whitelisted shell commands in a sandboxed environment."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": { "type": "string" },
                "args": { "type": "array", "items": { "type": "string" } }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, _input: ToolInput) -> AppResult<ToolOutput> {
        todo!("实现白名单 Shell 工具")
    }
}
