use crate::error::AppError;
use crate::services::gateway::{ChannelGateway, CredentialsManager};
use crate::services::message_bus::ChannelMessage;
use std::sync::Arc;

use super::converter::convert_telegram_update;
use super::token::TelegramTokenManager;

/// Telegram Bot API 基础 URL
const TELEGRAM_API_BASE: &str = "https://api.telegram.org";

/// Telegram Gateway 客户端
pub struct TelegramClient {
    http_client: reqwest::Client,
    token_manager: Arc<TelegramTokenManager>,
    /// 上次拉取的 update_id（用于增量拉取）
    last_update_id: tokio::sync::Mutex<Option<i64>>,
}

impl TelegramClient {
    pub fn new(token_manager: Arc<TelegramTokenManager>) -> Self {
        Self {
            http_client: reqwest::Client::new(),
            token_manager,
            last_update_id: tokio::sync::Mutex::new(None),
        }
    }

    /// 获取 Bot API URL 前缀
    async fn api_url(&self) -> Result<String, crate::error::AppError> {
        let token = self.token_manager.get_token().await?;
        Ok(format!("{}/bot{}", TELEGRAM_API_BASE, token))
    }

    /// 拉取 Telegram 消息（轮询 getUpdates）
    pub async fn poll_messages(
        &self,
        page_size: i32,
        page_token: Option<&str>,
    ) -> Result<(Vec<ChannelMessage>, Option<String>), crate::error::AppError> {
        let base_url = self.api_url().await?;
        let mut last_id = self.last_update_id.lock().await;

        let mut url = format!(
            "{}/getUpdates?limit={}&offset={}&allowed_updates=[\"message\"]",
            base_url,
            page_size,
            last_id.map(|id| id + 1).unwrap_or(0),
        );

        if let Some(pt) = page_token {
            url.push_str(&format!("&offset={}", pt));
        }

        let resp = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| AppError::Gateway(format!("拉取消息网络错误: {}", e)))?;

        #[derive(serde::Deserialize)]
        struct TelegramResponse {
            ok: bool,
            result: Option<Vec<serde_json::Value>>,
        }

        let body: TelegramResponse = resp
            .json()
            .await
            .map_err(|e| AppError::Gateway(format!("解析消息列表失败: {}", e)))?;

        if !body.ok {
            return Err(AppError::Gateway("Telegram API 返回错误".into()));
        }

        let updates = body.result.unwrap_or_default();
        let messages: Vec<ChannelMessage> =
            updates.iter().filter_map(convert_telegram_update).collect();

        // 更新 last_update_id
        if let Some(last_msg) = updates.last() {
            if let Some(update_id) = last_msg.get("update_id").and_then(|v| v.as_i64()) {
                *last_id = Some(update_id);
            }
        }

        let next_page_token = if updates.len() >= page_size as usize {
            last_id.map(|id| id.to_string())
        } else {
            None
        };

        Ok((messages, next_page_token))
    }

    /// 发送消息到 Telegram 会话
    pub async fn send_message(
        &self,
        chat_id: &str,
        content: &str,
        reply_to_msg_id: Option<&str>,
    ) -> Result<(), AppError> {
        let base_url = self.api_url().await?;

        let mut body = serde_json::json!({
            "chat_id": chat_id,
            "text": content,
            "parse_mode": "HTML",
        });

        if let Some(reply_id) = reply_to_msg_id {
            body["reply_to_message_id"] = serde_json::Value::String(reply_id.to_string());
        }

        let resp = self
            .http_client
            .post(format!("{}/sendMessage", base_url))
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::Gateway(format!("发送消息网络错误: {}", e)))?;

        #[derive(serde::Deserialize)]
        struct SendResp {
            ok: bool,
        }

        let send_resp: SendResp = resp
            .json()
            .await
            .map_err(|e| AppError::Gateway(format!("解析发送响应失败: {}", e)))?;

        if !send_resp.ok {
            return Err(AppError::Gateway("发送消息失败".into()));
        }

        Ok(())
    }
}

// ── ChannelGateway trait impl ────────────────────────────────

impl ChannelGateway for TelegramClient {
    fn channel_name(&self) -> &str {
        "telegram"
    }

    fn poll_messages(
        &self,
        page_size: i32,
        page_token: Option<&str>,
    ) -> Result<(Vec<ChannelMessage>, Option<String>), crate::services::gateway::GatewayError> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                self.poll_messages(page_size, page_token)
                    .await
                    .map_err(|e| match e {
                        AppError::Gateway(msg) => {
                            crate::services::gateway::GatewayError::Network(msg)
                        }
                        AppError::Unauthorized(_) => {
                            crate::services::gateway::GatewayError::Unauthorized
                        }
                        other => crate::services::gateway::GatewayError::Network(other.to_string()),
                    })
            })
        })
    }

    fn send_message(
        &self,
        conversation_id: &str,
        content: &str,
        reply_to: Option<&str>,
    ) -> Result<(), crate::services::gateway::GatewayError> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                self.send_message(conversation_id, content, reply_to)
                    .await
                    .map_err(|e| match e {
                        AppError::Gateway(msg) => {
                            crate::services::gateway::GatewayError::Network(msg)
                        }
                        AppError::Unauthorized(_) => {
                            crate::services::gateway::GatewayError::Unauthorized
                        }
                        other => crate::services::gateway::GatewayError::Network(other.to_string()),
                    })
            })
        })
    }

    fn credentials(&self) -> &dyn CredentialsManager {
        self.token_manager.as_ref()
    }
}
