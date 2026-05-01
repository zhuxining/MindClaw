//! Agent spawn tools — rig ToolDyn 实现
//!
//! delegate_to_agent / spawn_background_agent

use crate::agent::spawn::AgentSpawnDispatcher;
use rig::tool::ToolDyn;
use serde_json::{json, Value};
use std::pin::Pin;
use std::sync::Arc;

fn parse_task(args: &str) -> (String, Option<String>) {
    let v: Value = serde_json::from_str(args).unwrap_or_default();
    let task = v["task_description"].as_str().unwrap_or("").to_string();
    let label = v["label"].as_str().map(|s| s.to_string());
    (task, label)
}

fn task_schema(label_desc: &str) -> Value {
    json!({"type":"object","properties":{"task_description":{"type":"string","description":"Full description of the task for the sub-agent to complete"},"label":{"type":"string","description":label_desc}},"required":["task_description"]})
}

// ── delegate_to_agent ────────────────────────────────────

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
        let p = task_schema("Optional short label for the delegated task");
        Box::pin(async move {
            rig::completion::ToolDefinition {
                name: "delegate_to_agent".into(),
                description: "Delegate a task to a sub-agent and wait for the result inline."
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
            let (task, label) = parse_task(&args);
            self.dispatcher
                .delegate_to_agent(&task, label.as_deref())
                .await
                .map(|r| r.final_text)
                .map_err(|e| {
                    rig::tool::ToolError::ToolCallError(Box::new(std::io::Error::other(
                        e.to_string(),
                    )))
                })
        })
    }
}

// ── spawn_background_agent ───────────────────────────────

pub struct SpawnBackgroundAgentTool {
    dispatcher: Arc<AgentSpawnDispatcher>,
}

impl SpawnBackgroundAgentTool {
    pub fn new(dispatcher: Arc<AgentSpawnDispatcher>) -> Self {
        Self { dispatcher }
    }
}

impl ToolDyn for SpawnBackgroundAgentTool {
    fn name(&self) -> String {
        "spawn_background_agent".into()
    }
    fn definition(
        &self,
        _: String,
    ) -> Pin<Box<dyn std::future::Future<Output = rig::completion::ToolDefinition> + Send + '_>>
    {
        let p = task_schema("Optional short label for tracking progress");
        Box::pin(async move {
            rig::completion::ToolDefinition {
                name: "spawn_background_agent".into(),
                description: "Spawn a background agent to handle a task asynchronously.".into(),
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
            let (task, label) = parse_task(&args);
            self.dispatcher.spawn_background(&task, label.as_deref()).await
                .map(|task_id| format!("Background agent spawned (task_id: {task_id}). It will report results when complete."))
                .map_err(|e| rig::tool::ToolError::ToolCallError(Box::new(std::io::Error::other(e.to_string()))))
        })
    }
}
