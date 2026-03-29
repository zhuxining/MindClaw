use crate::error::AppResult;

/// 对话：→ AgentLoop (发消息) + SessionManager (查历史)
#[tauri::command]
pub async fn send_message(_session_id: String, _content: String) -> AppResult<String> {
    todo!("实现发送消息命令")
}

#[tauri::command]
pub async fn get_session_history(_session_id: String) -> AppResult<Vec<String>> {
    todo!("实现获取会话历史命令")
}
