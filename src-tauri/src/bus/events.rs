use serde::{Deserialize, Serialize};

/// 进入 MessageBus 的入站消息（Channel → Agent）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboundMessage {
    pub id: String,
    pub sender: String,
    pub channel: String,
    pub content: String,
    pub timestamp: i64,
}

/// 从 MessageBus 出站的消息（Agent → Channel）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboundMessage {
    pub id: String,
    pub target_sender: String,
    pub target_channel: String,
    pub content: String,
    pub is_streaming: bool,
    pub timestamp: i64,
}

/// MessageBus：双向异步消息队列（Channel ↔ Agent 解耦）
pub struct MessageBus {
    pub inbound_tx: tokio::sync::mpsc::Sender<InboundMessage>,
    pub inbound_rx: tokio::sync::mpsc::Receiver<InboundMessage>,
    pub outbound_tx: tokio::sync::mpsc::Sender<OutboundMessage>,
    pub outbound_rx: tokio::sync::mpsc::Receiver<OutboundMessage>,
}

impl MessageBus {
    pub fn new(capacity: usize) -> Self {
        let (inbound_tx, inbound_rx) = tokio::sync::mpsc::channel(capacity);
        let (outbound_tx, outbound_rx) = tokio::sync::mpsc::channel(capacity);
        Self {
            inbound_tx,
            inbound_rx,
            outbound_tx,
            outbound_rx,
        }
    }
}
