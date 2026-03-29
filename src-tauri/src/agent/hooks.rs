//! HookRegistry：事件钩子系统
//!
//! 支持在关键节点插入自定义逻辑：
//! - PreMessage: 收到消息后，处理前
//! - PostMessage: 消息处理后，响应前
//! - PreToolUse: 工具调用前
//! - PostToolUse: 工具调用后

use crate::error::AppError;
use async_trait::async_trait;
use std::sync::Arc;

/// 钩子触发点
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookPoint {
    /// 收到消息后，处理前
    PreMessage,
    /// 消息处理后，响应前
    PostMessage,
    /// 工具调用前
    PreToolUse,
    /// 工具调用后
    PostToolUse,
}

/// 钩子上下文
pub struct HookContext {
    /// 当前消息内容（可被修改）
    pub message_content: String,
    /// 元数据
    pub metadata: std::collections::HashMap<String, String>,
}

impl HookContext {
    /// 创建新的钩子上下文
    pub fn new(message_content: impl Into<String>) -> Self {
        Self {
            message_content: message_content.into(),
            metadata: std::collections::HashMap::new(),
        }
    }
}

/// 钩子 trait
#[async_trait]
pub trait Hook: Send + Sync {
    /// 钩子名称
    fn name(&self) -> &str;
    /// 触发点
    fn hook_point(&self) -> HookPoint;
    /// 执行钩子逻辑
    async fn execute(&self, context: &mut HookContext) -> Result<(), AppError>;
}

/// 钩子注册表
pub struct HookRegistry {
    hooks: Vec<Arc<dyn Hook>>,
}

impl HookRegistry {
    /// 创建新的钩子注册表
    pub fn new() -> Self {
        Self { hooks: Vec::new() }
    }

    /// 注册钩子
    pub fn register(&mut self, hook: Arc<dyn Hook>) {
        tracing::info!(hook_name = %hook.name(), hook_point = ?hook.hook_point(), "hook_registered");
        self.hooks.push(hook);
    }

    /// 执行指定触发点的所有钩子
    pub async fn run_hooks(&self, point: HookPoint, context: &mut HookContext) {
        for hook in &self.hooks {
            if hook.hook_point() == point {
                if let Err(e) = hook.execute(context).await {
                    tracing::error!(
                        hook_name = %hook.name(),
                        hook_point = ?point,
                        error = %e,
                        "hook_execution_failed"
                    );
                }
            }
        }
    }

    /// 获取已注册的钩子数量
    pub fn len(&self) -> usize {
        self.hooks.len()
    }

    /// 是否没有注册任何钩子
    pub fn is_empty(&self) -> bool {
        self.hooks.is_empty()
    }
}

impl Default for HookRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 内置钩子示例
// ============================================================================

/// 日志钩子（示例）
pub struct LoggingHook;

#[async_trait]
impl Hook for LoggingHook {
    fn name(&self) -> &str {
        "logging"
    }

    fn hook_point(&self) -> HookPoint {
        HookPoint::PreMessage
    }

    async fn execute(&self, context: &mut HookContext) -> Result<(), AppError> {
        tracing::info!(
            content_preview = %&context.message_content[..context.message_content.len().min(100)],
            "pre_message_hook"
        );
        Ok(())
    }
}
