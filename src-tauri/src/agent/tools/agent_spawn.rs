//! Agent spawn tools
//!
//! 将同步子代理委托和后台代理派发收敛到同一文件，
//! 但仍保留两个独立的 tool name 和语义边界。

use crate::agent::spawn::AgentSpawnDispatcher;
use crate::agent::tools::traits::{Tool, ToolInput, ToolOutput};
use crate::error::AppResult;
use serde_json::{json, Value};
use std::sync::Arc;

fn parse_task_input(input: ToolInput) -> (String, Option<String>) {
    let task_description = input
        .parameters
        .get("task_description")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let label = input
        .parameters
        .get("label")
        .and_then(|v| v.as_str())
        .map(str::to_string);

    (task_description, label)
}

fn task_input_schema(label_description: &str) -> Value {
    json!({
        "type": "object",
        "properties": {
            "task_description": {
                "type": "string",
                "description": "Full description of the task for the sub-agent to complete"
            },
            "label": {
                "type": "string",
                "description": label_description
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

#[async_trait::async_trait]
impl Tool for DelegateToAgentTool {
    fn name(&self) -> &str {
        "delegate_to_agent"
    }

    fn description(&self) -> &str {
        "Delegate a task to a sub-agent and wait for the result inline."
    }

    fn input_schema(&self) -> Value {
        task_input_schema("Optional short label for the delegated task")
    }

    async fn execute(&self, input: ToolInput) -> AppResult<ToolOutput> {
        let (task_description, label) = parse_task_input(input);

        match self
            .dispatcher
            .delegate_to_agent(&task_description, label.as_deref())
            .await
        {
            Ok(result) => Ok(ToolOutput::ok(result.final_text)),
            Err(e) => Ok(ToolOutput::err(format!(
                "Failed to delegate to sub-agent: {e}"
            ))),
        }
    }
}

pub struct SpawnBackgroundAgentTool {
    dispatcher: Arc<AgentSpawnDispatcher>,
}

impl SpawnBackgroundAgentTool {
    pub fn new(dispatcher: Arc<AgentSpawnDispatcher>) -> Self {
        Self { dispatcher }
    }
}

#[async_trait::async_trait]
impl Tool for SpawnBackgroundAgentTool {
    fn name(&self) -> &str {
        "spawn_background_agent"
    }

    fn description(&self) -> &str {
        "Spawn a background agent to handle a specific task asynchronously. \
         The agent runs independently and reports results back when complete. \
         Use this for time-consuming or parallel tasks that don't require immediate results."
    }

    fn input_schema(&self) -> Value {
        task_input_schema("Optional short label for tracking progress (auto-generated if omitted)")
    }

    async fn execute(&self, input: ToolInput) -> AppResult<ToolOutput> {
        let (task_description, label) = parse_task_input(input);

        match self
            .dispatcher
            .spawn_background(&task_description, label.as_deref())
            .await
        {
            Ok(task_id) => Ok(ToolOutput::ok(format!(
                "Background agent spawned (task_id: {task_id}). \
                 It is running in the background and will report results when complete."
            ))),
            Err(e) => Ok(ToolOutput::err(format!(
                "Failed to spawn background agent: {e}"
            ))),
        }
    }
}
