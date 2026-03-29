use super::traits::{Channel, ChannelMessage};
use crate::error::AppResult;

/// 飞书 Bot Channel（Phase 2）
#[allow(dead_code)]
pub struct FeishuChannel {
    app_id: String,
    app_secret: String,
}

impl FeishuChannel {
    pub fn new(app_id: String, app_secret: String) -> Self {
        Self { app_id, app_secret }
    }
}

#[async_trait::async_trait]
impl Channel for FeishuChannel {
    fn id(&self) -> &str {
        "feishu"
    }

    async fn start(&self) -> AppResult<()> {
        todo!("实现飞书 Bot 启动")
    }

    async fn stop(&self) -> AppResult<()> {
        todo!("实现飞书 Bot 停止")
    }

    async fn send(&self, _message: ChannelMessage) -> AppResult<()> {
        todo!("实现飞书消息发送")
    }
}
