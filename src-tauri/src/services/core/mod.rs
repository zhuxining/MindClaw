use serde::{Deserialize, Serialize};

/// 统一渠道消息 — 所有渠道的消息都转换为此格式。
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
