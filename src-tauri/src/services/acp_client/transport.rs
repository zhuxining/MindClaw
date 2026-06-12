use crate::services::core::AgentResponse;
use async_trait::async_trait;

/// ACP 传输层抽象。
///
/// 每种传输实现负责与 ACP Server 建立连接、发送 prompt 并返回响应。
#[allow(dead_code)]
#[async_trait]
pub trait Transport: Send + Sync {
    /// 向 ACP Server 发送 prompt 并等待完整响应。
    async fn dispatch(&self, prompt: String, request_id: String) -> AgentResponse;

    /// 测试与 ACP Server 的连接是否可用。
    async fn test_connection(&self) -> Result<(), String>;
}
