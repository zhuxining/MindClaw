//! 飞书 WebSocket 长连接（pbbp2.proto 帧）。
//!
//! 协议参考 zeroclaw `lark.rs`：
//! - POST /callback/ws/endpoint → (wss_url, ClientConfig{PingInterval})
//! - 帧为 protobuf pbbp2：method=0 CONTROL(ping/pong)，method=1 DATA(events)
//! - 连接后立即发 ping，按 PingInterval（默认 120s）周期 ping
//! - DATA 帧需在 3s 内 ACK；支持 sum/seq 分片重组
//! - 心跳超时 300s → 重连
//!
//! 断开后由 `run_with_reconnect` 指数退避重连。

use super::client::get_ws_endpoint;
use super::converter::{to_inbound, LarkEvent};
use crate::services::channels::MessageBus;
use crate::services::event_bus::{EventBus, RuntimeEvent};
use crate::services::gateway::GatewayError;
use futures_util::{SinkExt, StreamExt};
use prost::Message as ProstMessage;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio_tungstenite::tungstenite::Message as WsMsg;

const WS_HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(300);
const MAX_BACKOFF_SECS: u64 = 60;

/// 分片缓存条目：(slots, created_at)
type FragmentCache = (Vec<Option<Vec<u8>>>, Instant);

/// pbbp2.proto 帧头键值对。
#[derive(Clone, PartialEq, prost::Message)]
struct PbHeader {
    #[prost(string, tag = "1")]
    key: String,
    #[prost(string, tag = "2")]
    value: String,
}

/// pbbp2.proto 帧。method=0 → CONTROL，method=1 → DATA。
#[derive(Clone, prost::Message)]
struct PbFrame {
    #[prost(uint64, tag = "1")]
    seq_id: u64,
    #[prost(uint64, tag = "2")]
    #[allow(dead_code)]
    log_id: u64,
    #[prost(int32, tag = "3")]
    service: i32,
    #[prost(int32, tag = "4")]
    method: i32,
    #[prost(message, repeated, tag = "5")]
    headers: Vec<PbHeader>,
    #[prost(bytes = "vec", optional, tag = "8")]
    payload: Option<Vec<u8>>,
}

impl PbFrame {
    fn header_value(&self, key: &str) -> &str {
        self.headers
            .iter()
            .find(|h| h.key == key)
            .map(|h| h.value.as_str())
            .unwrap_or("")
    }
}

/// 带 reconnect 的飞书长连接运行循环。在 `Channel::start` 内调用。
pub async fn run_with_reconnect(
    http: &reqwest::Client,
    app_id: &str,
    app_secret: &str,
    bus: &MessageBus,
    event_bus: &EventBus,
    cancel: tokio_util::sync::CancellationToken,
) -> Result<(), GatewayError> {
    let mut backoff = 1u64;
    loop {
        if cancel.is_cancelled() {
            return Ok(());
        }
        let result = listen_once(http, app_id, app_secret, bus, event_bus, cancel.clone()).await;
        if cancel.is_cancelled() {
            return Ok(());
        }
        match result {
            Ok(()) => {
                // 连接正常关闭，短退避后重连
                event_bus.publish(RuntimeEvent::ChannelReconnecting {
                    channel: "feishu".into(),
                    reason: "connection closed".into(),
                });
                backoff = 1;
                tokio::time::sleep(Duration::from_secs(backoff)).await;
            }
            Err(e) => {
                event_bus.publish(RuntimeEvent::ChannelReconnecting {
                    channel: "feishu".into(),
                    reason: e.to_string(),
                });
                eprintln!("feishu WS error: {e}, reconnecting in {backoff}s");
                tokio::time::sleep(Duration::from_secs(backoff)).await;
                backoff = (backoff * 2).min(MAX_BACKOFF_SECS);
            }
        }
    }
}

