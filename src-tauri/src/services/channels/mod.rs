//! 消息渠道 trait 与共享抽象（参考 nanobot channels 架构）。
//!
//! 核心分层：
//! - [`Channel`]：最小生命周期 + 归一化消息边界（async）。
//! - [`CredentialsManager`]：渠道凭证生命周期（async，走 [`SecretStore`]）。
//! - [`ChannelDescriptor`] / [`InboundKind`]：渠道自描述元数据（替代硬编码 registry）。
//! - `MessageBus`（`bus.rs`）：inbound/outbound 双 mpsc 队列。
//! - `ChannelManager`（`manager.rs`）：生命周期编排 + outbound 路由/重试/coalescing。
//! - `ChannelRegistry`（`registry.rs`）：factory + descriptor 注册。

pub mod bus;
pub mod descriptor;
pub mod feishu;
pub mod manager;
pub mod registry;
pub mod runtime;
pub mod telegram;

pub use bus::MessageBus;
pub use descriptor::{Capabilities, ChannelDescriptor, InboundKind};
pub use manager::ChannelManager;
pub use registry::{ChannelDeps, ChannelFactory, ChannelRegistry};

use crate::services::core::{OutboundKind, OutboundMessage, SecretStore};
use crate::services::gateway::GatewayError;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

/// 凭证管理器 trait（async）。
///
/// 每个渠道自行解析凭证格式并经 [`SecretStore`] 加密持久化。
#[async_trait::async_trait]
pub trait CredentialsManager: Send + Sync {
    /// 设置凭证（渠道特有格式，由实现者解析后存入 SecretStore）。
    async fn set_credentials(
        &self,
        credentials: serde_json::Value,
        store: &dyn SecretStore,
    ) -> Result<(), GatewayError>;

    /// 清除凭证。
    #[allow(dead_code)]
    async fn clear_credentials(&self, store: &dyn SecretStore) -> Result<(), GatewayError>;

    /// 检查是否已配置凭证（查询 SecretStore，而非内存）。
    async fn has_credentials(&self, store: &dyn SecretStore) -> bool;

    /// 测试连接是否正常。
    async fn test_connection(&self) -> Result<(), GatewayError>;
}

/// 消息渠道 trait（async）。
///
/// 实现者负责：建立平台连接（`start`）、接收消息归一化为 [`crate::services::core::InboundMessage`]
/// 投递到 `MessageBus`、将 `OutboundMessage` 渲染回平台。
#[async_trait::async_trait]
pub trait Channel: Send + Sync {
    /// 返回渠道静态描述符。
    fn descriptor(&self) -> &'static ChannelDescriptor;

    /// 启动渠道运行时（长连接 / long-poll / webhook server）。
    ///
    /// 实现应在 `cancel` 取消时优雅退出；连接断开时由实现自行重连。
    async fn start(
        &self,
        bus: Arc<MessageBus>,
        cancel: CancellationToken,
    ) -> Result<(), GatewayError>;

    /// 停止渠道运行时。
    async fn stop(&self) -> Result<(), GatewayError>;

    /// 发送完整 / 进度类消息（`OutboundKind::Final` / `Progress` / `TurnEnd`）。
    async fn send(&self, msg: &OutboundMessage) -> Result<(), GatewayError>;

    /// 发送流式增量（`StreamDelta` / `ReasoningDelta`）。
    ///
    /// 默认空实现；支持流式的渠道覆写。`kind` 携带 `stream_id` 与 `end` 标记。
    async fn send_delta(
        &self,
        _chat_id: &str,
        _delta: &str,
        _kind: &OutboundKind,
    ) -> Result<(), GatewayError> {
        Ok(())
    }

    /// 是否支持流式输出（默认 false）。
    fn supports_streaming(&self) -> bool {
        false
    }

    /// 获取凭证管理器。
    fn credentials(&self) -> &dyn CredentialsManager;
}
