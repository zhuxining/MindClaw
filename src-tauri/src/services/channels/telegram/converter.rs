//! Telegram Update JSON → InboundMessage 归一化。

use crate::services::core::InboundMessage;

/// 从 getUpdates 的一项构造 InboundMessage。
pub fn to_inbound(update: &serde_json::Value) -> Option<InboundMessage> {
    let update_id = update.get("update_id")?.as_i64()?;
    let msg = update.get("message")?;
    let message_id = msg.get("message_id")?.as_i64()?;
    let chat = msg.get("chat")?;
    let chat_id = chat.get("id").map(|v| v.to_string()).unwrap_or_default();
    let from = msg.get("from")?;
    let sender_id = from
        .get("id")
        .map(|v| v.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let sender_name = from
        .get("first_name")
        .and_then(|v| v.as_str())
        .unwrap_or("Telegram User")
        .to_string();
    let content = msg
        .get("text")
        .or_else(|| msg.get("caption"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let timestamp = msg.get("date").and_then(|v| v.as_i64()).unwrap_or(0);

    Some(InboundMessage {
        channel: "telegram".to_string(),
        sender_id,
        chat_id,
        content,
        media: Vec::new(),
        metadata: serde_json::json!({
            "message_id": format!("tg_{update_id}_{message_id}"),
            "sender_name": sender_name,
            "timestamp": timestamp,
        }),
        session_key_override: None,
    })
}
