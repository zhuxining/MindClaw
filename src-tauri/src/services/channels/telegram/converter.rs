use crate::services::core::ChannelMessage;

/// 将 Telegram Update JSON 转换为统一 ChannelMessage
///
/// Telegram getUpdates 返回的 Update 结构：
/// {
///   "update_id": 123,
///   "message": {
///     "message_id": 456,
///     "from": { "id": 789, "first_name": "John", "username": "johndoe" },
///     "chat": { "id": -100123, "title": "Group", "type": "group" },
///     "text": "Hello",
///     "date": 1680000000
///   }
/// }
pub fn convert_telegram_update(update: &serde_json::Value) -> Option<ChannelMessage> {
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

    Some(ChannelMessage {
        message_id: format!("tg_{}_{}", update_id, message_id),
        channel: "telegram".to_string(),
        conversation_id: chat_id,
        sender_id,
        sender_name,
        content,
        timestamp,
        is_reply: false,
        reply_to: None,
    })
}
