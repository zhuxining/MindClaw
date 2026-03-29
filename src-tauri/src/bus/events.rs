use crate::agent::events::UserVisiblePhase;
use crate::models::conversation::ConversationMode;
use serde::{Deserialize, Serialize};

/// 进入 MessageBus 的入站消息（Channel → Agent）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboundMessage {
    pub id: String,
    /// 请求 ID（用于追踪）
    pub request_id: String,
    /// 会话 ID（可选，新会话时为 None）
    pub session_id: Option<String>,
    pub sender: String,
    pub channel: String,
    /// 对话模式
    pub mode: ConversationMode,
    pub content: String,
    pub timestamp: i64,
}

/// 从 MessageBus 出站的消息（Agent → Channel）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboundMessage {
    pub id: String,
    /// 请求 ID（关联入站消息）
    pub request_id: String,
    /// 会话 ID
    pub session_id: String,
    pub target_sender: String,
    pub target_channel: String,
    /// 出站 payload
    pub payload: OutboundPayload,
}

/// 出站消息负载
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum OutboundPayload {
    /// 文本增量（流式）
    Chunk { content: String },
    /// 完成
    Done,
    /// 错误
    Error { message: String, retryable: bool },
    /// 状态更新
    Status { phase: UserVisiblePhase },
}

/// 消息来源
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChannelSource {
    Desktop,
    Telegram,
    Feishu,
    Webhook,
}
