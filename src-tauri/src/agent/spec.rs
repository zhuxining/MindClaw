//! AgentRunSpec 与 AgentRunResult
//!
//! 声明式执行配置与结构化执行结果
//! 双层架构的核心数据类型

use crate::agent::messages::ToolChoice;
use crate::agent::messages::{ChatMessage, ToolSchema};
use crate::agent::retry::RetryMode;
use crate::agent::session::ToolTrace;
use serde::{Deserialize, Serialize};
use std::time::Duration;

// ============================================================================
// InvocationMode - 调用语义
// ============================================================================

/// 本次 run 的调用语义
///
/// 影响流式输出策略、结果交付方式、Session 持久化策略和 Hooks 实现选择。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum InvocationMode {
    /// 用户前台对话
    #[default]
    Interactive,
    /// 同步子代理，父 Agent 等待结果后回填为 ToolResult
    InlineChild,
    /// 后台代理，先返回 task_id，完成后单独通知
    Detached,
}

// ============================================================================
// AgentRunSpec - 声明式执行配置
// ============================================================================

/// 声明式 Agent 执行配置
///
/// 这是一个冻结的数据类，包含执行引擎所需的每一个参数。
/// 由于使用了不可变设计，该规范在构建后既节省内存又可预测。
#[derive(Debug, Clone)]
pub struct AgentRunSpec {
    /// Run 唯一标识
    pub run_id: String,
    /// Session 标识
    pub session_id: String,
    /// Agent 标识
    pub agent_id: String,
    /// 调用语义（决定流式、结果交付、持久化策略）
    pub invocation: InvocationMode,

    /// 预组装的消息历史记录
    pub messages: Vec<ChatMessage>,

    /// 系统提示词（分离，便于缓存）
    pub system_prompt: String,

    /// 本次运行可用的工具 Schema
    pub tools: Vec<ToolSchema>,

    /// LLM 模型标识符（如 "claude-3-5-sonnet"）
    pub model: String,

    /// 工具调用周期的硬性上限（防止无限循环）
    /// 默认：8
    pub max_iterations: usize,

    /// 采样温度覆盖值
    pub temperature: Option<f32>,

    /// 每次 LLM 调用的最大补全 Token 数
    pub max_tokens: Option<usize>,

    /// 是否并行执行工具调用
    /// true：并发执行（默认）
    /// false：顺序执行
    pub parallel_tools: bool,

    /// 工具选择策略
    pub tool_choice: ToolChoice,

    /// 工具错误是否立即中止循环
    /// true：第一个工具错误立即中止
    /// false：错误作为消息返回，LLM 可重试（默认）
    pub fail_on_tool_error: bool,

    /// LLM 调用重试模式
    /// 默认：Standard（最多 3 次重试）
    pub retry_mode: RetryMode,
}

impl Default for AgentRunSpec {
    fn default() -> Self {
        Self {
            run_id: uuid::Uuid::new_v4().to_string(),
            session_id: "default".to_string(),
            agent_id: "default".to_string(),
            invocation: InvocationMode::Interactive,
            messages: Vec::new(),
            system_prompt: String::new(),
            tools: Vec::new(),
            model: "claude-3-5-sonnet".to_string(),
            max_iterations: 8,
            temperature: None,
            max_tokens: None,
            parallel_tools: true,
            tool_choice: ToolChoice::Auto,
            fail_on_tool_error: false,
            retry_mode: RetryMode::Standard,
        }
    }
}

impl AgentRunSpec {
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// 创建标准对话配置
    pub fn standard(
        system_prompt: String,
        messages: Vec<ChatMessage>,
        tools: Vec<ToolSchema>,
        model: String,
    ) -> Self {
        Self {
            run_id: uuid::Uuid::new_v4().to_string(),
            session_id: "default".to_string(),
            agent_id: "default".to_string(),
            invocation: InvocationMode::Interactive,
            system_prompt,
            messages,
            tools,
            model,
            max_iterations: 8,
            temperature: None,
            max_tokens: None,
            parallel_tools: true,
            tool_choice: ToolChoice::Auto,
            fail_on_tool_error: false,
            retry_mode: RetryMode::Standard,
        }
    }

