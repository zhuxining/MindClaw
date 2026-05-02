//! Agent spawn tools — rig ToolDyn 实现

use crate::agent::subagent::AgentSpawnDispatcher;
use rig::tool::ToolDyn;
use serde_json::{json, Value};
use std::pin::Pin;
use std::sync::Arc;

struct DelegateArgs {
    task: String,
    label: Option<String>,
    background: bool,
}

fn parse_delegate_args(args: &str) -> DelegateArgs {
    let v: Value = serde_json::from_str(args).unwrap_or_default();
    DelegateArgs {
        task: v["task_description"].as_str().unwrap_or("").to_string(),
        label: v["label"].as_str().map(str::to_string),
        background: v["background"].as_bool().unwrap_or(false),
    }
}

fn delegate_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "task_description": {
                "type": "string",
                "description": "Full description of the task for the sub-agent to complete"
            },
            "label": {
                "type": "string",
                "description": "Optional short label for the delegated task"
            },
            "background": {
                "type": "boolean",
                "description": "When true, run the delegated task asynchronously and return a task id."
            }
        },
        "required": ["task_description"]
    })
}

pub struct DelegateToAgentTool {
    dispatcher: Arc<AgentSpawnDispatcher>,
}

impl DelegateToAgentTool {
    pub fn new(dispatcher: Arc<AgentSpawnDispatcher>) -> Self {
        Self { dispatcher }
    }
}

impl ToolDyn for DelegateToAgentTool {
    fn name(&self) -> String {
        "delegate_to_agent".into()
    }

    fn definition(
        &self,
        _: String,
    ) -> Pin<Box<dyn std::future::Future<Output = rig::completion::ToolDefinition> + Send + '_>>
    {
        let p = delegate_schema();
        Box::pin(async move {
            rig::completion::ToolDefinition {
                name: "delegate_to_agent".into(),
                description:
                    "Delegate a task to a sub-agent. Set background=true to run asynchronously."
                        .into(),
                parameters: p,
            }
        })
    }

    fn call(
        &self,
        args: String,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<String, rig::tool::ToolError>> + Send + '_>>
    {
        Box::pin(async move {
            let parsed = parse_delegate_args(&args);
            let result = if parsed.background {
                self.dispatcher
                    .spawn_background(&parsed.task, parsed.label.as_deref())
                    .await
            } else {
                self.dispatcher
                    .delegate_inline(&parsed.task)
                    .await
                    .map(|r| r.final_text)
            };

            result.map_err(|e| {
                rig::tool::ToolError::ToolCallError(Box::new(std::io::Error::other(e.to_string())))
            })
        })
    }
}
