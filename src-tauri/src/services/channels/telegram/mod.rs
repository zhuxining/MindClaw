//! 电报渠道：long-polling ingress + send-then-edit 流式 send_delta + Stronghold 凭证。

pub mod client;
pub mod converter;
pub mod token;

pub use token::TelegramCredentials;

use crate::services::channels::{
    Capabilities, Channel, ChannelDeps, ChannelDescriptor, ChannelFactory, CredentialsManager,
    MessageBus,
};
use crate::services::core::{InboundMessage, OutboundKind, OutboundMessage, SecretStore};
use crate::services::event_bus::EventBus;
use crate::services::gateway::GatewayError;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

const TELEGRAM_API_BASE: &str = "https://api.telegram.org";
/// 流式编辑间隔。
const STREAM_EDIT_INTERVAL: Duration = Duration::from_millis(600);

static DESCRIPTOR: OnceLock<ChannelDescriptor> = OnceLock::new();

pub fn descriptor() -> &'static ChannelDescriptor {
    DESCRIPTOR.get_or_init(|| ChannelDescriptor {
        id: "telegram",
        display_name: "Telegram",
        inbound: crate::services::channels::InboundKind::LongPolling,
        credential_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "bot_token": { "type": "string", "title": "Bot Token", "format": "password" }
            },
            "required": ["bot_token"]
        }),
        capabilities: Capabilities {
            streaming: true,
            reasoning: false,
            file_edit: false,
            reply: true,
        },
    })
}

#[derive(Debug, Clone, Default)]
struct StreamBuf {
    text: String,
    message_id: Option<i64>,
    last_edit: Option<std::time::Instant>,
}

pub struct TelegramChannel {
    http: reqwest::Client,
    creds: Arc<TelegramCredentials>,
    secrets: Arc<dyn SecretStore>,
    event_bus: Arc<EventBus>,
    last_update_id: Mutex<Option<i64>>,
    streams: Mutex<HashMap<String, StreamBuf>>,
}

impl TelegramChannel {
    pub fn new(
        http: reqwest::Client,
        secrets: Arc<dyn SecretStore>,
        event_bus: Arc<EventBus>,
    ) -> Self {
        Self {
            http,
            creds: Arc::new(TelegramCredentials::new()),
            secrets,
            event_bus,
            last_update_id: Mutex::new(None),
            streams: Mutex::new(HashMap::new()),
        }
    }

    async fn api_url(&self) -> Result<String, GatewayError> {
        let token = self.creds.get_token().await?;
        Ok(format!("{TELEGRAM_API_BASE}/bot{token}"))
    }
}