    /// 使用应用配置创建 AgentRunSpec
    pub fn with_config(
        system_prompt: String,
        messages: Vec<ChatMessage>,
        tools: Vec<ToolSchema>,
        model: String,
        config: &crate::runtime::config::AppConfig,
    ) -> Self {
        Self {
            run_id: uuid::Uuid::new_v4().to_string(),
            session_id: "default".to_string(),
            agent_id: "default".to_string(),
            invocation: InvocationMode::Interactive,
            system_prompt,
            messages,
            tools,
            model,
            max_iterations: config.agent_max_iterations,
            temperature: config.agent_temperature,
            max_tokens: config.agent_max_tokens,
            parallel_tools: true,
            tool_choice: ToolChoice::Auto,
            fail_on_tool_error: false,
            retry_mode: RetryMode::Standard,
        }
    }

    /// 创建后台任务配置（SubAgent）
    pub fn background(
        system_prompt: String,
        task_description: String,
        tools: Vec<ToolSchema>,
        model: String,
    ) -> Self {
        Self {
            run_id: uuid::Uuid::new_v4().to_string(),
            session_id: "background".to_string(),
            agent_id: "background".to_string(),
            invocation: InvocationMode::Detached,
            system_prompt,
            messages: vec![ChatMessage::user(&task_description)],
            tools,
            model,
            max_iterations: 15,     // SubAgent 允许更多迭代
            temperature: Some(0.0), // 确定性输出
            max_tokens: Some(2000),
            parallel_tools: true,
            tool_choice: ToolChoice::Auto,
            fail_on_tool_error: true,          // 后台任务严格模式
            retry_mode: RetryMode::Persistent, // 后台任务持久重试
        }
    }

    /// 创建测试配置
    pub fn test(messages: Vec<ChatMessage>, tools: Vec<ToolSchema>) -> Self {
        Self {
            run_id: uuid::Uuid::new_v4().to_string(),
            session_id: "test".to_string(),
            agent_id: "test".to_string(),
            invocation: InvocationMode::Interactive,
            system_prompt: "你是一个测试助手。".to_string(),
            messages,
            tools,
            model: "mock".to_string(),
            max_iterations: 2,
            temperature: None,
            max_tokens: None,
            parallel_tools: false,
            tool_choice: ToolChoice::Auto,
            fail_on_tool_error: true,
            retry_mode: RetryMode::Disabled, // 测试禁用重试
        }
    }
}

// ============================================================================
// AgentRunResult - 结构化执行结果
// ============================================================================

/// 停止原因枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    /// LLM 返回了不包含工具调用的响应，循环自然中断
    #[default]
    Completed,

    /// 达到 max_iterations 上限
    MaxIterations,

    /// 工具错误且 fail_on_tool_error=true
    ToolError,

    /// 执行被取消（如用户 /stop）
    Cancelled,

    /// LLM 返回错误
    Error,
}

/// Token 使用量统计
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
}

impl TokenUsage {
    pub fn new(prompt: usize, completion: usize) -> Self {
        Self {
            prompt_tokens: prompt,
            completion_tokens: completion,
            total_tokens: prompt + completion,
        }
    }

    /// 累加多次使用的 Token
    pub fn add(&mut self, other: &TokenUsage) {
        self.prompt_tokens += other.prompt_tokens;
        self.completion_tokens += other.completion_tokens;
        self.total_tokens += other.total_tokens;
    }
}

/// 工具调用事件状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolStatus {
    Started,
    Succeeded,
    Failed { error: String },
}

/// 工具调用事件轨迹
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolEvent {
    pub name: String,
    pub tool_call_id: String,
    pub status: ToolStatus,
    pub duration_ms: u64,
    /// 输入摘要（用于调试）
    pub input_summary: String,
    /// 输出摘要（用于调试）
    pub output_summary: String,
}

impl From<&ToolTrace> for ToolEvent {
    fn from(trace: &ToolTrace) -> Self {
        let status = if trace.success {
            ToolStatus::Succeeded
        } else {
            ToolStatus::Failed {
                error: trace.output_summary.clone(),
            }
        };

        Self {
            name: trace.tool_name.clone(),
            tool_call_id: String::new(), // ToolTrace 暂无此字段
            status,
            duration_ms: trace.duration_ms,
            input_summary: trace.input_summary.clone(),
            output_summary: trace.output_summary.clone(),
        }
    }
}

