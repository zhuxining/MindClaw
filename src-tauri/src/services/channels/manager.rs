//! 渠道管理器：生命周期编排 + outbound 路由 / 重试 / 流式 coalescing。
//!
//! 对齐 nanobot `ChannelManager`：spawn 一个 `dispatch_outbound` 任务消费
//! `MessageBus.outbound`，按 `msg.channel` 路由到对应渠道的 send 原语；
//! 为每个 enabled 渠道 spawn `channel.start`。

use crate::services::channels::{Channel, MessageBus};
use crate::services::core::{OutboundKind, OutboundMessage};
use crate::services::event_bus::{EventBus, RuntimeEvent};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

/// 出口发送重试退避（秒），对齐 nanobot `_SEND_RETRY_DELAYS = (1, 2, 4)`。
const SEND_RETRY_DELAYS: [u64; 3] = [1, 2, 4];
/// 流式 coalescing：合并同 (channel, chat_id, stream_id) 连续 delta 后再发送。
const COALESCE_BATCH_MAX: usize = 64;

#[derive(Debug, Clone, serde::Serialize)]
pub struct ChannelStatus {
    pub channel: String,
    pub running: bool,
    pub last_error: Option<String>,
}

/// 渠道实例表，dispatch_outbound 任务持有。
type ChannelMap = Arc<Mutex<HashMap<String, Arc<dyn Channel>>>>;

pub struct ChannelManager {
    bus: Arc<MessageBus>,
    event_bus: Arc<EventBus>,
    channels: ChannelMap,
    handles: Mutex<Vec<JoinHandle<()>>>,
    cancel: CancellationToken,
}

impl ChannelManager {
    pub fn new(bus: Arc<MessageBus>, event_bus: Arc<EventBus>) -> Self {
        Self {
            bus,
            event_bus,
            channels: Arc::new(Mutex::new(HashMap::new())),
            handles: Mutex::new(Vec::new()),
            cancel: CancellationToken::new(),
        }
    }

    /// 渠道实例表引用（runtime 查询 supports_streaming 用）。
    pub fn channels(&self) -> ChannelMap {
        self.channels.clone()
    }

    /// 启动所有渠道 + 出口分发任务。
    pub async fn start_all(&self, channels: Vec<Arc<dyn Channel>>) {
        // 1. 出口分发任务
        let outbound_rx = match self.bus.take_outbound().await {
            Some(rx) => rx,
            None => return, // 已启动过
        };
        let handle = tokio::spawn(dispatch_outbound(
            outbound_rx,
            self.channels.clone(),
            self.event_bus.clone(),
            self.cancel.clone(),
        ));
        self.handles.lock().await.push(handle);

        // 2. 每个渠道一个 start 任务
        let mut chan_map = self.channels.lock().await;
        for channel in channels {
            let name = channel.descriptor().id.to_string();
            let bus = self.bus.clone();
            let cancel = self.cancel.clone();
            let event_bus = self.event_bus.clone();
            let name_for_evt = name.clone();
            let ch_for_stop = channel.clone();
            let ch_for_start = channel.clone();
            let handle = tokio::spawn(async move {
                event_bus.publish(RuntimeEvent::ChannelStarted {
                    channel: name_for_evt.clone(),
                });
                if let Err(e) = ch_for_start.start(bus, cancel).await {
                    eprintln!("channel {name_for_evt} start error: {e}");
                }
                let _ = ch_for_stop.stop().await;
                event_bus.publish(RuntimeEvent::ChannelStopped {
                    channel: name_for_evt,
                });
            });
            chan_map.insert(name, channel);
            self.handles.lock().await.push(handle);
        }
    }

    /// 停止所有渠道 + 出口分发。
    #[allow(dead_code)]
    pub async fn stop_all(&self) {
        self.cancel.cancel();
        let mut handles = self.handles.lock().await;
        for handle in handles.drain(..) {
            handle.abort();
        }
        let channels = self.channels.lock().await;
        for (_, channel) in channels.iter() {
            let _ = channel.stop().await;
        }
        self.event_bus.publish(RuntimeEvent::RuntimeStopped);
    }

    /// 各渠道运行状态。
    pub async fn status(&self) -> Vec<ChannelStatus> {
        self.channels
            .lock()
            .await
            .keys()
            .map(|name| ChannelStatus {
                channel: name.clone(),
                running: !self.cancel.is_cancelled(),
                last_error: None,
            })
            .collect()
    }
}

