//! 飞书渠道：WS 长连接 ingress + CardKit 流式 send_delta + Stronghold 凭证。

pub mod client;
pub mod converter;
pub mod token;
pub mod ws;

use crate::services::channels::{
    Capabilities, Channel, ChannelDeps, ChannelDescriptor, ChannelFactory, CredentialsManager,
    MessageBus,
};
use crate::services::core::{OutboundKind, OutboundMessage, SecretStore};
use crate::services::event_bus::EventBus;
use crate::services::gateway::GatewayError;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

pub use token::FeishuCredentials;
use token::INVALID_TOKEN_CODE;

/// 流式卡片 PATCH 最小间隔。
const STREAM_EDIT_INTERVAL: Duration = Duration::from_millis(500);

static DESCRIPTOR: OnceLock<ChannelDescriptor> = OnceLock::new();

/// 飞书渠道描述符（运行时构造，因含 serde_json::Value）。
pub fn descriptor() -> &'static ChannelDescriptor {
    DESCRIPTOR.get_or_init(|| ChannelDescriptor {
        id: "feishu",
        display_name: "飞书",
        inbound: crate::services::channels::InboundKind::LongConnection,
        credential_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "app_id": { "type": "string", "title": "App ID" },
                "app_secret": { "type": "string", "title": "App Secret", "format": "password" }
            },
            "required": ["app_id", "app_secret"]
        }),
        capabilities: Capabilities {
            streaming: true,
            reasoning: false,
            file_edit: false,
            reply: true,
        },
    })
}

/// 飞书渠道实例。
pub struct FeishuChannel {
    http: reqwest::Client,
    creds: Arc<FeishuCredentials>,
    secrets: Arc<dyn SecretStore>,
    event_bus: Arc<EventBus>,
    streams: Mutex<HashMap<String, client::StreamBuf>>,
}