/// 结构化 Agent 执行结果
///
/// 包含执行期间所发生情况的完整记录
#[derive(Debug, Clone)]
pub struct AgentRunResult {
    /// 清理并完成最终定稿的文本响应（设计文档：final_text）
    pub final_text: String,

    /// 包含所有中间工具调用轮次的完整消息列表（设计文档：full_message_chain）
    pub full_message_chain: Vec<ChatMessage>,

    /// 执行期间调用的工具名称的有序列表
    pub tools_used: Vec<String>,

    /// 所有 LLM 调用的聚合 Token 使用量
    pub usage: TokenUsage,

    /// 停止原因
    pub stop_reason: StopReason,

    /// 当 stop_reason 为 ToolError 或 Error 时的错误描述
    pub error: Option<String>,

    /// 每次工具调用的结构化事件轨迹
    pub tool_events: Vec<ToolEvent>,
}

impl Default for AgentRunResult {
    fn default() -> Self {
        Self {
            final_text: String::new(),
            full_message_chain: Vec::new(),
            tools_used: Vec::new(),
            usage: TokenUsage::default(),
            stop_reason: StopReason::Completed,
            error: None,
            tool_events: Vec::new(),
        }
    }
}

impl AgentRunResult {
    /// 创建成功完成的结果
    pub fn completed(
        final_text: String,
        full_message_chain: Vec<ChatMessage>,
        usage: TokenUsage,
    ) -> Self {
        Self {
            final_text,
            full_message_chain,
            tools_used: Vec::new(),
            usage,
            stop_reason: StopReason::Completed,
            error: None,
            tool_events: Vec::new(),
        }
    }

    /// 创建达到最大迭代的结果
    pub fn max_iterations(
        final_text: String,
        full_message_chain: Vec<ChatMessage>,
        max: usize,
    ) -> Self {
        Self {
            final_text,
            full_message_chain,
            tools_used: Vec::new(),
            usage: TokenUsage::default(),
            stop_reason: StopReason::MaxIterations,
            error: Some(format!("Reached maximum iterations ({})", max)),
            tool_events: Vec::new(),
        }
    }

    /// 创建工具错误结果
    pub fn tool_error(error: String, full_message_chain: Vec<ChatMessage>) -> Self {
        Self {
            final_text: format!("Tool error: {}", error),
            full_message_chain,
            tools_used: Vec::new(),
            usage: TokenUsage::default(),
            stop_reason: StopReason::ToolError,
            error: Some(error),
            tool_events: Vec::new(),
        }
    }

    /// 创建取消结果
    pub fn cancelled() -> Self {
        Self {
            final_text: "Execution cancelled".to_string(),
            full_message_chain: Vec::new(),
            tools_used: Vec::new(),
            usage: TokenUsage::default(),
            stop_reason: StopReason::Cancelled,
            error: None,
            tool_events: Vec::new(),
        }
    }

    /// 添加工具事件
    pub fn add_tool_event(&mut self, event: ToolEvent) {
        self.tools_used.push(event.name.clone());
        self.tool_events.push(event);
    }

    /// 是否成功完成
    pub fn is_success(&self) -> bool {
        self.stop_reason == StopReason::Completed
    }

    /// 是否有错误
    pub fn has_error(&self) -> bool {
        self.error.is_some()
    }
}

// ============================================================================
// IterationState - 迭代状态
// ============================================================================

/// 迭代状态（传递给 RunHooks）
#[derive(Debug, Clone)]
pub struct IterationState {
    /// 当前迭代次数（0-based）
    pub iteration: usize,
    /// 当前消息数量
    pub message_count: usize,
    /// 迭代开始时间
    pub start_time: std::time::Instant,
}

impl IterationState {
    pub fn new(iteration: usize, message_count: usize) -> Self {
        Self {
            iteration,
            message_count,
            start_time: std::time::Instant::now(),
        }
    }

    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }
}
