//! RunHooks — 生命周期钩子
//!
//! 连接业务层与执行层的桥梁
//! 提供 12 个执行节点扩展点，允许业务层将特定行为注入到 Runner 的生命周期
//!
//! 当前文件集中承接：
//! - `RunHooks` trait（12 个 on_* 方法）
//! - 面向 MessageBus 的 `InteractiveRunHooks`
//! - 空实现 `NoopRunHooks`
//! - 测试记录实现 `RecordingRunHooks`
//!
//! 边界说明：
//! - hooks 只处理 Runner 生命周期节点
//! - 不处理 AgentLoop 控制面命令
//! - 不推进迭代，只观察并做副作用桥接

use crate::agent::spec::{AgentRunResult, TokenUsage};
// Tool call placeholder (rig handles tool execution now)
#[derive(Debug, Clone)]
pub struct ToolCallPlaceholder {
    pub name: String,
    pub id: String,
}
use std::time::Instant;

// ============================================================================
// 上下文类型定义
// ============================================================================

/// Run 开始上下文
#[derive(Debug, Clone)]
pub struct RunStartContext {
    pub run_id: String,
    pub session_id: String,
    pub agent_id: String,
    pub message_count: usize,
}

/// 迭代开始上下文
#[derive(Debug, Clone)]
pub struct IterationStartContext {
    pub run_id: String,
    pub session_id: String,
    pub agent_id: String,
    pub iteration: usize,
    pub message_count: usize,
}

/// 模型请求上下文
#[derive(Debug, Clone)]
pub struct ModelRequestContext {
    pub run_id: String,
    pub session_id: String,
    pub agent_id: String,
    pub iteration: usize,
    pub model: String,
    pub is_streaming: bool,
}

/// 模型响应上下文
#[derive(Debug, Clone)]
pub struct ModelResponseContext {
    pub run_id: String,
    pub session_id: String,
    pub agent_id: String,
    pub iteration: usize,
    pub has_tool_calls: bool,
    pub tool_call_count: usize,
    pub usage: TokenUsage,
}

/// 迭代结束上下文
#[derive(Debug, Clone)]
pub struct IterationFinishContext {
    pub run_id: String,
    pub session_id: String,
    pub agent_id: String,
    pub iteration: usize,
    pub message_count: usize,
    pub usage: TokenUsage,
}

/// Run 中止原因
#[derive(Debug, Clone)]
pub enum RunAbortReason {
    Cancelled,
    Timeout { elapsed_ms: u64 },
    Error { message: String },
}

/// 流式模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamingMode {
    Disabled,
    TextOnly,
}

// ============================================================================
// RunHooks Trait
// ============================================================================

/// Run 生命周期钩子
///
/// 提供 12 个扩展点，对应 Runner 的关键执行节点。
/// 所有方法都有默认空实现，使得 Hook 完全可选。
pub trait RunHooks: Send {
    /// 返回流式传输模式
    fn streaming_mode(&self) -> StreamingMode {
        StreamingMode::TextOnly
    }

    /// Run 开始时调用
    fn on_run_start(&mut self, _ctx: &RunStartContext) {}

    /// 每次迭代开始时调用
    fn on_iteration_start(&mut self, _ctx: &IterationStartContext) {}

    /// 模型请求开始时调用
    fn on_model_request_start(&mut self, _ctx: &ModelRequestContext) {}

    /// 流式传输期间的每个内容增量
    fn on_model_text_delta(&mut self, _delta: &str) {}

    /// 模型响应准备好时调用（无论是否流式）
    fn on_model_response_ready(&mut self, _ctx: &ModelResponseContext) {}

    /// 工具调用批次开始时调用
    fn on_tool_batch_start(&mut self, _calls: &[ToolCallPlaceholder]) {}

    /// 单个工具调用开始时调用
    fn on_tool_call_start(&mut self, _call: &ToolCallPlaceholder) {}

    /// 单个工具调用完成时调用
    fn on_tool_call_finish(
        &mut self,
        _call: &ToolCallPlaceholder,
        _success: bool,
        _result_summary: &str,
    ) {
    }