impl FeishuChannel {
    pub fn new(
        http: reqwest::Client,
        secrets: Arc<dyn SecretStore>,
        event_bus: Arc<EventBus>,
    ) -> Self {
        let creds = Arc::new(FeishuCredentials::new(http.clone()));
        Self {
            http,
            creds,
            secrets,
            event_bus,
            streams: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait::async_trait]
impl Channel for FeishuChannel {
    fn descriptor(&self) -> &'static ChannelDescriptor {
        descriptor()
    }

    async fn start(
        &self,
        bus: Arc<MessageBus>,
        cancel: CancellationToken,
    ) -> Result<(), GatewayError> {
        // 等待凭证就绪（用户可能稍后才配置）
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

        let app_id = self
            .creds
            .app_id()
            .await
            .ok_or(GatewayError::Unauthorized)?;
        let app_secret = self
            .creds
            .app_secret()
            .await
            .ok_or(GatewayError::Unauthorized)?;
        eprintln!(
            "[feishu] credentials loaded, starting WS (app_id={:.8}...)",
            &app_id[..8.min(app_id.len())]
        );

        ws::run_with_reconnect(
            &self.http,
            &app_id,
            &app_secret,
            bus.as_ref(),
            &self.event_bus,
            cancel,
        )
        .await
    }

    async fn stop(&self) -> Result<(), GatewayError> {
        Ok(())
    }

    async fn send(&self, msg: &OutboundMessage) -> Result<(), GatewayError> {
        match &msg.kind {
            OutboundKind::Final | OutboundKind::TurnEnd => {
                client::send_text(&self.http, self.creds.as_ref(), &msg.chat_id, &msg.content)
                    .await
                    .map_err(|e| self.maybe_invalidate(e))
            }
            // 非流式渠道收到进度/文件编辑：以文本发送
            _ => client::send_text(&self.http, self.creds.as_ref(), &msg.chat_id, &msg.content)
                .await
                .map_err(|e| self.maybe_invalidate(e)),
        }
    }

    async fn send_delta(
        &self,
        chat_id: &str,
        delta: &str,
        kind: &OutboundKind,
    ) -> Result<(), GatewayError> {
        let (stream_id, end, is_reasoning) = match kind {
            OutboundKind::StreamDelta { stream_id, end } => (stream_id.clone(), *end, false),
            OutboundKind::ReasoningDelta { stream_id, end } => (stream_id.clone(), *end, true),
            _ => return Ok(()),
        };
        let _ = stream_id;
        if is_reasoning {
            // 推理过程暂不展示，丢弃
            return Ok(());
        }

        let mut streams = self.streams.lock().await;
        let buf = streams
            .entry(chat_id.to_string())
            .or_insert_with(client::StreamBuf::default);
        if !delta.is_empty() {
            buf.text.push_str(delta);
        }

        let now = Instant::now();
        if end {
            // 流结束：最终渲染 + 关闭
            let markdown = client::render_markdown(buf);
            if let Some(card_id) = &buf.card_message_id {
                let r =
                    client::patch_card(&self.http, self.creds.as_ref(), card_id, &markdown).await;
                streams.remove(chat_id);
                return r.map_err(|e| self.maybe_invalidate(e));
            }
            // 无卡片（从未发过 delta，仅 end）：退化为文本
            let text = std::mem::take(&mut buf.text);
            streams.remove(chat_id);
            if !text.is_empty() {
                return client::send_text(&self.http, self.creds.as_ref(), chat_id, &text)
                    .await
                    .map_err(|e| self.maybe_invalidate(e));
            }
            return Ok(());
        }

        // 首个 delta：创建卡片
        if buf.card_message_id.is_none() {
            let markdown = client::render_markdown(buf);
            match client::send_card(&self.http, self.creds.as_ref(), chat_id, &markdown).await {
                Ok(id) if !id.is_empty() => {
                    buf.card_message_id = Some(id);
                    buf.last_edit = Some(now);
                }
                Ok(_) => {
                    // 返回了空 message_id：退化为文本
                    let text = buf.text.clone();
                    return client::send_text(&self.http, self.creds.as_ref(), chat_id, &text)
                        .await
                        .map_err(|e| self.maybe_invalidate(e));
                }
                Err(e) => return Err(self.maybe_invalidate(e)),
            }
            return Ok(());
        }

        // 节流更新
        let should_edit = buf
            .last_edit
            .map(|t| now.duration_since(t) >= STREAM_EDIT_INTERVAL)
            .unwrap_or(true);
        if should_edit {
            let markdown = client::render_markdown(buf);
            if let Some(card_id) = buf.card_message_id.clone() {
                client::patch_card(&self.http, self.creds.as_ref(), &card_id, &markdown)
                    .await
                    .map_err(|e| self.maybe_invalidate(e))?;
                buf.last_edit = Some(now);
            }
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

impl FeishuChannel {
    /// 若 API 报 token 无效（401 或业务码），作废缓存 token 以便下次刷新。
    fn maybe_invalidate(&self, e: GatewayError) -> GatewayError {
        match &e {
            GatewayError::Unauthorized => {
                let creds = self.creds.clone();
                tokio::spawn(async move { creds.invalidate().await });
            }
            GatewayError::Api { code, .. } if *code as i64 == INVALID_TOKEN_CODE => {
                let creds = self.creds.clone();
                tokio::spawn(async move { creds.invalidate().await });
            }
            _ => {}
        }
        e
    }
}

/// 飞书渠道工厂。
pub struct FeishuFactory;

impl ChannelFactory for FeishuFactory {
    fn descriptor(&self) -> &'static ChannelDescriptor {
        descriptor()
    }

    fn build(&self, deps: &ChannelDeps) -> Arc<dyn Channel> {
        Arc::new(FeishuChannel::new(
            deps.http.clone(),
            deps.secrets.clone(),
            deps.event_bus.clone(),
        ))
    }
}
