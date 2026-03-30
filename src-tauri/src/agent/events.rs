use serde::{Deserialize, Serialize};

/// Provider → AgentLoop 的事件流
/// Provider 实现必须在内部缓冲 tool call arguments delta，
/// 仅在参数 JSON 完整后才发出 ToolCall 事件
#[derive(Debug, Clone)]
pub enum ProviderEvent {
    /// 文本增量
    TextDelta { text: String },
    /// 工具调用请求（完整参数）
    ToolCall {
        id: String,
        name: String,
        arguments_json: serde_json::Value,
    },
    /// 流式响应结束
    Finished {
        stop_reason: String,
        usage: UsageStats,
    },
}

/// Token 使用统计
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct UsageStats {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

/// AgentLoop 内部观测事件（日志/审计/指标）
#[derive(Debug, Clone)]
pub enum AgentEvent {
    /// Run 开始
    RunStarted {
        session_id: String,
        request_id: String,
    },
    /// Session 解析完成
    SessionResolved { session_id: String },
    /// Agent Command 被拦截
    CommandIntercepted { name: String },
    /// 上下文构建完成
    ContextBuilt { fragments: usize },
    /// 收到文本增量
    ProviderTextDelta { len: usize },
    /// 收到工具调用
    ProviderToolCall { name: String },
    /// 工具开始执行
    ToolStarted { name: String },
    /// 工具执行完成
    ToolFinished { name: String, success: bool },
    /// Steering 补充指令已注入当前工具轮次（软打断）
    SteeringInjected { count: usize },
    /// Run 正常完成
    RunCompleted,
    /// Run 被取消
    RunCancelled,
    /// Run 失败
    RunFailed { message: String },
}

/// 内部 run 阶段（仅供 AgentEvent/Observer 使用，不暴露给前端）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunPhase {
    /// 已入队等待
    Queued,
    /// 正在解析 Session
    ResolvingSession,
    /// 检查是否命中 Agent Command
    CheckingAgentCommand,
    /// 正在构建上下文
    BuildingContext,
    /// 正在流式输出 assistant 响应
    StreamingAssistant,
    /// 正在执行工具
    ExecutingTools,
    /// 正在持久化对话回合
    PersistingTurn,
    /// 已完成
    Completed,
    /// 已取消
    Cancelled,
    /// 已失败
    Failed,
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
