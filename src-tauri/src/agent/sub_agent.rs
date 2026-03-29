//! SubAgentRegistry：子代理注册表
//!
//! 子代理是异步后台任务，用于执行：
//! - 知识库索引更新
//! - 历史数据归档
//! - 长时间运行的分析任务

use crate::error::AppError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// 子代理任务定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgentTask {
    /// 任务 ID
    pub id: String,
    /// 任务类型
    pub task_type: String,
    /// 任务参数
    pub parameters: serde_json::Value,
    /// 优先级（数值越小优先级越高）
    pub priority: i32,
}

impl SubAgentTask {
    /// 创建新的子代理任务
    pub fn new(task_type: impl Into<String>, parameters: serde_json::Value) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            task_type: task_type.into(),
            parameters,
            priority: 0,
        }
    }

    /// 设置优先级
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }
}

/// 子代理执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgentResult {
    /// 是否成功
    pub success: bool,
    /// 输出数据
    pub output: serde_json::Value,
    /// 执行日志
    pub logs: Vec<String>,
}

impl SubAgentResult {
    /// 创建成功的结果
    pub fn success(output: serde_json::Value) -> Self {
        Self {
            success: true,
            output,
            logs: Vec::new(),
        }
    }

    /// 创建失败的结果
    pub fn failure(message: impl Into<String>) -> Self {
        Self {
            success: false,
            output: serde_json::json!({"error": message.into()}),
            logs: Vec::new(),
        }
    }

    /// 添加日志
    pub fn log(&mut self, message: impl Into<String>) {
        self.logs.push(message.into());
    }
}

/// 子代理执行器 trait
#[async_trait]
pub trait SubAgentExecutor: Send + Sync {
    /// 任务类型
    fn task_type(&self) -> &str;
    /// 执行任务
    async fn execute(&self, task: &SubAgentTask) -> Result<SubAgentResult, AppError>;
}

/// 子代理注册表
pub struct SubAgentRegistry {
    executors: Vec<Box<dyn SubAgentExecutor>>,
}

impl SubAgentRegistry {
    /// 创建新的子代理注册表
    pub fn new() -> Self {
        Self {
            executors: Vec::new(),
        }
    }

    /// 注册执行器
    pub fn register(&mut self, executor: Box<dyn SubAgentExecutor>) {
        tracing::info!(
            task_type = %executor.task_type(),
            "sub_agent_executor_registered"
        );
        self.executors.push(executor);
    }

    /// 派发任务（异步，不阻塞调用者）
    pub async fn dispatch(&self, task: SubAgentTask) -> Result<SubAgentResult, AppError> {
        for executor in &self.executors {
            if executor.task_type() == task.task_type {
                tracing::info!(
                    task_id = %task.id,
                    task_type = %task.task_type,
                    "sub_agent_task_dispatched"
                );
                return executor.execute(&task).await;
            }
        }

        tracing::warn!(
            task_id = %task.id,
            task_type = %task.task_type,
            "no_sub_agent_executor_found"
        );
        Ok(SubAgentResult::failure(format!(
            "No executor found for task type: {}",
            task.task_type
        )))
    }

    /// 获取已注册的执行器数量
    pub fn len(&self) -> usize {
        self.executors.len()
    }

    /// 是否没有注册任何执行器
    pub fn is_empty(&self) -> bool {
        self.executors.is_empty()
    }

    /// 获取所有支持的任务类型
    pub fn supported_types(&self) -> Vec<String> {
        self.executors
            .iter()
            .map(|e| e.task_type().to_string())
            .collect()
    }
}

impl Default for SubAgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 内置执行器示例
// ============================================================================

/// 知识库索引更新任务
pub struct KnowledgeIndexExecutor;

#[async_trait]
impl SubAgentExecutor for KnowledgeIndexExecutor {
    fn task_type(&self) -> &str {
        "knowledge_index"
    }

    async fn execute(&self, task: &SubAgentTask) -> Result<SubAgentResult, AppError> {
        tracing::info!(task_id = %task.id, "knowledge_index_task_started");

        // TODO: 实现知识库索引更新逻辑
        let mut result = SubAgentResult::success(serde_json::json!({
            "indexed_count": 0,
            "duration_ms": 0,
        }));
        result.log("Knowledge index task completed");

        Ok(result)
    }
}

/// 历史数据归档任务
pub struct ArchiveExecutor;

#[async_trait]
impl SubAgentExecutor for ArchiveExecutor {
    fn task_type(&self) -> &str {
        "archive"
    }

    async fn execute(&self, task: &SubAgentTask) -> Result<SubAgentResult, AppError> {
        tracing::info!(task_id = %task.id, "archive_task_started");

        // TODO: 实现归档逻辑
        let mut result = SubAgentResult::success(serde_json::json!({
            "archived_count": 0,
        }));
        result.log("Archive task completed");

        Ok(result)
    }
}
