//! Runtime Events — 运行期共享事件协议
//!
//! 该文件只定义运行过程中的共享事件契约，不承载业务逻辑。
//!
//! 当前保留：
//! - `UserVisiblePhase`：前端或 Channel 可见的简化状态
//!
//! 已移除（由 rig 内建 tracing spans 替代）：
//! - `ProviderEvent`、`ProviderUsage`：LLM 流事件
//! - `AgentEvent`、`LoopPhase`：内部观测事件

use serde::{Deserialize, Serialize};

/// 前端/Channel 可见的简化状态（由 OutboundPayload::Status 携带）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserVisiblePhase {
    /// 组装上下文 + 等待 LLM 首 token
    Thinking,
    /// 执行工具中
    UsingTools,
    /// 正在输出文本（前端收到首个 Chunk 时自动进入）
    Streaming,
    /// Run 成功完成
    Completed,
    /// Run 被取消
    Cancelled,
    /// Run 出错
    Error,
}

impl std::fmt::Display for UserVisiblePhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserVisiblePhase::Thinking => write!(f, "thinking"),
            UserVisiblePhase::UsingTools => write!(f, "using_tools"),
            UserVisiblePhase::Streaming => write!(f, "streaming"),
            UserVisiblePhase::Completed => write!(f, "completed"),
            UserVisiblePhase::Cancelled => write!(f, "cancelled"),
            UserVisiblePhase::Error => write!(f, "error"),
        }
    }
}
