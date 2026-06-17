//! 电报 REST 辅助（供外部 / 测试使用）。

use super::token::TelegramCredentials;
use crate::services::gateway::GatewayError;

const TELEGRAM_API_BASE: &str = "https://api.telegram.org";

/// 发送文本消息（非 Channel trait 路径，供直接调用）。
#[allow(dead_code)]
pub async fn send_message_raw(
    http: &reqwest::Client,
    creds: &TelegramCredentials,
    chat_id: &str,
    content: &str,
) -> Result<(), GatewayError> {
    let token = creds.get_token().await?;
    let body = serde_json::json!({
        "chat_id": chat_id,
        "text": content,
    });
    let resp = http
        .post(format!("{TELEGRAM_API_BASE}/bot{token}/sendMessage"))
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
