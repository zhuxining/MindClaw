use crate::services::message_bus::ChannelMessage;
use serde::{Deserialize, Serialize};

/// 飞书消息原始结构（来自 API 响应）
#[derive(Debug, Deserialize)]
pub struct FeishuMessage {
    pub message_id: String,
    pub chat_id: String,
    #[allow(dead_code)]
    pub chat_type: String,
    #[allow(dead_code)]
    pub msg_type: String,
    pub sender: Option<FeishuSender>,
    pub body: Option<FeishuMessageBody>,
    pub create_time: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct FeishuSender {
    pub id: Option<FeishuUserId>,
    #[allow(dead_code)]
    pub sender_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct FeishuUserId {
    pub user_id: Option<String>,
    pub open_id: Option<String>,
    pub union_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct FeishuMessageBody {
    pub content: Option<String>,
}

/// 飞书 API 消息列表响应
#[derive(Debug, Deserialize)]
pub struct FeishuMessageListResponse {
    pub code: i32,
    pub msg: Option<String>,
    pub data: Option<FeishuMessageListData>,
}

#[derive(Debug, Deserialize)]
pub struct FeishuMessageListData {
    pub items: Option<Vec<FeishuMessage>>,
    pub has_more: Option<bool>,
    pub page_token: Option<String>,
}

/// 飞书发送消息请求
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct FeishuSendMessageRequest {
    pub msg_type: String,
    pub content: String,
}

/// 飞书文本消息内容
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct FeishuTextContent {
    pub text: String,
}

/// 飞书回复消息内容（包含引用）
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct FeishuReplyContent {
    pub text: String,
    #[serde(rename = "reply_msg_id")]
    pub reply_msg_id: String,
}

/// 将飞书消息转换为统一 ChannelMessage
pub fn convert_feishu_message(msg: FeishuMessage, sender_name: &str) -> ChannelMessage {
    let content = msg.body.and_then(|b| b.content).unwrap_or_default();

    let sender_id = msg
        .sender
        .as_ref()
        .and_then(|s| s.id.as_ref())
        .and_then(|id| {
            id.open_id
                .clone()
                .or(id.user_id.clone().or(id.union_id.clone()))
        })
        .unwrap_or_else(|| "unknown".to_string());

    let timestamp = msg
        .create_time
        .and_then(|t| t.parse::<i64>().ok())
        .unwrap_or(0);

    ChannelMessage {
        message_id: msg.message_id,
        channel: "feishu".to_string(),
        conversation_id: msg.chat_id,
        sender_id,
        sender_name: sender_name.to_string(),
        content,
        timestamp,
        is_reply: false,
        reply_to: None,
    }
}
