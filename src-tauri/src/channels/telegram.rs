use super::traits::{Channel, ChannelMessage};
use crate::error::AppResult;

/// Telegram Bot Channel（Phase 1 后期）
#[allow(dead_code)]
pub struct TelegramChannel {
    bot_token: String,
}

impl TelegramChannel {
    pub fn new(bot_token: String) -> Self {
        Self { bot_token }
    }
}

#[async_trait::async_trait]
impl Channel for TelegramChannel {
    fn id(&self) -> &str {
        "telegram"
    }

    async fn start(&self) -> AppResult<()> {
        todo!("实现 Telegram Bot 轮询/Webhook 启动")
    }

    async fn stop(&self) -> AppResult<()> {
        todo!("实现 Telegram Bot 停止")
    }

    async fn send(&self, _message: ChannelMessage) -> AppResult<()> {
        todo!("实现 Telegram 消息发送")
    }
}