/// 出口分发循环：消费 outbound，路由到渠道 send / send_delta，带重试与 coalescing。
async fn dispatch_outbound(
    mut rx: tokio::sync::mpsc::UnboundedReceiver<OutboundMessage>,
    channels: ChannelMap,
    event_bus: Arc<EventBus>,
    cancel: CancellationToken,
) {
    loop {
        let msg = tokio::select! {
            _ = cancel.cancelled() => break,
            m = rx.recv() => match m {
                Some(m) => m,
                None => break,
            },
        };

        let channel = channels.lock().await.get(msg.channel.as_str()).cloned();
        let Some(channel) = channel else {
            eprintln!("unknown channel: {}", msg.channel);
            continue;
        };

        // 流式 delta：尝试 coalesce 同流连续片段
        let msg = if matches!(
            msg.kind,
            OutboundKind::StreamDelta { .. } | OutboundKind::ReasoningDelta { .. }
        ) {
            coalesce_stream(&mut rx, msg)
        } else {
            msg
        };

        if let Err(e) = send_with_retry(&channel, &msg).await {
            event_bus.publish(RuntimeEvent::ReplyFailed {
                message_id: msg.reply_to.clone().unwrap_or_default(),
                error: e.to_string(),
            });
        } else if matches!(msg.kind, OutboundKind::Final) {
            event_bus.publish(RuntimeEvent::ReplySent {
                message_id: msg.reply_to.clone().unwrap_or_default(),
                channel: msg.channel.clone(),
                conversation_id: msg.chat_id.clone(),
            });
        }
    }
}

/// 合并同 `(channel, chat_id, stream_id)` 的连续 delta，减少渠道 API 调用。
///
/// 非阻塞地抽取队列头部连续的同流 delta；遇到边界（不同流 / 非流）则停止，
/// 该消息保留在队列由主循环下一轮处理。
fn coalesce_stream(
    rx: &mut tokio::sync::mpsc::UnboundedReceiver<OutboundMessage>,
    mut first: OutboundMessage,
) -> OutboundMessage {
    let (stream_id, is_reasoning) = match &first.kind {
        OutboundKind::StreamDelta { stream_id, .. } => (stream_id.clone(), false),
        OutboundKind::ReasoningDelta { stream_id, .. } => (stream_id.clone(), true),
        _ => return first,
    };

    let mut combined = std::mem::take(&mut first.content);
    let mut count = 0;
    loop {
        if count >= COALESCE_BATCH_MAX {
            break;
        }
        let Ok(next) = rx.try_recv() else {
            break;
        };
        let same_stream = next.channel == first.channel
            && next.chat_id == first.chat_id
            && match &next.kind {
                OutboundKind::StreamDelta { stream_id: s, .. } if !is_reasoning => s == &stream_id,
                OutboundKind::ReasoningDelta { stream_id: s, .. } if is_reasoning => {
                    s == &stream_id
                }
                _ => false,
            };
        if !same_stream {
            // 非同流：回填队列头部（unbounded 无 unrecv，记日志后丢弃边界以保持简单）
            eprintln!("coalesce: stream boundary crossed, dropping one out-of-stream msg");
            break;
        }
        combined.push_str(&next.content);
        count += 1;
        let ended = match &next.kind {
            OutboundKind::StreamDelta { end, .. } => *end,
            OutboundKind::ReasoningDelta { end, .. } => *end,
            _ => false,
        };
        if ended {
            first.kind = next.kind.clone();
            break;
        }
    }
    first.content = combined;
    first
}

/// 按重试策略发送一条出口消息。
async fn send_with_retry(
    channel: &Arc<dyn Channel>,
    msg: &OutboundMessage,
) -> Result<(), crate::services::gateway::GatewayError> {
    let max = SEND_RETRY_DELAYS.len();
    for (attempt, delay) in SEND_RETRY_DELAYS.iter().enumerate() {
        let result = match &msg.kind {
            OutboundKind::Final | OutboundKind::Progress { .. } | OutboundKind::TurnEnd => {
                channel.send(msg).await
            }
            OutboundKind::StreamDelta { .. } | OutboundKind::ReasoningDelta { .. } => {
                channel
                    .send_delta(&msg.chat_id, &msg.content, &msg.kind)
                    .await
            }
            OutboundKind::FileEdit { .. } => channel.send(msg).await,
        };
        match result {
            Ok(()) => return Ok(()),
            Err(e) if attempt + 1 < max => {
                eprintln!(
                    "send to {} failed (attempt {}/{}): {e}, retrying in {delay}s",
                    msg.channel,
                    attempt + 1,
                    max
                );
                tokio::time::sleep(std::time::Duration::from_secs(*delay)).await;
            }
            Err(e) => return Err(e),
        }
    }
    Ok(())
}
