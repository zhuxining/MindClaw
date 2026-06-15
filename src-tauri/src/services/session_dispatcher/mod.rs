use crate::error::AppError;
use crate::services::acp_client::AcpClient;
use crate::services::agent::{AgentResolver, ConversationKey};
use crate::services::core::{AgentResponse, ChannelMessage};
use crate::services::event_bus::EventBus;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};

mod retry;
mod types;
mod worker;

use types::DispatchCommand;
use worker::session_worker;

pub struct SessionDispatcher {
    resolver: Arc<AgentResolver>,
    acp_client: Arc<AcpClient>,
    event_bus: Arc<EventBus>,
    sessions: Arc<DashMap<ConversationKey, mpsc::UnboundedSender<DispatchCommand>>>,
}

impl SessionDispatcher {
    pub fn new(
        resolver: Arc<AgentResolver>,
        acp_client: Arc<AcpClient>,
        event_bus: Arc<EventBus>,
    ) -> Self {
        Self {
            resolver,
            acp_client,
            event_bus,
            sessions: Arc::new(DashMap::new()),
        }
    }

    /// 按 session 保序分发消息：同一 `channel + conversation_id` 的消息 FIFO 处理，
    /// 不同 session 的消息并发执行。
    pub async fn dispatch(&self, message: ChannelMessage) -> Result<AgentResponse, AppError> {
        let key = ConversationKey {
            channel: message.channel.clone(),
            conversation_id: message.conversation_id.clone(),
        };

        let sessions = self.sessions.clone();
        let sender = self
            .sessions
            .entry(key.clone())
            .or_insert_with(|| {
                let (tx, rx) = mpsc::unbounded_channel();
                let worker_key = key.clone();
                tokio::spawn(session_worker(
                    rx,
                    self.resolver.clone(),
                    self.acp_client.clone(),
                    self.event_bus.clone(),
                    sessions,
                    worker_key,
                ));
                tx
            })
            .clone();

        let (responder, rx) = oneshot::channel();
        sender
            .send(DispatchCommand { message, responder })
            .map_err(|_| AppError::Internal("会话 worker 已关闭".to_string()))?;

        rx.await
            .map_err(|_| AppError::Internal("会话 worker 无响应".to_string()))?
    }
}
