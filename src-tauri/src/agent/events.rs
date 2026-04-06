//! Runtime Events — 运行期共享事件协议
//!
//! 该文件只定义运行过程中的共享事件契约，不承载业务逻辑。
//! 它与 `spec.rs` 平级：
//! - `spec.rs` 定义 run 前后的输入输出契约
//! - `events.rs` 定义 run 过程中的事件与状态契约
//!
//! 这里的命名按四层语义区分：
//! - `ProviderEvent`：Provider -> AgentRunner 的标准化流事件
//! - `ProviderUsage`：Provider 单次响应的原始 token 使用量
//! - `AgentEvent`：Loop / Runner / Tool / Spawn 的内部观测事件

//! - `UserVisiblePhase`：前端或 Channel 可见的简化状态

use serde::{Deserialize, Serialize};

/// Provider → AgentRunner 的标准化流事件
/// Provider 实现必须在内部缓冲 tool call arguments delta，
/// 仅在参数 JSON 完整后才发出 ToolCallReady 事件
#[derive(Debug, Clone)]
pub enum ProviderEvent {
    /// 文本增量
    TextDelta { text: String },
    /// 工具调用请求（完整参数）
    ToolCallReady {
        id: String,
        name: String,
        arguments_json: serde_json::Value,
    },
    /// 流式响应结束
    StreamFinished {
        stop_reason: String,
        usage: ProviderUsage,
    },
}

/// Provider 单次响应的原始 Token 使用统计
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ProviderUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

/// Runtime 内部观测事件（日志/审计/指标）
#[derive(Debug, Clone)]
pub enum AgentEvent {
    /// Run 开始
    RunStarted {
        session_id: String,
        request_id: String,
    },
    /// Session 解析完成
    SessionLoaded { session_id: String },
    /// 控制命令被拦截
    ControlCommandIntercepted { name: String },
    /// 上下文构建完成
    ContextPrepared { fragments: usize },
    /// 收到文本增量
    ModelTextDeltaObserved { len: usize },
    /// 收到工具调用
    ModelToolCallObserved { name: String },
    /// 工具开始执行
    ToolExecutionStarted { name: String },
    /// 工具执行完成
    ToolExecutionFinished { name: String, success: bool },
    /// Steering 补充指令已注入当前工具轮次（软打断）
    SteeringMessageInjected { count: usize },
    /// Run 正常完成
    RunCompleted,
    /// Run 被取消
    RunCancelled,
    /// Run 失败
    RunFailed { message: String },
}

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
}

impl std::fmt::Display for UserVisiblePhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserVisiblePhase::Thinking => write!(f, "thinking"),
            UserVisiblePhase::UsingTools => write!(f, "using_tools"),
            UserVisiblePhase::Streaming => write!(f, "streaming"),
        }
    }
}
