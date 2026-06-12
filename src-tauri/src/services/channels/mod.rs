pub mod inbound;
pub mod registry;

use crate::services::core::ChannelMessage;
use crate::services::gateway::GatewayError;

pub use registry::ChannelRegistry;

/// 凭证管理器 trait
///
/// 每个渠道自行管理其凭证生命周期（Token 刷新、存储等）。
#[allow(dead_code)]
pub trait CredentialsManager: Send + Sync {
    /// 设置凭证（渠道特有格式，由实现者解析）
    fn set_credentials(&self, credentials: serde_json::Value) -> Result<(), GatewayError>;

    /// 清除凭证
    fn clear_credentials(&self) -> Result<(), GatewayError>;

    /// 检查是否已配置凭证
    fn has_credentials(&self) -> bool;

    /// 测试连接是否正常
    fn test_connection(&self) -> Result<(), GatewayError>;
}

/// 消息渠道 trait
///
/// 每个消息渠道实现此 trait，负责：
/// 1. 从渠道拉取消息并转换为统一 `ChannelMessage` 格式
/// 2. 将回复消息发送回渠道
/// 3. 管理该渠道的凭证
pub trait Channel: Send + Sync {
    /// 返回渠道唯一标识名称（如 "feishu"、"dingtalk"）
    fn channel_name(&self) -> &str;

    /// 拉取消息
    ///
    /// 从渠道 API 拉取新消息，转换为 `ChannelMessage` 列表。
    /// `page_token` 用于分页，`page_size` 控制每页数量。
    fn poll_messages(
        &self,
        page_size: i32,
        page_token: Option<&str>,
    ) -> Result<(Vec<ChannelMessage>, Option<String>), GatewayError>;

    /// 发送消息到渠道
    fn send_message(
        &self,
        conversation_id: &str,
        content: &str,
        reply_to: Option<&str>,
    ) -> Result<(), GatewayError>;

    /// 获取凭证管理器引用
    fn credentials(&self) -> &dyn CredentialsManager;
}