    /// 每次迭代结束时调用
    fn on_iteration_finish(&mut self, _ctx: &IterationFinishContext) {}

    /// 最终响应定稿时调用
    /// 返回处理后的内容（如剥离 think 标签）
    fn finalize_response(&mut self, content: &str) -> String {
        content.to_string()
    }

    /// Run 正常完成时调用
    fn on_finish(&mut self, _result: &AgentRunResult) {}

    /// Run 中止时调用
    fn on_abort(&mut self, _reason: &RunAbortReason) {}
}

// ============================================================================
// InteractiveRunHooks - 业务层桥接实现
// ============================================================================

/// RunHook 发布器 trait（解耦 MessageBus）
pub trait RunHookPublisher: Send {
    fn emit_status(
        &self,
        request_id: &str,
        session_id: &str,
        status: crate::agent::events::UserVisiblePhase,
    );
    fn emit_chunk(&self, request_id: &str, session_id: &str, segment_id: u64, content: &str);
    fn emit_segment_end(&self, request_id: &str, session_id: &str, segment_id: u64, resuming: bool);
}

/// InteractiveRunHooks - AgentLoop 使用的内部 Hook
///
/// 将 Runner 事件桥接到 MessageBus
pub struct InteractiveRunHooks {
    /// 消息总线发布器
    publisher: Box<dyn RunHookPublisher>,
    /// 会话 ID
    session_id: String,
    /// 请求 ID
    request_id: String,
    /// 段计数器
    segment_id: u64,
    /// 流式缓冲区
    buffer: String,
    /// 上次剥离后的长度
    last_stripped_len: usize,
    /// 迭代开始时间
    iteration_start: Option<Instant>,
    /// 当前迭代
    current_iteration: usize,
}

impl InteractiveRunHooks {
    /// 创建新的 InteractiveRunHooks
    pub fn new(
        publisher: Box<dyn RunHookPublisher>,
        session_id: String,
        request_id: String,
    ) -> Self {
        Self {
            publisher,
            session_id,
            request_id,
            segment_id: 0,
            buffer: String::new(),
            last_stripped_len: 0,
            iteration_start: None,
            current_iteration: 0,
        }
    }
}

impl RunHooks for InteractiveRunHooks {
    fn streaming_mode(&self) -> StreamingMode {
        StreamingMode::TextOnly
    }

    fn on_run_start(&mut self, _ctx: &RunStartContext) {
        self.segment_id = 0;
        self.current_iteration = 0;
    }

    fn on_iteration_start(&mut self, ctx: &IterationStartContext) {
        self.current_iteration = ctx.iteration;
        self.buffer.clear();
        self.last_stripped_len = 0;
        self.iteration_start = Some(Instant::now());

        // 发送思考状态
        self.publisher.emit_status(
            &self.request_id,
            &self.session_id,
            crate::agent::events::UserVisiblePhase::Thinking,
        );
    }

    fn on_model_text_delta(&mut self, delta: &str) {
        // 1. 缓冲原始内容
        self.buffer.push_str(delta);

        // 2. 剥离 think 标签（差分剥离）
        let stripped = strip_think_tags(&self.buffer);

        // 安全切片：当 <think> 标签跨 delta 被补全时，stripped 长度可能小于
        // last_stripped_len（部分标签内容从输出中消失），此时 get() 返回 None，
        // 跳过本轮发送；last_stripped_len 同步更新，后续 delta 照常工作。
        let new_content = stripped.get(self.last_stripped_len..).unwrap_or("");

        // 3. 只发送新增的清洁字符
        if !new_content.is_empty() {
            self.publisher.emit_chunk(
                &self.request_id,
                &self.session_id,
                self.segment_id,
                new_content,
            );
        }
        self.last_stripped_len = stripped.len();
    }

