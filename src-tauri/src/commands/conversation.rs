use crate::bus::events::InboundMessage;
use crate::error::AppResult;
use crate::models::conversation::ConversationMode;
use crate::providers::MessageContent;
use crate::runtime::AppRuntime;
use std::sync::Arc;

/// 发送消息到 Agent
#[tauri::command]
pub async fn send_message(
    runtime: tauri::State<'_, Arc<AppRuntime>>,
    session_id: Option<String>,
    content: String,
) -> AppResult<String> {
    let request_id = uuid::Uuid::new_v4().to_string();

    let message = InboundMessage {
        id: uuid::Uuid::new_v4().to_string(),
        request_id: request_id.clone(),
        session_id,
        sender: "desktop_user".to_string(),
        channel: "desktop".to_string(),
        mode: ConversationMode::Chat,
        content,
        timestamp: chrono::Utc::now().timestamp_millis(),
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