#[async_trait::async_trait]
impl Channel for TelegramChannel {
    fn descriptor(&self) -> &'static ChannelDescriptor {
        descriptor()
    }

    async fn start(
        &self,
        bus: Arc<MessageBus>,
        cancel: CancellationToken,
    ) -> Result<(), GatewayError> {
        // 等待凭证就绪
        loop {
            if cancel.is_cancelled() {
                return Ok(());
            }
            self.creds.load(self.secrets.as_ref()).await?;
            if self.creds.has_credentials().await {
                break;
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }

        loop {
            if cancel.is_cancelled() {
                return Ok(());
            }
            match self.poll_once().await {
                Ok(messages) => {
                    for msg in messages {
                        bus.publish_inbound(msg);
                    }
                }
                Err(e) => {
                    self.event_bus.publish(
                        crate::services::event_bus::RuntimeEvent::ChannelPollFailed {
                            channel: "telegram".into(),
                            error: e.to_string(),
                        },
                    );
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
            // 短间隔拉取
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }

    async fn stop(&self) -> Result<(), GatewayError> {
        Ok(())
    }

    async fn send(&self, msg: &OutboundMessage) -> Result<(), GatewayError> {
        let base = self.api_url().await?;
        let mut body = serde_json::json!({
            "chat_id": msg.chat_id,
            "text": msg.content,
            "parse_mode": "HTML",
        });
        if let Some(reply_to) = &msg.reply_to {
            body["reply_to_message_id"] = serde_json::Value::String(reply_to.clone());
        }
        let resp = self
            .http
            .post(format!("{base}/sendMessage"))
            .json(&body)
            .send()
            .await
            .map_err(|_| GatewayError::Network("发送消息网络错误: 请求失败".into()))?;
        #[derive(serde::Deserialize)]
        struct R {
            ok: bool,
        }
        let r: R = resp
            .json()
            .await
            .map_err(|_| GatewayError::Network("解析响应失败".into()))?;
        if !r.ok {
            return Err(GatewayError::Network("发送失败".into()));
        }
        Ok(())
    }

    async fn send_delta(
        &self,
        chat_id: &str,
        delta: &str,
        kind: &OutboundKind,
    ) -> Result<(), GatewayError> {
        let (end, is_reasoning) = match kind {
            OutboundKind::StreamDelta { end, .. } => (*end, false),
            OutboundKind::ReasoningDelta { end, .. } => (*end, true),
            _ => return Ok(()),
        };
        if is_reasoning {
            return Ok(());
        }

        let mut streams = self.streams.lock().await;
        let buf = streams
            .entry(chat_id.to_string())
            .or_insert_with(StreamBuf::default);
        if !delta.is_empty() {
            buf.text.push_str(delta);
        }
        let now = std::time::Instant::now();

        if end {
            // 最终编辑
            if let Some(msg_id) = buf.message_id {
                let base = self.api_url().await?;
                let body = serde_json::json!({
                    "chat_id": chat_id,
                    "message_id": msg_id,
                    "text": buf.text,
                    "parse_mode": "HTML",
                });
                let _ = self
                    .http
                    .post(format!("{base}/editMessageText"))
                    .json(&body)
                    .send()
                    .await;
            }
            streams.remove(chat_id);
            return Ok(());
        }

        if buf.message_id.is_none() {
            // 首条：发送新消息
            let base = self.api_url().await?;
            let body = serde_json::json!({
                "chat_id": chat_id,
                "text": buf.text,
                "parse_mode": "HTML",
            });
            let resp = self
                .http
                .post(format!("{base}/sendMessage"))
                .json(&body)
                .send()
                .await
                .map_err(|_| GatewayError::Network("发送消息失败".into()))?;
            #[derive(serde::Deserialize)]
            struct SendResult {
                ok: bool,
                result: Option<MsgResult>,
            }
            #[derive(serde::Deserialize)]
            struct MsgResult {
                message_id: i64,
            }
            let r: SendResult = resp
                .json()
                .await
                .map_err(|_| GatewayError::Network("解析失败".into()))?;
            if r.ok {
                if let Some(msg) = r.result {
                    buf.message_id = Some(msg.message_id);
                }
            }
            buf.last_edit = Some(now);
            return Ok(());
        }

        // 节流编辑
        let should_edit = buf
            .last_edit
            .map(|t| now.duration_since(t) >= STREAM_EDIT_INTERVAL)
            .unwrap_or(true);
        if should_edit && buf.message_id.is_some() {
            let base = self.api_url().await?;
            let body = serde_json::json!({
                "chat_id": chat_id,
                "message_id": buf.message_id,
                "text": buf.text,
                "parse_mode": "HTML",
            });
            let _ = self
                .http
                .post(format!("{base}/editMessageText"))
                .json(&body)
                .send()
                .await;
            buf.last_edit = Some(now);
        }
        Ok(())
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    fn credentials(&self) -> &dyn CredentialsManager {
        self.creds.as_ref()
    }
}

impl TelegramChannel {
    async fn poll_once(&self) -> Result<Vec<InboundMessage>, GatewayError> {
        let base = self.api_url().await?;
        let mut last_id = self.last_update_id.lock().await;
        let url = format!(
            "{base}/getUpdates?limit=20&offset={}&timeout=30&allowed_updates=[\"message\"]",
            last_id.map(|id| id + 1).unwrap_or(0),
        );
        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|_| GatewayError::Network("拉取消息网络错误: 请求失败".into()))?;
        #[derive(serde::Deserialize)]
        struct R {
            ok: bool,
            result: Option<Vec<serde_json::Value>>,
        }
        let body: R = resp
            .json()
            .await
            .map_err(|_| GatewayError::Network("解析失败".into()))?;
        if !body.ok {
            return Err(GatewayError::Network("API 返回错误".into()));
        }
        let updates = body.result.unwrap_or_default();

        let mut messages = Vec::new();
        for update in &updates {
            if let Some(msg) = converter::to_inbound(update) {
                messages.push(msg);
            }
        }
        if let Some(last) = updates.last() {
            if let Some(id) = last.get("update_id").and_then(|v| v.as_i64()) {
                *last_id = Some(id);
            }
        }
        Ok(messages)
    }
}

pub struct TelegramFactory;

impl ChannelFactory for TelegramFactory {
    fn descriptor(&self) -> &'static ChannelDescriptor {
        descriptor()
    }

    fn build(&self, deps: &ChannelDeps) -> Arc<dyn Channel> {
        Arc::new(TelegramChannel::new(
            deps.http.clone(),
            deps.secrets.clone(),
            deps.event_bus.clone(),
        ))
    }
}