    fn on_model_response_ready(&mut self, ctx: &ModelResponseContext) {
        // 流结束信号
        let resuming = ctx.has_tool_calls;
        self.publisher.emit_segment_end(
            &self.request_id,
            &self.session_id,
            self.segment_id,
            resuming,
        );

        if resuming {
            // 还有工具调用，新分段
            self.segment_id += 1;
            self.buffer.clear();
            self.last_stripped_len = 0;

            // 发送工具状态
            self.publisher.emit_status(
                &self.request_id,
                &self.session_id,
                crate::agent::events::UserVisiblePhase::UsingTools,
            );
        }
    }

    fn on_tool_batch_start(&mut self, calls: &[ToolCallPlaceholder]) {
        let tool_names: Vec<_> = calls.iter().map(|c| c.name.as_str()).collect();
        tracing::info!(tools = ?tool_names, "tool_calls_started");
    }

    fn on_iteration_finish(&mut self, _ctx: &IterationFinishContext) {
        // 记录迭代耗时
        if let Some(start) = self.iteration_start {
            let elapsed = start.elapsed();
            tracing::debug!(
                iteration = self.current_iteration,
                elapsed_ms = elapsed.as_millis(),
                "iteration_completed"
            );
        }
    }

    fn finalize_response(&mut self, content: &str) -> String {
        // 剥离 think 标签
        strip_think_tags(content)
    }

    fn on_finish(&mut self, _result: &AgentRunResult) {
        // 发送完成状态
        self.publisher.emit_status(
            &self.request_id,
            &self.session_id,
            crate::agent::events::UserVisiblePhase::Completed,
        );
    }

    fn on_abort(&mut self, reason: &RunAbortReason) {
        let status = match reason {
            RunAbortReason::Cancelled => crate::agent::events::UserVisiblePhase::Cancelled,
            RunAbortReason::Timeout { .. } => crate::agent::events::UserVisiblePhase::Error,
            RunAbortReason::Error { .. } => crate::agent::events::UserVisiblePhase::Error,
        };
        self.publisher
            .emit_status(&self.request_id, &self.session_id, status);
    }
}

// ============================================================================
// NoopRunHooks - 后台任务空实现
// ============================================================================

/// NoopRunHooks - 后台任务使用的空实现
pub struct NoopRunHooks;

impl RunHooks for NoopRunHooks {
    fn streaming_mode(&self) -> StreamingMode {
        StreamingMode::Disabled
    }
}

// ============================================================================
// RecordingRunHooks - 测试记录实现
// ============================================================================

/// Hook 事件记录
#[derive(Debug, Clone)]
pub enum RunHookEvent {
    RunStart { run_id: String },
    IterationStart { iteration: usize },
    ModelRequestStart { iteration: usize, model: String },
    ModelTextDelta { len: usize },
    ModelResponseReady { has_tool_calls: bool },
    ToolBatchStart { tool_count: usize },
    ToolCallStart { name: String },
    ToolCallFinish { name: String, success: bool },
    IterationFinish { iteration: usize },
    FinalizeResponse { input_len: usize, output_len: usize },
    Finish { stop_reason: String },
    Abort { reason: String },
}

/// RecordingRunHooks - 测试使用的记录型 Hook
pub struct RecordingRunHooks {
    pub events: Vec<RunHookEvent>,
    pub stream_deltas: Vec<String>,
    streaming: bool,
    current_iteration: usize,
}

impl RecordingRunHooks {
    pub fn new(streaming: bool) -> Self {
        Self {
            events: Vec::new(),
            stream_deltas: Vec::new(),
            streaming,
            current_iteration: 0,
        }
    }
}

impl RunHooks for RecordingRunHooks {
    fn streaming_mode(&self) -> StreamingMode {
        if self.streaming {
            StreamingMode::TextOnly
        } else {
            StreamingMode::Disabled
        }
    }

    fn on_run_start(&mut self, ctx: &RunStartContext) {
        self.events.push(RunHookEvent::RunStart {
            run_id: ctx.run_id.clone(),
        });
    }

    fn on_iteration_start(&mut self, ctx: &IterationStartContext) {
        self.current_iteration = ctx.iteration;
        self.events.push(RunHookEvent::IterationStart {
            iteration: ctx.iteration,
        });
    }

