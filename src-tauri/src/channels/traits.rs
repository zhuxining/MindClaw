use crate::error::AppResult;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelMessage {
    pub sender: String,
    pub content: String,
    pub metadata: std::collections::HashMap<String, String>,
    pub timestamp: i64,
}

/// Channel trait：所有消息渠道实现此接口
#[async_trait::async_trait]
pub trait Channel: Send + Sync {
    fn id(&self) -> &str;
    async fn start(&self) -> AppResult<()>;
    async fn stop(&self) -> AppResult<()>;
    async fn send(&self, message: ChannelMessage) -> AppResult<()>;
}

/// 启动所有 Channel，返回消息接收通道
pub async fn start_channels(
    channels: Vec<Box<dyn Channel>>,
) -> AppResult<tokio::sync::mpsc::Receiver<ChannelMessage>> {
    let (_tx, rx) = tokio::sync::mpsc::channel(256);
    for channel in channels {
        channel.start().await?;
    }
    Ok(rx)
}
