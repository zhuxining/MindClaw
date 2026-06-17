//! 进程内消息总线：inbound / outbound 双 mpsc 队列。
//!
//! 对齐 nanobot `MessageBus`：渠道入口投递 [`InboundMessage`]，
//! Agent / 调度器产出投递 [`OutboundMessage`]，由 `ChannelManager` 消费出口侧。

use crate::services::core::{InboundMessage, OutboundMessage};
use tokio::sync::{mpsc, Mutex};

/// 进程内消息总线。
///
/// inbound / outbound 各为单消费者通道：启动时由 manager / runtime 分别 `take_*` 取出接收端。
pub struct MessageBus {
    inbound_tx: mpsc::UnboundedSender<InboundMessage>,
    inbound_rx: Mutex<Option<mpsc::UnboundedReceiver<InboundMessage>>>,
    outbound_tx: mpsc::UnboundedSender<OutboundMessage>,
    outbound_rx: Mutex<Option<mpsc::UnboundedReceiver<OutboundMessage>>>,
}

impl MessageBus {
    pub fn new() -> Self {
        let (inbound_tx, inbound_rx) = mpsc::unbounded_channel();
        let (outbound_tx, outbound_rx) = mpsc::unbounded_channel();
        Self {
            inbound_tx,
            inbound_rx: Mutex::new(Some(inbound_rx)),
            outbound_tx,
            outbound_rx: Mutex::new(Some(outbound_rx)),
        }
    }

    /// 投递入口消息（渠道调用）。
    pub fn publish_inbound(&self, msg: InboundMessage) {
        let _ = self.inbound_tx.send(msg);
    }

    /// 投递出口消息（调度器 / agent 调用）。
    pub fn publish_outbound(&self, msg: OutboundMessage) {
        let _ = self.outbound_tx.send(msg);
    }

    /// 入口克隆发送端（渠道运行时持有）。
    #[allow(dead_code)]
    pub fn inbound_sender(&self) -> mpsc::UnboundedSender<InboundMessage> {
        self.inbound_tx.clone()
    }

    /// 取走入口接收端（ChannelRuntime 启动时调用，仅一次）。
    pub async fn take_inbound(&self) -> Option<mpsc::UnboundedReceiver<InboundMessage>> {
        self.inbound_rx.lock().await.take()
    }

    /// 取走出口接收端（ChannelManager 启动时调用，仅一次）。
    pub async fn take_outbound(&self) -> Option<mpsc::UnboundedReceiver<OutboundMessage>> {
        self.outbound_rx.lock().await.take()
    }
}

impl Default for MessageBus {
    fn default() -> Self {
        Self::new()
    }
}