/// 单次连接的事件循环，连接关闭或致命错误时返回。
async fn listen_once(
    http: &reqwest::Client,
    app_id: &str,
    app_secret: &str,
    bus: &MessageBus,
    _event_bus: &EventBus,
    cancel: tokio_util::sync::CancellationToken,
) -> Result<(), GatewayError> {
    let (wss_url, client_config) = get_ws_endpoint(http, app_id, app_secret).await?;
    let service_id = wss_url
        .split('?')
        .nth(1)
        .and_then(|qs| {
            qs.split('&')
                .find(|kv| kv.starts_with("service_id="))
                .and_then(|kv| kv.split('=').nth(1))
                .and_then(|v| v.parse::<i32>().ok())
        })
        .unwrap_or(0);

    let (ws_stream, _) = tokio_tungstenite::connect_async(&wss_url)
        .await
        .map_err(|e| GatewayError::Network(format!("WS 连接失败: {e}")))?;
    eprintln!("[feishu] WS connected, service_id={service_id}");
    let (mut write, mut read) = ws_stream.split();

    let mut ping_secs = client_config.ping_interval.unwrap_or(120).max(10);
    let mut hb_interval = tokio::time::interval(Duration::from_secs(ping_secs));
    let mut timeout_check = tokio::time::interval(Duration::from_secs(10));
    hb_interval.tick().await; // 消费立即 tick

    let mut seq: u64 = 0;
    let mut last_recv = Instant::now();

    // 立即发初始 ping（对齐官方 SDK）
    seq = seq.wrapping_add(1);
    let initial_ping = PbFrame {
        seq_id: seq,
        log_id: 0,
        service: service_id,
        method: 0,
        headers: vec![PbHeader {
            key: "type".into(),
            value: "ping".into(),
        }],
        payload: None,
    };
    if write
        .send(WsMsg::Binary(initial_ping.encode_to_vec()))
        .await
        .is_err()
    {
        return Err(GatewayError::Network("初始 ping 失败".into()));
    }

    // 分片重组缓存：message_id → (slots, created_at)
    let mut frag_cache: HashMap<String, FragmentCache> = HashMap::new();

    loop {
        tokio::select! {
            biased;
            _ = cancel.cancelled() => return Ok(()),

            _ = hb_interval.tick() => {
                seq = seq.wrapping_add(1);
                let ping = PbFrame {
                    seq_id: seq, log_id: 0, service: service_id, method: 0,
                    headers: vec![PbHeader { key: "type".into(), value: "ping".into() }],
                    payload: None,
                };
                if write.send(WsMsg::Binary(ping.encode_to_vec())).await.is_err() {
                    return Err(GatewayError::Network("ping 失败，重连".into()));
                }
                // GC 过期分片
                let cutoff = Instant::now().checked_sub(Duration::from_secs(300)).unwrap_or(Instant::now());
                frag_cache.retain(|_, (_, ts)| *ts > cutoff);
            }

            _ = timeout_check.tick() => {
                if last_recv.elapsed() > WS_HEARTBEAT_TIMEOUT {
                    return Err(GatewayError::Network("心跳超时，重连".into()));
                }
            }

            msg = read.next() => {
                let raw = match msg {
                    Some(Ok(ws_msg)) => {
                        match ws_msg {
                            WsMsg::Binary(b) => { last_recv = Instant::now(); b }
                            WsMsg::Ping(d) => { let _ = write.send(WsMsg::Pong(d)).await; continue; }
                            WsMsg::Close(_) => return Ok(()),
                            WsMsg::Pong(_) => { last_recv = Instant::now(); continue; }
                            _ => continue,
                        }
                    }
                    None => return Ok(()),
                    Some(Err(e)) => return Err(GatewayError::Network(format!("WS 读错误: {e}"))),
                };

                let frame = match PbFrame::decode(&raw[..]) {
                    Ok(f) => f,
                    Err(e) => {
                        eprintln!("feishu proto decode error: {e}");
                        continue;
                    }
                };

                // CONTROL 帧
                if frame.method == 0 {
                    if frame.header_value("type") == "pong" {
                        if let Some(p) = &frame.payload {
                            if let Ok(cfg) = serde_json::from_slice::<super::client::WsClientConfig>(p) {
                                if let Some(secs) = cfg.ping_interval {
                                    let secs = secs.max(10);
                                    if secs != ping_secs {
                                        ping_secs = secs;
                                        hb_interval = tokio::time::interval(Duration::from_secs(ping_secs));
                                    }
                                }
                            }
                        }
                    }
                    continue;
                }

                // DATA 帧
                let msg_type = frame.header_value("type").to_string();
                let msg_id = frame.header_value("message_id").to_string();
                let sum = frame.header_value("sum").parse::<usize>().unwrap_or(1);
                let seq_num = frame.header_value("seq").parse::<usize>().unwrap_or(0);

                // 立即 ACK（飞书要求 3s 内）
                {
                    let mut ack = frame.clone();
                    ack.payload = Some(br#"{"code":200,"headers":{},"data":[]}"#.to_vec());
                    ack.headers.push(PbHeader { key: "biz_rt".into(), value: "0".into() });
                    let _ = write.send(WsMsg::Binary(ack.encode_to_vec())).await;
                }

                // 分片重组
                let sum = if sum == 0 { 1 } else { sum };
                let payload: Vec<u8> = if sum == 1 || msg_id.is_empty() || seq_num >= sum {
                    frame.payload.clone().unwrap_or_default()
                } else {
                    let entry = frag_cache
                        .entry(msg_id.clone())
                        .or_insert_with(|| (vec![None; sum], Instant::now()));
                    if entry.0.len() != sum { *entry = (vec![None; sum], Instant::now()); }
                    entry.0[seq_num] = frame.payload.clone();
                    if entry.0.iter().all(|s| s.is_some()) {
                        let full: Vec<u8> = entry.0.iter()
                            .flat_map(|s| s.as_deref().unwrap_or(&[]))
                            .copied().collect();
                        frag_cache.remove(&msg_id);
                        full
                    } else { continue; }
                };
                if msg_type != "event" { continue; }

                let event: LarkEvent = match serde_json::from_slice(&payload) {
                    Ok(e) => e,
                    Err(e) => { eprintln!("feishu event JSON 解析失败: {e}"); continue; }
                };
                if event.header.event_type != "im.message.receive_v1" {
                    continue;
                }

                if let Some(inbound) = to_inbound(&event.event) {
                    eprintln!("[feishu] msg received: chat_id={}, sender={}, text={:.60}",
                        inbound.chat_id, inbound.sender_id, inbound.content);
                    bus.publish_inbound(inbound);
                }
            }
        }
    }
}
