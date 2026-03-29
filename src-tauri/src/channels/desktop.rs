use super::traits::{Channel, ChannelMessage};
use crate::error::AppResult;

/// 桌面端 Channel：Tauri IPC ↔ ChannelMessage 桥接
pub struct DesktopChannel;

#[async_trait::async_trait]
impl Channel for DesktopChannel {
    fn id(&self) -> &str {
        "desktop"
    }

    async fn start(&self) -> AppResult<()> {
        todo!("实现桌面端 Channel 启动")
    }

    async fn stop(&self) -> AppResult<()> {
        Ok(())
    }

    async fn send(&self, _message: ChannelMessage) -> AppResult<()> {
        todo!("实现桌面端消息发送（Tauri emit）")
    }
}
