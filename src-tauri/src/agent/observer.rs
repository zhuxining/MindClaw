use crate::agent::events::AgentEvent;
use async_trait::async_trait;

/// 内部观测接口（日志/审计/指标）
#[async_trait]
pub trait AgentObserver: Send + Sync {
    /// 处理 Agent 事件
    async fn on_event(&self, event: &AgentEvent);
}

/// MVP 默认实现：tracing 日志
pub struct TracingObserver;

impl TracingObserver {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TracingObserver {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AgentObserver for TracingObserver {
    async fn on_event(&self, event: &AgentEvent) {
        match event {
            AgentEvent::RunStarted {
                session_id,
                request_id,
            } => {
                tracing::info!(
                    session_id = %session_id,
                    request_id = %request_id,
                    "agent_run_started"
                );
            }
            AgentEvent::SessionResolved { session_id } => {
                tracing::debug!(session_id = %session_id, "agent_session_resolved");
            }
            AgentEvent::CommandIntercepted { name } => {
                tracing::info!(command = %name, "agent_command_intercepted");
            }
            AgentEvent::ContextBuilt { fragments } => {
                tracing::debug!(fragments = %fragments, "agent_context_built");
            }
            AgentEvent::ProviderTextDelta { len } => {
                tracing::trace!(len = %len, "agent_provider_text_delta");
            }
            AgentEvent::ProviderToolCall { name } => {
                tracing::info!(tool_name = %name, "agent_provider_tool_call");
            }
            AgentEvent::ToolStarted { name } => {
                tracing::info!(tool_name = %name, "agent_tool_started");
            }
            AgentEvent::ToolFinished { name, success } => {
                tracing::info!(
                    tool_name = %name,
                    success = %success,
                    "agent_tool_finished"
                );
            }
            AgentEvent::RunCompleted => {
                tracing::info!("agent_run_completed");
            }
            AgentEvent::RunCancelled => {
                tracing::info!("agent_run_cancelled");
            }
            AgentEvent::RunFailed { message } => {
                tracing::error!(error = %message, "agent_run_failed");
            }
        }
    }
}

/// 组合多个 Observer（用于同时输出到日志、指标、审计）
pub struct CompositeObserver {
    observers: Vec<Box<dyn AgentObserver>>,
}

impl CompositeObserver {
    pub fn new() -> Self {
        Self {
            observers: Vec::new(),
        }
    }

    pub fn add(&mut self, observer: Box<dyn AgentObserver>) {
        self.observers.push(observer);
    }
}

impl Default for CompositeObserver {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AgentObserver for CompositeObserver {
    async fn on_event(&self, event: &AgentEvent) {
        for observer in &self.observers {
            observer.on_event(event).await;
        }
    }
}