    fn on_model_request_start(&mut self, ctx: &ModelRequestContext) {
        self.events.push(RunHookEvent::ModelRequestStart {
            iteration: ctx.iteration,
            model: ctx.model.clone(),
        });
    }

    fn on_model_text_delta(&mut self, delta: &str) {
        self.stream_deltas.push(delta.to_string());
        self.events
            .push(RunHookEvent::ModelTextDelta { len: delta.len() });
    }

    fn on_model_response_ready(&mut self, ctx: &ModelResponseContext) {
        self.events.push(RunHookEvent::ModelResponseReady {
            has_tool_calls: ctx.has_tool_calls,
        });
    }

    fn on_tool_batch_start(&mut self, calls: &[ToolCallPlaceholder]) {
        self.events.push(RunHookEvent::ToolBatchStart {
            tool_count: calls.len(),
        });
    }

    fn on_tool_call_start(&mut self, call: &ToolCallPlaceholder) {
        self.events.push(RunHookEvent::ToolCallStart {
            name: call.name.clone(),
        });
    }

    fn on_tool_call_finish(
        &mut self,
        call: &ToolCallPlaceholder,
        success: bool,
        _result_summary: &str,
    ) {
        self.events.push(RunHookEvent::ToolCallFinish {
            name: call.name.clone(),
            success,
        });
    }

    fn on_iteration_finish(&mut self, ctx: &IterationFinishContext) {
        self.events.push(RunHookEvent::IterationFinish {
            iteration: ctx.iteration,
        });
    }

    fn finalize_response(&mut self, content: &str) -> String {
        let output = strip_think_tags(content);
        self.events.push(RunHookEvent::FinalizeResponse {
            input_len: content.len(),
            output_len: output.len(),
        });
        output
    }

    fn on_finish(&mut self, result: &AgentRunResult) {
        self.events.push(RunHookEvent::Finish {
            stop_reason: format!("{:?}", result.stop_reason),
        });
    }

    fn on_abort(&mut self, reason: &RunAbortReason) {
        self.events.push(RunHookEvent::Abort {
            reason: format!("{:?}", reason),
        });
    }
}

// ============================================================================
// Think 标签剥离
// ============================================================================

/// 剥离 <think>...</think> 标签
///
/// 使用差分剥离策略：保留原始缓冲区，计算剥离后的版本，
/// 只返回在剥离后出现在最新增量中的字符。
pub fn strip_think_tags(content: &str) -> String {
    let mut result = String::new();
    let mut in_think = false;
    let mut chars = content.chars().peekable();

    while let Some(ch) = chars.next() {
        // 检查是否进入 think 标签
        if ch == '<' {
            let mut tag_buffer = String::new();
            tag_buffer.push(ch);

            // 收集可能的标签（最多到 '>' 或 buffer 满）
            while tag_buffer.len() < 8 {
                match chars.peek() {
                    Some(&next_ch) if next_ch != '>' => {
                        tag_buffer.push(next_ch);
                        chars.next(); // consume the character
                    }
                    _ => break,
                }
            }

            // 检查标签类型
            if tag_buffer == "<think" && chars.peek() == Some(&'>') {
                chars.next(); // 消耗 '>'
                in_think = true;
                continue;
            } else if tag_buffer == "</think" && chars.peek() == Some(&'>') {
                chars.next(); // 消耗 '>'
                in_think = false;
                continue;
            } else {
                // 不是 think 标签，输出已收集的字符
                if !in_think {
                    result.push_str(&tag_buffer);
                }
                continue;
            }
        }

        if !in_think {
            result.push(ch);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_think_tags() {
        let input = "Hello <think>internal thought</think> World";
        let output = strip_think_tags(input);
        assert_eq!(output, "Hello  World");
    }

    #[test]
    fn test_strip_think_tags_multiple() {
        let input = "A <think>x</think> B <think>y</think> C";
        let output = strip_think_tags(input);
        assert_eq!(output, "A  B  C");
    }

    #[test]
    fn test_strip_think_tags_incomplete() {
        let input = "Start <think>partial";
        let output = strip_think_tags(input);
        assert_eq!(output, "Start ");
    }
}
