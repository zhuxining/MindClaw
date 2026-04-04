//! SubAgent 核心类型定义

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// SubAgent 执行模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubAgentMode {
    /// 后台异步，Run 完成后派发，持久化队列
    Background,
    /// 并行执行，Run 内启动，与其他工具调用并发
    Parallel,
}

/// 从 Markdown frontmatter + body 解析的 SubAgent 定义
#[derive(Debug, Clone)]
pub struct SubAgentDef {
    /// 唯一名称
    pub name: String,
    /// 描述（供 LLM 和 operations list 使用）
    pub description: String,
    /// Markdown body = system prompt
    pub system_prompt: String,
    /// 默认执行模式
    pub mode: SubAgentMode,
    /// 首选模型（None 则继承主代理）
    pub model: Option<String>,
    /// 安全边界
    pub capabilities: CapabilityProfile,
    /// 来源路径（None = builtin via include_str!）
    pub source_path: Option<PathBuf>,
}

impl Default for SubAgentDef {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            system_prompt: String::new(),
            mode: SubAgentMode::Background,
            model: None,
            capabilities: CapabilityProfile::default(),
            source_path: None,
        }
    }
}

/// 安全边界
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityProfile {
    pub allowed_tools: Vec<String>,
    pub max_tool_calls: u32,
    pub timeout_ms: u64,
}

impl Default for CapabilityProfile {
    fn default() -> Self {
        Self {
            allowed_tools: vec![
                "read_file".to_string(),
                "write_file".to_string(),
                "list_directory".to_string(),
                "shell".to_string(),
                "web_search".to_string(),
                "web_fetch".to_string(),
            ],
            max_tool_calls: 15,
            timeout_ms: 60_000,
        }
    }
}

/// SubAgent 执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgentResult {
    pub success: bool,
    pub output: serde_json::Value,
    pub logs: Vec<String>,
    pub tool_events: Vec<crate::agent::spec::ToolEvent>,
}

impl SubAgentResult {
    pub fn success(output: serde_json::Value) -> Self {
        Self {
            success: true,
            output,
            logs: Vec::new(),
            tool_events: Vec::new(),
        }
    }

    pub fn failure(message: impl Into<String>) -> Self {
        Self {
            success: false,
            output: serde_json::json!({"error": message.into()}),
            logs: Vec::new(),
            tool_events: Vec::new(),
        }
    }
}

/// SubAgent 摘要信息（供 operations list 返回）
#[derive(Debug, Clone, Serialize)]
pub struct SubAgentInfo {
    pub name: String,
    pub description: String,
    pub mode: SubAgentMode,
    pub model: Option<String>,
    pub builtin: bool,
}

/// 路由上下文（执行时传入）
#[derive(Debug, Clone)]
pub struct RoutingContext {
    pub session_key: String,
    pub channel: String,
}
