//! AgentHook — 生命周期钩子
//!
//! 连接业务层与执行层的桥梁
//! 提供六个扩展点，允许业务层将特定行为注入到 Runner 的迭代循环中

use crate::agent::spec::IterationState;
use crate::agent::tools::ToolCall;
use std::time::Instant;

// ============================================================================
// AgentHook Trait
// ============================================================================

/// Agent 生命周期钩子
///
/// 提供六个扩展点，允许业务层将特定行为注入到 Runner 的迭代循环中。
/// 默认实现对所有方法都是空操作，使得 Hook 完全可选。
pub trait AgentHook: Send {
    /// 决定本次迭代是否使用流式传输
    ///
    /// 返回 true：使用 chat_stream，逐 token 回调 on_stream
    /// 返回 false：使用 chat，一次性返回完整响应
    fn wants_streaming(&self) -> bool {
        true
    }

    /// 每次迭代开始时调用
    ///
    /// 用途：观察/重置状态，为新的 LLM 调用做准备
    fn before_iteration(&mut self, _state: &mut IterationState) {}

    /// 流式传输期间的每个内容增量
    ///
    /// 用途：将内容增量转发到 UI 层
    fn on_stream(&mut self, _delta: &str) {}

    /// 流式传输完成时调用
    ///
    /// # 参数
    /// - `resuming`: true 表示后续还有工具调用，false 表示最终响应
    ///
    /// 用途：发出流结束信号，UI 层可据此调整状态
    fn on_stream_end(&mut self, _resuming: bool) {}

    /// 工具执行之前调用
    ///
    /// 用途：设置工具路由上下文，记录工具调用日志，发送进度事件
    fn before_execute_tools(&mut self, _calls: &[ToolCall]) {}

    /// 每次迭代结束时调用
    ///
    /// 用途：持久化指标，完成状态定稿
    fn after_iteration(&mut self, _state: &IterationState) {}

    /// 最终响应时调用
    ///
    /// 用途：后处理内容（如剥离 think 标签）
    /// 返回处理后的内容
    fn finalize_content(&mut self, content: &str) -> String {
        content.to_string()
    }
}

// ============================================================================
// LoopHook - 业务层桥接实现
// ============================================================================

/// LoopHook - AgentLoop 使用的内部 Hook
///
/// 将 Runner 事件桥接到 MessageBus
pub struct LoopHook {
    /// 消息总线发布器
    publisher: Box<dyn LoopHookPublisher>,
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
}

/// LoopHook 发布器 trait（解耦 MessageBus）
pub trait LoopHookPublisher: Send {
    fn emit_status(&self, request_id: &str, session_id: &str, phase: crate::agent::events::UserVisiblePhase);
    fn emit_chunk(&self, request_id: &str, session_id: &str, segment_id: u64, content: &str);
    fn emit_segment_end(&self, request_id: &str, session_id: &str, segment_id: u64, resuming: bool);
}

impl LoopHook {
    /// 创建新的 LoopHook
    pub fn new(
        publisher: Box<dyn LoopHookPublisher>,
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
        }
    }
}

impl AgentHook for LoopHook {
    fn wants_streaming(&self) -> bool {
        true
    }

    fn before_iteration(&mut self, _state: &mut IterationState) {
        // 重置缓冲区
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

    fn on_stream(&mut self, delta: &str) {
        // 1. 缓冲原始内容
        self.buffer.push_str(delta);

        // 2. 剥离 think 标签（差分剥离）
        let stripped = strip_think_tags(&self.buffer);
        let new_chars = &stripped[self.last_stripped_len..];

        // 3. 只发送新增的清洁字符
        if !new_chars.is_empty() {
            self.publisher.emit_chunk(
                &self.request_id,
                &self.session_id,
                self.segment_id,
                new_chars,
            );
            self.last_stripped_len = stripped.len();
        }
    }

    fn on_stream_end(&mut self, resuming: bool) {
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

    fn before_execute_tools(&mut self, calls: &[ToolCall]) {
        // 发送工具提示
        let tool_names: Vec<_> = calls.iter().map(|c| c.name.as_str()).collect();
        tracing::info!(tools = ?tool_names, "tool_calls_started");
    }

    fn after_iteration(&mut self, state: &IterationState) {
        // 记录迭代耗时
        if let Some(start) = self.iteration_start {
            let elapsed = start.elapsed();
            tracing::debug!(
                iteration = state.iteration,
                elapsed_ms = elapsed.as_millis(),
                "iteration_completed"
            );
        }
    }

    fn finalize_content(&mut self, content: &str) -> String {
        // 剥离 think 标签
        strip_think_tags(content)
    }
}

// ============================================================================
// NoOpHook - 后台任务空实现
// ============================================================================

/// NoOpHook - 后台任务使用的空实现
pub struct NoOpHook;

impl AgentHook for NoOpHook {
    fn wants_streaming(&self) -> bool {
        false
    }
}

// ============================================================================
// TestHook - 测试记录实现
// ============================================================================

/// Hook 事件记录
#[derive(Debug, Clone)]
pub enum HookEvent {
    BeforeIteration { iteration: usize },
    StreamDelta { len: usize },
    StreamEnd { resuming: bool },
    BeforeExecuteTools { tool_count: usize },
    AfterIteration { iteration: usize },
    FinalizeContent { input_len: usize, output_len: usize },
}

/// TestHook - 测试使用的记录型 Hook
pub struct TestHook {
    pub events: Vec<HookEvent>,
    pub stream_deltas: Vec<String>,
    streaming: bool,
}

impl TestHook {
    pub fn new(streaming: bool) -> Self {
        Self {
            events: Vec::new(),
            stream_deltas: Vec::new(),
            streaming,
        }
    }
}

impl AgentHook for TestHook {
    fn wants_streaming(&self) -> bool {
        self.streaming
    }

    fn before_iteration(&mut self, state: &mut IterationState) {
        self.events.push(HookEvent::BeforeIteration {
            iteration: state.iteration,
        });
    }

    fn on_stream(&mut self, delta: &str) {
        self.stream_deltas.push(delta.to_string());
        self.events.push(HookEvent::StreamDelta { len: delta.len() });
    }

    fn on_stream_end(&mut self, resuming: bool) {
        self.events.push(HookEvent::StreamEnd { resuming });
    }

    fn before_execute_tools(&mut self, calls: &[ToolCall]) {
        self.events.push(HookEvent::BeforeExecuteTools {
            tool_count: calls.len(),
        });
    }

    fn after_iteration(&mut self, state: &IterationState) {
        self.events.push(HookEvent::AfterIteration {
            iteration: state.iteration,
        });
    }

    fn finalize_content(&mut self, content: &str) -> String {
        let output = strip_think_tags(content);
        self.events.push(HookEvent::FinalizeContent {
            input_len: content.len(),
            output_len: output.len(),
        });
        output
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

            // 收集可能的标签
            while let Some(&next_ch) = chars.peek() {
                if next_ch == '>' || tag_buffer.len() >= 7 {
                    break;
                }
                tag_buffer.push(chars.next().unwrap());
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