use super::converter::{convert_feishu_message, FeishuMessageListResponse};
use super::token::TokenManager;
use crate::error::AppError;
use crate::services::gateway::{ChannelGateway, GatewayError};
use crate::services::message_bus::ChannelMessage;
use std::sync::Arc;

/// 飞书 API 基础 URL
const FEISHU_API_BASE: &str = "https://open.feishu.cn/open-apis";

/// 飞书 Gateway 客户端
pub struct FeishuClient {
    http_client: reqwest::Client,
    token_manager: Arc<TokenManager>,
}

impl FeishuClient {
    pub fn new(token_manager: Arc<TokenManager>) -> Self {
        Self {
            http_client: reqwest::Client::new(),
            token_manager,
        }
    }

    /// 拉取飞书消息列表
    /// 使用飞书 Open API: GET /im/v1/messages
    pub async fn poll_messages(
        &self,
        container_id_type: &str,
        page_size: i32,
        page_token: Option<&str>,
    ) -> Result<(Vec<ChannelMessage>, Option<String>), AppError> {
        let token = self.token_manager.get_token().await?;

        let mut url = format!(
            "{}/im/v1/messages?receive_id_type=open_id&container_id_type={}&page_size={}&sort_type=ByCreateTimeDesc",
            FEISHU_API_BASE, container_id_type, page_size
        );

        if let Some(pt) = page_token {
            url.push_str(&format!("&page_token={}", pt));
        }

        let resp = self
            .http_client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| AppError::FeishuGateway(format!("拉取消息网络错误: {}", e)))?;

        let body: FeishuMessageListResponse = resp
            .json()
            .await
            .map_err(|e| AppError::FeishuGateway(format!("解析消息列表失败: {}", e)))?;

        if body.code != 0 {
            return Err(AppError::FeishuGateway(format!(
                "拉取消息失败: {}",
                body.msg.as_deref().unwrap_or("未知错误")
            )));
        }

        let data = body.data.unwrap_or(
            FeishuMessageListResponse {
                code: 0,
                msg: None,
                data: None,
            }
            .data
            .unwrap_or(super::converter::FeishuMessageListData {
                items: None,
                has_more: None,
                page_token: None,
            }),
        );

        let messages: Vec<ChannelMessage> = data
            .items
            .unwrap_or_default()
            .into_iter()
            .map(|msg| convert_feishu_message(msg, "飞书用户"))
            .collect();

        let next_page_token = if data.has_more.unwrap_or(false) {
            data.page_token
        } else {
            None
        };

        Ok((messages, next_page_token))
    }

    /// 发送消息到飞书会话
    /// 使用飞书 Open API: POST /im/v1/messages
    pub async fn send_message(
        &self,
        receive_id: &str,
        msg_type: &str,
        content: &str,
        reply_msg_id: Option<&str>,
    ) -> Result<(), AppError> {
        let token = self.token_manager.get_token().await?;

        let content_json = if let Some(ref reply_id) = reply_msg_id {
            serde_json::json!({
                "text": content,
                "reply_msg_id": reply_id,
            })
        } else {
            serde_json::json!({
                "text": content,
            })
        };

        let body = serde_json::json!({
            "receive_id": receive_id,
            "msg_type": msg_type,
            "content": content_json.to_string(),
        });

        let resp = self
            .http_client
            .post(format!(
                "{}/im/v1/messages?receive_id_type=chat_id",
                FEISHU_API_BASE
            ))
            .header("Authorization", format!("Bearer {}", token))
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::FeishuGateway(format!("发送消息网络错误: {}", e)))?;

        #[derive(serde::Deserialize)]
        struct SendResp {
            code: i32,
            msg: Option<String>,
        }

        let send_resp: SendResp = resp
            .json()
            .await
            .map_err(|e| AppError::FeishuGateway(format!("解析发送响应失败: {}", e)))?;

        if send_resp.code != 0 {
            return Err(AppError::FeishuGateway(format!(
                "发送消息失败: {}",
                send_resp.msg.as_deref().unwrap_or("未知错误")
            )));
        }

        Ok(())
    }

    /// 发送回复消息到飞书
    #[allow(dead_code)]
    pub async fn send_reply(
        &self,
        conversation_id: &str,
        content: &str,
        reply_msg_id: &str,
    ) -> Result<(), AppError> {
        self.send_message(conversation_id, "text", content, Some(reply_msg_id))
            .await
    }

    /// 获取聊天名称
    #[allow(dead_code)]
    pub async fn get_chat_name(&self, chat_id: &str) -> Result<String, AppError> {
        let token = self.token_manager.get_token().await?;

        let resp = self
            .http_client
            .get(format!("{}/im/v1/chats/{}", FEISHU_API_BASE, chat_id))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| AppError::FeishuGateway(format!("获取聊天信息失败: {}", e)))?;

        #[derive(serde::Deserialize)]
        struct ChatResp {
            code: i32,
            data: Option<ChatData>,
        }

        #[derive(serde::Deserialize)]
        struct ChatData {
            name: Option<String>,
        }

        let chat_resp: ChatResp = resp
            .json()
            .await
            .map_err(|e| AppError::FeishuGateway(format!("解析聊天信息失败: {}", e)))?;

        Ok(chat_resp
            .data
            .and_then(|d| d.name)
            .unwrap_or_else(|| "未知群聊".to_string()))
    }
}

// ── ChannelGateway trait impl ────────────────────────────────

impl ChannelGateway for FeishuClient {
    fn channel_name(&self) -> &str {
        "feishu"
    }

    fn poll_messages(
        &self,
        page_size: i32,
        page_token: Option<&str>,
    ) -> Result<(Vec<ChannelMessage>, Option<String>), GatewayError> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                self.poll_messages("chat", page_size, page_token)
                    .await
                    .map_err(|e| match e {
                        AppError::FeishuGateway(msg) => GatewayError::Network(msg),
                        AppError::Unauthorized(_) => GatewayError::Unauthorized,
                        other => GatewayError::Network(other.to_string()),
                    })
            })
        })
    }

    fn send_message(
        &self,
        conversation_id: &str,
        content: &str,
        reply_to: Option<&str>,
    ) -> Result<(), GatewayError> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                self.send_message(conversation_id, "text", content, reply_to)
                    .await
                    .map_err(|e| match e {
                        AppError::FeishuGateway(msg) => GatewayError::Network(msg),
                        AppError::Unauthorized(_) => GatewayError::Unauthorized,
                        other => GatewayError::Network(other.to_string()),
                    })
            })
        })
    }

    fn credentials(&self) -> &dyn crate::services::gateway::CredentialsManager {
        self.token_manager.as_ref()
    }
}
