//! 渠道自描述元数据。替代 nanobot 的 `default_config()` + 硬编码 registry，
//! 让前端可据 `credential_schema` 动态渲染设置表单。

use serde::{Deserialize, Serialize};

/// 渠道入口（ingress）传输模型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InboundKind {
    /// 持久长连接（飞书 WSS）
    LongConnection,
    /// 长轮询（Telegram getUpdates timeout）
    LongPolling,
    /// 短轮询
    Polling,
    /// 被动 webhook（需 HTTPS endpoint）
    Webhook,
    /// 手动拉取（CLI input）
    Manual,
}

/// 渠道能力声明。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Capabilities {
    /// 是否支持流式增量输出（send_delta）
    pub streaming: bool,
    /// 是否支持推理过程展示
    pub reasoning: bool,
    /// 是否支持文件编辑事件
    pub file_edit: bool,
    /// 是否支持回复引用
    pub reply: bool,
}

/// 渠道静态描述符。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelDescriptor {
    /// 渠道唯一标识（如 "feishu" / "telegram"）
    pub id: &'static str,
    /// 展示名称
    pub display_name: &'static str,
    /// 入口传输模型
    pub inbound: InboundKind,
    /// 凭证 JSON Schema（前端动态渲染表单）
    pub credential_schema: serde_json::Value,
    /// 能力声明
    pub capabilities: Capabilities,
}
