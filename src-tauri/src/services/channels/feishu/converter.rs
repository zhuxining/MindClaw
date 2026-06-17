//! 飞书 WS 事件 → InboundMessage 归一化。

use crate::services::core::InboundMessage;

/// LarkEvent envelope（method=1 / type=event payload）。
#[derive(Debug, serde::Deserialize)]
pub struct LarkEvent {
    pub header: LarkEventHeader,
    pub event: serde_json::Value,
}

#[derive(Debug, serde::Deserialize)]
pub struct LarkEventHeader {
    pub event_type: String,
    #[allow(dead_code)]
    pub event_id: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct MsgReceivePayload {
    pub sender: LarkSender,
    pub message: LarkMessage,
}

#[derive(Debug, serde::Deserialize)]
pub struct LarkSender {
    pub sender_id: LarkSenderId,
    #[serde(default)]
    pub sender_type: String,
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct LarkSenderId {
    #[serde(default)]
    pub open_id: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct LarkMessage {
    pub message_id: String,
    pub chat_id: String,
    pub chat_type: String,
    pub message_type: String,
    #[serde(default)]
    pub content: String,
}

/// 从 `im.message.receive_v1` 事件 payload 构造 InboundMessage。
///
/// 仅处理 text / post；其它类型返回 None（上层跳过）。
pub fn to_inbound(payload: &serde_json::Value) -> Option<InboundMessage> {
    let recv: MsgReceivePayload = serde_json::from_value(payload.clone()).ok()?;

    if recv.sender.sender_type == "app" || recv.sender.sender_type == "bot" {
        return None;
    }

    let sender_id = recv
        .sender
        .sender_id
        .open_id
        .clone()
        .unwrap_or_else(|| "unknown".to_string());

    let text = match recv.message.message_type.as_str() {
        "text" => {
            let v: serde_json::Value = serde_json::from_str(&recv.message.content).ok()?;
            v.get("text")
                .and_then(|t| t.as_str())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())?
        }
        "post" => parse_post_text(&recv.message.content)?,
        _ => return None,
    };
    let text = text.trim().to_string();
    if text.is_empty() {
        return None;
    }

    Some(InboundMessage {
        channel: "feishu".to_string(),
        sender_id,
        chat_id: recv.message.chat_id.clone(),
        content: text,
        media: Vec::new(),
        metadata: serde_json::json!({
            "message_id": recv.message.message_id,
            "chat_type": recv.message.chat_type,
            "msg_type": recv.message.message_type,
            "timestamp": chrono::Utc::now().timestamp(),
        }),
        session_key_override: None,
    })
}

/// 解析 post 消息为纯文本。
fn parse_post_content(content: &str) -> Option<Vec<Vec<PostNode>>> {
    let v: serde_json::Value = serde_json::from_str(content).ok()?;
    v.get("content")
        .and_then(|c| serde_json::from_value(c.clone()).ok())
}

fn parse_post_text(content: &str) -> Option<String> {
    let lines = parse_post_content(content)?;
    let mut out = String::new();
    for line in lines {
        for node in line {
            if let Some(t) = node.text {
                out.push_str(&t);
            }
        }
        out.push('\n');
    }
    let trimmed = out.trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

#[derive(Debug, serde::Deserialize)]
struct PostNode {
    #[serde(default)]
    text: Option<String>,
}
