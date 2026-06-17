pub mod secrets;

pub use secrets::{credential_key, SecretStore};

use serde::{Deserialize, Serialize};

/// 统一渠道消息 — 所有渠道的消息都转换为此格式。
///
/// 现为 UI 读模型：渠道运行时归一化为 [`InboundMessage`] 推送调度器，
/// 同时落库 / 回写为 `ChannelMessage` 供前端展示历史。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelMessage {
    /// 唯一消息 ID（来自原始渠道，用于去重）
    pub message_id: String,
    /// 来源渠道标识（如 "feishu"）
    pub channel: String,
    /// 会话/群聊 ID
    pub conversation_id: String,
    /// 发送者标识
    pub sender_id: String,
    /// 发送者显示名称
    pub sender_name: String,
    /// 消息文本内容
    pub content: String,
    /// 消息时间戳（Unix 秒）
    pub timestamp: i64,
    /// 是否为 Agent 回复（回写时标记）
    pub is_reply: bool,
    /// 引用消息 ID（回复时使用）
    pub reply_to: Option<String>,
}

/// Agent 处理结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResponse {
    /// 对应的请求 ID
    pub request_id: String,
    /// 处理状态
    pub status: ResponseStatus,
    /// 输出内容
    pub output: String,
    /// 错误信息（status 为 Error 时）
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ResponseStatus {
    #[serde(rename = "success")]
    Success,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "timeout")]
    Timeout,
}

// ── 渠道运行时归一化消息（参考 nanobot BaseChannel / MessageBus） ────────

/// 渠道运行时入口归一化消息。
///
/// ConcreteChannel 将平台事件解析为 `InboundMessage` 后投递到 `MessageBus`，
/// 由 `ChannelRuntime` 消费并交给 `SessionDispatcher`。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboundMessage {
    /// 来源渠道标识（如 "feishu"）
    pub channel: String,
    /// 发送者标识（渠道特有，如 open_id）
    pub sender_id: String,
    /// 会话/群聊 ID（= ChannelMessage.conversation_id）
    pub chat_id: String,
    /// 消息文本内容
    pub content: String,
    /// 附件（本地路径或渠道 marker）
    #[serde(default)]
    pub media: Vec<String>,
    /// 渠道特有上下文（message_id / chat_type / reply 信息等）
    #[serde(default)]
    pub metadata: serde_json::Value,
    /// 显式 session key 覆盖，默认 `"{channel}:{chat_id}"`
    #[serde(default)]
    pub session_key_override: Option<String>,
}

impl InboundMessage {
    /// session 分区键：默认 `{channel}:{chat_id}`。
    #[allow(dead_code)]
    pub fn session_key(&self) -> String {
        self.session_key_override
            .clone()
            .unwrap_or_else(|| format!("{}:{}", self.channel, self.chat_id))
    }
}

/// 渠道运行时出口归一化消息。
///
/// `SessionDispatcher` / Agent 产出的回复通过 `OutboundMessage` 投递到 `MessageBus`，
/// 由 `ChannelManager.dispatch_outbound` 按 `kind` 路由到对应渠道 send 原语。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboundMessage {
    /// 目标渠道标识
    pub channel: String,
    /// 目标会话/群聊 ID
    pub chat_id: String,
    /// 文本内容（流式 delta 时为增量片段）
    pub content: String,
    /// 引用消息 ID（回复时使用）
    #[serde(default)]
    pub reply_to: Option<String>,
    /// 附件路径
    #[serde(default)]
    pub media: Vec<String>,
    /// 消息种类（决定调用 send / send_delta / send_reasoning_*）
    pub kind: OutboundKind,
}

/// 出口消息种类 — 替代 nanobot 的 `metadata` dict 约定，强类型化以利于 Rust 匹配。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutboundKind {
    /// 普通完整消息
    Final,
    /// 流式增量片段；`end=true` 标记该 stream 结束
    StreamDelta { stream_id: String, end: bool },
    /// 推理过程增量片段；`end=true` 标记结束
    ReasoningDelta { stream_id: String, end: bool },
    /// 结构化文件编辑事件
    FileEdit {
        #[serde(default)]
        events: Vec<FileEditEvent>,
    },
    /// 进度提示（可选标记为 tool hint）
    Progress {
        #[serde(default)]
        tool_hint: bool,
    },
    /// 一轮对话结束
    TurnEnd,
}

/// 文件编辑事件（流式 UI 用）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileEditEvent {
    pub path: String,
    #[serde(default)]
    pub summary: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_message_serializes_reply_to_null() {
        let message = ChannelMessage {
            message_id: "msg-1".to_string(),
            channel: "feishu".to_string(),
            conversation_id: "chat-1".to_string(),
            sender_id: "user-1".to_string(),
            sender_name: "User".to_string(),
            content: "hello".to_string(),
            timestamp: 1,
            is_reply: false,
            reply_to: None,
        };

        let json = serde_json::to_value(message).unwrap();
        assert!(json.get("reply_to").unwrap().is_null());
    }
}
