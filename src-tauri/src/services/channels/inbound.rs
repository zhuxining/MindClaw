use crate::services::core::ChannelMessage;
use crate::services::gateway::GatewayError;

/// 渠道消息接收方式抽象。
///
/// 不同渠道的消息入口模型不同：
/// - polling：主动轮询（飞书）
/// - long polling：长轮询（Telegram）
/// - stream：持续事件流（MCP event）
/// - webhook：被动接收（需本机或 relay HTTPS endpoint）
/// - manual：手动拉取（CLI input）
#[allow(dead_code)]
pub enum InboundDriver {
    Polling { interval_secs: u64 },
    LongPolling,
    Stream,
    Webhook,
    Manual,
}

/// 可被 InboundDriver 驱动的渠道。
///
/// 实现此 trait 的 Channel 可以通过 InboundDriver 自动接收消息，
/// 而不需要上层手动调用 `poll_messages()`。
#[allow(dead_code)]
#[async_trait::async_trait]
pub trait InboundChannel: super::Channel {
    /// 返回该渠道支持的接收方式。
    fn inbound_strategy(&self) -> InboundDriver;

    /// 执行一次消息接收循环（由 driver 调用）。
    async fn receive_batch(&self) -> Result<Vec<ChannelMessage>, GatewayError>;
}
