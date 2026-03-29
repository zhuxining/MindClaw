pub mod events;

use crate::error::AppError;
use events::{InboundMessage, OutboundMessage};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use tokio::sync::mpsc;

/// MessageBus：双向异步消息队列（Channel ↔ Agent 解耦）
pub struct MessageBus {
    inbound: mpsc::Sender<InboundMessage>,
    inbound_rx: Mutex<Option<mpsc::Receiver<InboundMessage>>>,
    outbound: mpsc::Sender<OutboundMessage>,
    outbound_rx: Mutex<Option<mpsc::Receiver<OutboundMessage>>>,
    inbound_count: AtomicUsize,
    outbound_count: AtomicUsize,
}

impl MessageBus {
    /// 创建新的 MessageBus
    pub fn new(capacity: usize) -> Self {
        let (inbound, inbound_rx) = mpsc::channel(capacity);
        let (outbound, outbound_rx) = mpsc::channel(capacity);

        Self {
            inbound,
            inbound_rx: Mutex::new(Some(inbound_rx)),
            outbound,
            outbound_rx: Mutex::new(Some(outbound_rx)),
            inbound_count: AtomicUsize::new(0),
            outbound_count: AtomicUsize::new(0),
        }
    }

    /// 发布入站消息
    pub async fn publish_inbound(&self, msg: InboundMessage) -> Result<(), AppError> {
        self.inbound_count.fetch_add(1, Ordering::SeqCst);
        self.inbound
            .send(msg)
            .await
            .map_err(|_| AppError::Internal("inbound channel closed".to_string()))?;
        Ok(())
    }

    /// 发布出站消息
    pub async fn publish_outbound(&self, msg: OutboundMessage) -> Result<(), AppError> {
        self.outbound_count.fetch_add(1, Ordering::SeqCst);
        self.outbound
            .send(msg)
            .await
            .map_err(|_| AppError::Internal("outbound channel closed".to_string()))?;
        Ok(())
    }

    /// 取出入站接收器（仅一次）
    pub async fn take_inbound_rx(&self) -> Result<mpsc::Receiver<InboundMessage>, AppError> {
        self.inbound_rx
            .lock()
            .unwrap()
            .take()
            .ok_or_else(|| AppError::Internal("inbound receiver already taken".to_string()))
    }

    /// 取出出站接收器（仅一次）
    pub async fn take_outbound_rx(&self) -> Result<mpsc::Receiver<OutboundMessage>, AppError> {
        self.outbound_rx
            .lock()
            .unwrap()
            .take()
            .ok_or_else(|| AppError::Internal("outbound receiver already taken".to_string()))
    }

    /// 入站待处理数（粗略估计）
    pub fn inbound_pending(&self) -> usize {
        self.inbound_count.load(Ordering::SeqCst)
    }

    /// 出站待处理数（粗略估计）
    pub fn outbound_pending(&self) -> usize {
        self.outbound_count.load(Ordering::SeqCst)
    }
}

impl Default for MessageBus {
    fn default() -> Self {
        Self::new(100)
    }
}
