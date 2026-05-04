use crate::agent::messages::MessageContent;
use crate::agent::session::SessionListItem;
use crate::bus::events::InboundMessage;
use crate::error::AppResult;
use crate::models::conversation::ConversationMode;
use crate::runtime::AppRuntime;
use std::sync::Arc;

/// 发送消息到 Agent
#[tauri::command]
pub async fn send_message(
    runtime: tauri::State<'_, Arc<AppRuntime>>,
    session_id: Option<String>,
    content: String,
    mode: Option<String>,
) -> AppResult<String> {
    let request_id = uuid::Uuid::new_v4().to_string();

    let conversation_mode = match mode.as_deref() {
        Some("companion") => ConversationMode::Companion,
        Some("reflection") => ConversationMode::Reflection,
        Some("challenge") => ConversationMode::Challenge,
        Some("vault") => ConversationMode::Vault,
        Some("private") => ConversationMode::Private,
        _ => ConversationMode::Companion, // 默认陪伴模式
    };

    let message = InboundMessage {
        id: uuid::Uuid::new_v4().to_string(),
        request_id: request_id.clone(),
        session_id,
        sender: "desktop_user".to_string(),
        channel: "desktop".to_string(),
        mode: conversation_mode,
        content,
        timestamp: chrono::Utc::now().timestamp_millis(),
        is_injection: false,
    };

    runtime.bus().publish_inbound(message).await?;
    Ok(request_id)
}

/// 获取会话历史
#[tauri::command]
pub async fn get_session_history(
    runtime: tauri::State<'_, Arc<AppRuntime>>,
    session_id: String,
) -> AppResult<Vec<String>> {
    let session = runtime.session_mgr().get(&session_id).await;
    match session {
        Some(s) => {
            let messages: Vec<String> = s
                .turns
                .iter()
                .filter_map(|t| {
                    t.user_message.content.iter().find_map(|c| match c {
                        MessageContent::Text { text } => Some(text.clone()),
                        _ => None,
                    })
                })
                .collect();
            Ok(messages)
        }
        None => Ok(Vec::new()),
    }
}

/// 列出所有会话
#[tauri::command]
pub async fn list_sessions(
    runtime: tauri::State<'_, Arc<AppRuntime>>,
    limit: Option<usize>,
) -> AppResult<Vec<SessionListItem>> {
    let limit = limit.unwrap_or(50);
    runtime.session_mgr().list_sessions(limit).await
}

/// 删除会话
#[tauri::command]
pub async fn delete_session(
    runtime: tauri::State<'_, Arc<AppRuntime>>,
    session_id: String,
) -> AppResult<()> {
    runtime.session_mgr().delete_session(&session_id).await
}
