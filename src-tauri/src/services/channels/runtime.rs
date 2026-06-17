//! 渠道运行时入口循环：消费 `MessageBus.inbound`，归一化为 `ChannelMessage`
//! 经 ingress 去重后交 `SessionDispatcher`，并将 Agent 回复以 `OutboundMessage`
//! 投递回 `MessageBus`（支持流式的渠道做分段模拟）。

use crate::config::AppConfig;
use crate::error::AppError;
use crate::services::channels::manager::ChannelManager;
use crate::services::channels::MessageBus;
use crate::services::core::{
    AgentResponse, ChannelMessage, InboundMessage, OutboundKind, OutboundMessage, ResponseStatus,
};
use crate::services::event_bus::{EventBus, RuntimeEvent};
use crate::services::session_dispatcher::SessionDispatcher;
use crate::storage::MessageStore;
use std::sync::{Arc, Mutex};
use tokio_util::sync::CancellationToken;

/// 流式分段大小（字符），用于把最终输出模拟成增量 delta。
const STREAM_CHUNK_SIZE: usize = 480;

pub struct ChannelRuntime {
    bus: Arc<MessageBus>,
    dispatcher: Arc<SessionDispatcher>,
    messages: Arc<MessageStore>,
    manager: Arc<ChannelManager>,
    event_bus: Arc<EventBus>,
    config: Arc<Mutex<AppConfig>>,
}

impl ChannelRuntime {
    pub fn new(
        bus: Arc<MessageBus>,
        dispatcher: Arc<SessionDispatcher>,
        messages: Arc<MessageStore>,
        manager: Arc<ChannelManager>,
        event_bus: Arc<EventBus>,
        config: Arc<Mutex<AppConfig>>,
    ) -> Self {
        Self {
            bus,
            dispatcher,
            messages,
            manager,
            event_bus,
            config,
        }
    }

    /// 启动入口循环（spawn 一个长任务）。
    pub fn spawn(self: Arc<Self>, cancel: CancellationToken) {
        tokio::spawn(async move {
            let mut rx = match self.bus.take_inbound().await {
                Some(rx) => rx,
                None => return,
            };
            while let Some(msg) = tokio::select! {
                _ = cancel.cancelled() => None,
                m = rx.recv() => m,
            } {
                if let Err(e) = self.handle_inbound(msg).await {
                    eprintln!("channel runtime inbound error: {e}");
                }
            }
        });
    }

    async fn handle_inbound(&self, inbound: InboundMessage) -> Result<(), AppError> {
        eprintln!(
            "[runtime] inbound: channel={}, chat_id={}, content={:.80}",
            inbound.channel, inbound.chat_id, inbound.content
        );

        let message = self.normalize(&inbound);

        // ingress 去重（独立于 dispatch 去重）：仅在此处用一次
        let messages = self.messages.clone();
        let message_id = message.message_id.clone();
        let is_new = tokio::task::spawn_blocking(move || messages.check_and_mark_seen(&message_id))
            .await
            .unwrap_or(false);
        if !is_new {
            self.event_bus.publish(RuntimeEvent::MessageDeduplicated {
                message_id: message.message_id.clone(),
            });
            return Ok(());
        }

        self.messages.save_message(message.clone());
        self.event_bus.publish(RuntimeEvent::MessageReceived {
            message_id: message.message_id.clone(),
            channel: message.channel.clone(),
            conversation_id: message.conversation_id.clone(),
        });

        // 分发到 SessionDispatcher（per-session FIFO）
        let response = self.dispatcher.dispatch(message.clone()).await?;

        // auto_reply：将回复投递回渠道
        let auto_reply = self
            .config
            .lock()
            .unwrap()
            .get_channel_config(&message.channel)
            .auto_reply;
        if auto_reply && response.status == ResponseStatus::Success && !response.output.is_empty() {
            self.publish_reply(&message, &response).await;
        }
        Ok(())
    }

    /// 把回复投递回 bus：流式渠道做分段模拟，否则整段 Final。
    async fn publish_reply(&self, message: &ChannelMessage, response: &AgentResponse) {
        let supports_streaming = self
            .manager
            .channels()
            .lock()
            .await
            .get(message.channel.as_str())
            .map(|c| c.supports_streaming())
            .unwrap_or(false);

        if supports_streaming {
            let stream_id = response.request_id.clone();
            for chunk in split_chunks(&response.output, STREAM_CHUNK_SIZE) {
                self.bus.publish_outbound(OutboundMessage {
                    channel: message.channel.clone(),
                    chat_id: message.conversation_id.clone(),
                    content: chunk,
                    reply_to: Some(message.message_id.clone()),
                    media: Vec::new(),
                    kind: OutboundKind::StreamDelta {
                        stream_id: stream_id.clone(),
                        end: false,
                    },
                });
            }
            self.bus.publish_outbound(OutboundMessage {
                channel: message.channel.clone(),
                chat_id: message.conversation_id.clone(),
                content: String::new(),
                reply_to: Some(message.message_id.clone()),
                media: Vec::new(),
                kind: OutboundKind::StreamDelta {
                    stream_id,
                    end: true,
                },
            });
        } else {
            self.bus.publish_outbound(OutboundMessage {
                channel: message.channel.clone(),
                chat_id: message.conversation_id.clone(),
                content: response.output.clone(),
                reply_to: Some(message.message_id.clone()),
                media: Vec::new(),
                kind: OutboundKind::Final,
            });
        }
    }

    /// InboundMessage → ChannelMessage（从 metadata 提取 message_id / sender_name / timestamp）。
    fn normalize(&self, inbound: &InboundMessage) -> ChannelMessage {
        let meta = inbound.metadata.as_object();
        let message_id = meta
            .and_then(|m| m.get("message_id"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let sender_name = meta
            .and_then(|m| m.get("sender_name"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let timestamp = meta
            .and_then(|m| m.get("timestamp"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let reply_to = meta
            .and_then(|m| m.get("reply_to"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        ChannelMessage {
            message_id,
            channel: inbound.channel.clone(),
            conversation_id: inbound.chat_id.clone(),
            sender_id: inbound.sender_id.clone(),
            sender_name,
            content: inbound.content.clone(),
            timestamp,
            is_reply: false,
            reply_to,
        }
    }
}

/// 按字符边界切分，避免截断多字节字符。
fn split_chunks(text: &str, size: usize) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }
    text.chars()
        .collect::<Vec<_>>()
        .chunks(size)
        .map(|c| c.iter().collect())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_chunks_respects_char_boundary() {
        let text = "你好世界abcdefghij"; // 4 multibyte + 10 ascii
        let chunks = split_chunks(text, 3);
        assert_eq!(chunks.concat(), text);
        assert!(chunks.iter().all(|c| c.chars().count() <= 3));
    }

    #[test]
    fn split_chunks_empty_returns_empty() {
        assert!(split_chunks("", 10).is_empty());
    }
}
