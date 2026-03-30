//! AppRuntime：统一运行时，Tauri 无关
//!
//! 三个入口点（Tauri Desktop / CLI / Gateway）共享同一套初始化逻辑和生命周期管理

pub mod builder;
pub mod config;
pub mod services;

use crate::agent::{AgentLoop, SessionManager};
use crate::bus::events::InboundMessage;
use crate::bus::MessageBus;
use crate::error::AppResult;
use crate::models::conversation::ConversationMode;
use rusqlite::Connection;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

pub use builder::AppRuntimeBuilder;
pub use config::AppConfig;
pub use services::ServiceContainer;

/// 应用运行时：拥有所有共享基础设施
pub struct AppRuntime {
    /// SQLite 连接
    pub(crate) db: Arc<Mutex<Connection>>,
    /// 统一服务层
    pub(crate) services: Arc<ServiceContainer>,
    /// 消息总线
    pub(crate) bus: Arc<MessageBus>,
    /// Agent 循环
    pub(crate) agent: Arc<AgentLoop>,
    /// 会话管理器
    pub(crate) session_mgr: Arc<SessionManager>,
    /// 应用配置
    pub(crate) config: Arc<AppConfig>,
    /// 关停信号
    pub(crate) shutdown: CancellationToken,
    /// 后台任务句柄
    pub(crate) tasks: Mutex<Vec<JoinHandle<()>>>,
}

impl AppRuntime {
    /// 创建 Builder
    pub fn builder() -> AppRuntimeBuilder {
        AppRuntimeBuilder::new()
    }

    /// 启动后台任务（AgentLoop 等）
    pub async fn start(&self) -> AppResult<()> {
        let mut tasks = self.tasks.lock().await;

        // 启动 AgentLoop
        let agent = self.agent.clone();
        let cancel = self.shutdown.clone();
        tasks.push(tokio::spawn(async move {
            tokio::select! {
                result = AgentLoop::run(agent) => {
                    if let Err(e) = result {
                        tracing::error!(error = %e, "agent_loop_failed");
                    }
                }
                _ = cancel.cancelled() => {
                    tracing::info!("agent_loop_shutdown");
                }
            }
        }));

        tracing::info!("app_runtime_started");
        Ok(())
    }

    /// 优雅关停：取消所有后台任务并等待完成
    pub async fn shutdown(&self) {
        tracing::info!("app_runtime_shutting_down");
        self.shutdown.cancel();

        let mut tasks = self.tasks.lock().await;
        for handle in tasks.drain(..) {
            let _ = handle.await;
        }
        tracing::info!("app_runtime_shutdown_complete");
    }

    // --- Accessors ---

    pub fn db(&self) -> &Arc<Mutex<Connection>> {
        &self.db
    }

    pub fn services(&self) -> &Arc<ServiceContainer> {
        &self.services
    }

    pub fn bus(&self) -> &Arc<MessageBus> {
        &self.bus
    }

    pub fn agent(&self) -> &Arc<AgentLoop> {
        &self.agent
    }

    pub fn session_mgr(&self) -> &Arc<SessionManager> {
        &self.session_mgr
    }

    pub fn config(&self) -> &Arc<AppConfig> {
        &self.config
    }

    // --- Messaging ---

    /// 向 Agent 发送消息（通用，不限入口点）
    pub async fn send_message(
        &self,
        content: &str,
        sender: &str,
        channel: &str,
        session_id: Option<String>,
    ) -> AppResult<String> {
        let request_id = uuid::Uuid::new_v4().to_string();

        let message = InboundMessage {
            id: uuid::Uuid::new_v4().to_string(),
            request_id: request_id.clone(),
            session_id,
            sender: sender.to_string(),
            channel: channel.to_string(),
            mode: ConversationMode::Chat,
            content: content.to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
        };

        self.bus.publish_inbound(message).await?;
        Ok(request_id)
    }
}
