//! 飞书 REST API 客户端：token、WS endpoint、发送/更新卡片消息。

use super::token::FeishuCredentials;
use crate::services::gateway::GatewayError;

const FEISHU_API_BASE: &str = "https://open.feishu.cn/open-apis";
const FEISHU_WS_BASE: &str = "https://open.feishu.cn";

/// 卡片 markdown 内容上限（~30KB 限制留余量）。
pub const CARD_MARKDOWN_MAX_BYTES: usize = 28_000;

/// POST /callback/ws/endpoint 响应。
#[derive(Debug, serde::Deserialize)]
struct WsEndpointResp {
    code: i32,
    #[serde(default)]
    msg: Option<String>,
    #[serde(default)]
    data: Option<WsEndpoint>,
}

#[derive(Debug, serde::Deserialize)]
struct WsEndpoint {
    #[serde(rename = "URL")]
    url: String,
    #[serde(rename = "ClientConfig")]
    #[serde(default)]
    client_config: Option<WsClientConfig>,
}

#[derive(Debug, serde::Deserialize, Default, Clone)]
pub struct WsClientConfig {
    #[serde(rename = "PingInterval")]
    #[serde(default)]
    pub ping_interval: Option<u64>,
}

/// 获取 WS 长连接端点 URL + 客户端配置。
pub async fn get_ws_endpoint(
    http: &reqwest::Client,
    app_id: &str,
    app_secret: &str,
) -> Result<(String, WsClientConfig), GatewayError> {
    let resp = http
        .post(format!("{FEISHU_WS_BASE}/callback/ws/endpoint"))
        .header("locale", "zh")
        .header("Content-Type", "application/json; charset=utf-8")
        .json(&serde_json::json!({ "AppID": app_id, "AppSecret": app_secret }))
        .send()
        .await
        .map_err(|_| GatewayError::Network("获取 WS endpoint 网络错误".into()))?
        .json::<WsEndpointResp>()
        .await
        .map_err(|e| GatewayError::Network(format!("解析 WS endpoint 失败: {e}")))?;

    if resp.code != 0 {
        return Err(GatewayError::Network(format!(
            "WS endpoint 失败: code={} msg={}",
            resp.code,
            resp.msg.as_deref().unwrap_or("(none)")
        )));
    }
    let ep = resp
        .data
        .ok_or_else(|| GatewayError::Network("WS endpoint: empty data".into()))?;
    Ok((ep.url, ep.client_config.unwrap_or_default()))
}

/// 统一发送 POST 并校验业务码。
async fn post_json(
    http: &reqwest::Client,
    url: &str,
    token: &str,
    body: &serde_json::Value,
) -> Result<serde_json::Value, GatewayError> {
    let resp = http
        .post(url)
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "application/json; charset=utf-8")
        .json(body)
        .send()
        .await
        .map_err(|_| GatewayError::Network("发送请求网络错误".into()))?;
    let status = resp.status();
    let raw = resp.text().await.unwrap_or_default();
    let parsed: serde_json::Value =
        serde_json::from_str(&raw).unwrap_or_else(|_| serde_json::json!({ "raw": raw }));

    // 401 → token 失效
    if status == reqwest::StatusCode::UNAUTHORIZED {
        return Err(GatewayError::Unauthorized);
    }
    let code = parsed.get("code").and_then(|c| c.as_i64()).unwrap_or(-1);
    if code != 0 {
        return Err(GatewayError::Api {
            code: code as i32,
            msg: parsed
                .get("msg")
                .and_then(|m| m.as_str())
                .unwrap_or("未知错误")
                .to_string(),
        });
    }
    Ok(parsed)
}

/// 构建 Card JSON 2.0 单 markdown 元素内容。
fn build_card_content(markdown: &str) -> String {
    serde_json::json!({
        "schema": "2.0",
        "body": { "elements": [{ "tag": "markdown", "content": markdown }] }
    })
    .to_string()
}

/// 截断到卡片字节上限（按 UTF-8 边界）。
fn truncate_card_markdown(text: &str) -> String {
    if text.len() <= CARD_MARKDOWN_MAX_BYTES {
        return text.to_string();
    }
    let suffix = "\n\n…_(updating)_";
    let budget = CARD_MARKDOWN_MAX_BYTES.saturating_sub(suffix.len());
    let mut end = 0;
    for (idx, ch) in text.char_indices() {
        if idx + ch.len_utf8() > budget {
            break;
        }
        end = idx + ch.len_utf8();
    }
    format!("{}{suffix}", &text[..end])
}

/// 发送交互卡片消息，返回 message_id。
pub async fn send_card(
    http: &reqwest::Client,
    creds: &FeishuCredentials,
    chat_id: &str,
    markdown: &str,
) -> Result<String, GatewayError> {
    let token = creds.get_token().await?;
    let body = serde_json::json!({
        "receive_id": chat_id,
        "msg_type": "interactive",
        "content": build_card_content(markdown),
    });
    let resp = post_json(
        http,
        &format!("{FEISHU_API_BASE}/im/v1/messages?receive_id_type=chat_id"),
        &token,
        &body,
    )
    .await?;
    let message_id = resp
        .pointer("/data/message_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    Ok(message_id)
}

/// PATCH 更新已有卡片内容（流式增量）。
pub async fn patch_card(
    http: &reqwest::Client,
    creds: &FeishuCredentials,
    message_id: &str,
    markdown: &str,
) -> Result<(), GatewayError> {
    let token = creds.get_token().await?;
    let body = serde_json::json!({
        "msg_type": "interactive",
        "content": build_card_content(markdown),
    });
    let _ = post_json(
        http,
        &format!("{FEISHU_API_BASE}/im/v1/messages/{message_id}"),
        &token,
        &body,
    )
    .await?;
    Ok(())
}

/// 发送纯文本消息（Final 回复）。
pub async fn send_text(
    http: &reqwest::Client,
    creds: &FeishuCredentials,
    chat_id: &str,
    content: &str,
) -> Result<(), GatewayError> {
    let token = creds.get_token().await?;
    let body = serde_json::json!({
        "receive_id": chat_id,
        "msg_type": "text",
        "content": serde_json::json!({ "text": content }).to_string(),
    });
    post_json(
        http,
        &format!("{FEISHU_API_BASE}/im/v1/messages?receive_id_type=chat_id"),
        &token,
        &body,
    )
    .await
    .map(|_| ())
}

/// 流式 buffer：累积文本 + 已发送卡片 message_id + 上次 PATCH 时间。
#[derive(Debug, Default, Clone)]
pub struct StreamBuf {
    pub text: String,
    pub card_message_id: Option<String>,
    pub last_edit: Option<std::time::Instant>,
}

/// 给定累积文本返回截断后用于渲染的内容。
pub fn render_markdown(buf: &StreamBuf) -> String {
    truncate_card_markdown(&buf.text)
}
