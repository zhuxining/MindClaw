//! SubAgent：多 Agent 编排
//!
//! 统一 Markdown 定义，两种执行模式：
//! - Background：后台异步，Run 完成后派发
//! - Parallel：并行执行，Run 内启动

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

// ============================================================================
// 核心类型
// ============================================================================

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

/// 安全边界
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityProfile {
    pub allowed_tools: Vec<String>,
    pub max_tool_calls: u32,
    pub timeout_ms: u64,
}

/// SubAgent 运行时上下文（执行时传入，与定义解耦）
pub struct SubAgentContext {
    pub task_id: String,
    pub session_id: String,
    pub parameters: serde_json::Value,
    pub cancel: tokio_util::sync::CancellationToken,
}

/// SubAgent 执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgentResult {
    pub success: bool,
    pub output: serde_json::Value,
    pub logs: Vec<String>,
}

impl SubAgentResult {
    pub fn success(output: serde_json::Value) -> Self {
        Self {
            success: true,
            output,
            logs: Vec::new(),
        }
    }

    pub fn failure(message: impl Into<String>) -> Self {
        Self {
            success: false,
            output: serde_json::json!({"error": message.into()}),
            logs: Vec::new(),
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

// ============================================================================
// SubAgentRegistry
// ============================================================================

/// 统一注册表，管理所有 Markdown 定义的 SubAgent
pub struct SubAgentRegistry {
    agents: HashMap<String, Arc<SubAgentDef>>,
}

impl SubAgentRegistry {
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
        }
    }

    /// 注册 SubAgent 定义
    pub fn register(&mut self, def: Arc<SubAgentDef>) {
        tracing::info!(
            name = %def.name,
            mode = ?def.mode,
            builtin = def.source_path.is_none(),
            "sub_agent_registered"
        );
        self.agents.insert(def.name.clone(), def);
    }

    /// 按名称查找
    pub fn get(&self, name: &str) -> Option<&Arc<SubAgentDef>> {
        self.agents.get(name)
    }

    /// 列出所有可用 SubAgent
    pub fn list(&self) -> Vec<SubAgentInfo> {
        self.agents
            .values()
            .map(|a| SubAgentInfo {
                name: a.name.clone(),
                description: a.description.clone(),
                mode: a.mode,
                model: a.model.clone(),
                builtin: a.source_path.is_none(),
            })
            .collect()
    }

    pub fn len(&self) -> usize {
        self.agents.len()
    }

    pub fn is_empty(&self) -> bool {
        self.agents.is_empty()
    }
}

impl Default for SubAgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}
