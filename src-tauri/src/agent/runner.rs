//! AgentRunner — 纯执行层
//!
//! 无状态的 LLM 迭代循环引擎，对基础设施一无所知
//! 只依赖输入的 AgentRunSpec，通过 RunHooks 与业务层通信
//!
//! 当前文件只负责：
//! - 多轮 LLM 调用
//! - 工具调用解析与执行
//! - 停止条件判断
//! - 结果聚合
//!
//! 它不负责：
//! - session 管理
//! - MessageBus 发布
//! - slash 命令
//! - 后台任务调度

use crate::agent::events::ProviderEvent;
use crate::agent::hooks::RunHooks;
use crate::agent::spec::{AgentRunResult, AgentRunSpec, IterationState, TokenUsage, ToolEvent};
use crate::agent::tools::{ToolCall, ToolRegistry, ToolResultMessage};
use crate::error::AppError;
use crate::providers::{ChatMessage, ChatRequest, MessageContent, Provider, ToolChoice};
use futures_util::StreamExt;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

/// AgentRunner - 纯执行层
///
/// 负责执行核心的"LLM 调用与工具执行"迭代循环。
/// 无状态，不持有 session、message bus 或任何基础设施依赖。
pub struct AgentRunner {
    /// LLM Provider
    provider: Arc<dyn Provider>,
    /// 工具注册表
    tool_registry: Arc<ToolRegistry>,
}

impl AgentRunner {
    /// 创建新的 AgentRunner
    pub fn new(provider: Arc<dyn Provider>, tool_registry: Arc<ToolRegistry>) -> Self {
        Self {
            provider,
            tool_registry,
        }
    }

    /// 获取 Provider 引用
    pub fn provider(&self) -> Arc<dyn Provider> {
        self.provider.clone()
    }

    /// 执行 Agent 迭代循环
    ///
    /// 核心流程：
    /// 1. 遍历迭代（最多 max_iterations）
    /// 2. 每次迭代调用 LLM（流式或非流式）
    /// 3. 检查是否有工具调用
    /// 4. 有工具调用则执行，追加结果，继续迭代
    /// 5. 无工具调用则定稿内容，返回结果
    pub async fn run(
        &self,
        spec: AgentRunSpec,
        hook: &mut dyn RunHooks,
        cancel: CancellationToken,
    ) -> Result<AgentRunResult, AppError> {
        let mut messages = spec.messages.clone();
        let mut tools_used = Vec::new();
        let mut tool_events = Vec::new();
        let mut usage = TokenUsage::default();

        for iteration in 0..spec.max_iterations {
            // 检查取消
            if cancel.is_cancelled() {
                return Ok(AgentRunResult::cancelled());
            }

            // 1. 迭代前钩子
            let mut state = IterationState::new(iteration, messages.len());
            hook.before_iteration(&mut state);

            // 2. 调用 LLM（流式或非流式）
            let response = if hook.wants_streaming() {
                self.chat_stream(&spec, &messages, hook, cancel.clone())
                    .await?
            } else {
                self.chat(&spec, &messages, cancel.clone()).await?
            };

            // 3. 更新使用量
            usage.add(&response.usage);

            // 4. 检查是否有工具调用
            if response.has_tool_calls {
                // 流结束信号（后续还有工具调用）
                hook.on_stream_end(true);

                // 追加 assistant 消息（含 tool_calls）
                let assistant_msg =
                    self.build_assistant_message(&response.content, &response.tool_calls);
                messages.push(assistant_msg);

                // 执行工具前钩子
                hook.before_execute_tools(&response.tool_calls);

                // 执行工具
                let results = self
                    .execute_tools(&spec, &response.tool_calls, cancel.clone())
                    .await;

                // 检查致命错误
                if spec.fail_on_tool_error {
                    let fatal_error = results.iter().find(|r| r.is_error);
                    if let Some(err) = fatal_error {
                        return Ok(AgentRunResult::tool_error(err.content.clone(), messages));
                    }
                }

                // 追加工具结果和记录事件
                for (call, result) in response.tool_calls.iter().zip(results.iter()) {
                    messages.push(ChatMessage::tool_result(
                        truncate(&result.content, 4_000),
                        call.id.clone(),
                        result.is_error,
                    ));
                    tools_used.push(call.name.clone());

                    tool_events.push(ToolEvent {
                        name: call.name.clone(),
                        tool_call_id: call.id.clone(),
                        status: if result.is_error {
                            crate::agent::spec::ToolStatus::Failed {
                                error: result.content.clone(),
                            }
                        } else {
                            crate::agent::spec::ToolStatus::Succeeded
                        },
                        duration_ms: 0, // TODO: 记录实际耗时
                        input_summary: truncate(&call.arguments.to_string(), 500),
                        output_summary: truncate(&result.content, 1_000),
                    });
                }

                // 迭代后钩子
                hook.after_iteration(&state);

                // 继续下一轮
                continue;
            }

            // 无工具调用 - 最终响应
            hook.on_stream_end(false);

            // 最终内容定稿
            let final_content = hook.finalize_content(&response.content);

            // 追加最终 assistant 消息
            messages.push(ChatMessage::assistant_text(&final_content));

            // 构建结果
            let mut result = AgentRunResult::completed(final_content, messages, usage);
            result.tools_used = tools_used;
            result.tool_events = tool_events;

            return Ok(result);
        }

        // 达到最大迭代次数
        let max_msg = format!("Reached maximum iterations ({})", spec.max_iterations);
        let mut result =
            AgentRunResult::max_iterations(max_msg.clone(), messages, spec.max_iterations);
        result.tools_used = tools_used;
        result.tool_events = tool_events;
        result.usage = usage;

        Ok(result)
    }

    /// 非流式 LLM 调用
    async fn chat(
        &self,
        spec: &AgentRunSpec,
        messages: &[ChatMessage],
        cancel: CancellationToken,
    ) -> Result<LLMResponse, AppError> {
        let response = self
            .provider
            .chat(ChatRequest {
                model: &spec.model,
                messages,
                system: Some(&spec.system_prompt),
                tools: &spec.tools,
                tool_choice: ToolChoice::Auto,
                max_tokens: spec.max_tokens.map(|n| n as u32),
                cancel,
            })
            .await?;

        // 解析工具调用
        let tool_calls = response
            .message
            .content
            .iter()
            .filter_map(|part| match part {
                MessageContent::ToolUse { id, name, input } => Some(ToolCall {
                    id: id.clone(),
                    name: name.clone(),
                    arguments: input.clone(),
                }),
                _ => None,
            })
            .collect::<Vec<_>>();

        let text_content = response.message.text_content();

        Ok(LLMResponse {
            content: text_content,
            has_tool_calls: !tool_calls.is_empty(),
            tool_calls,
            usage: TokenUsage::new(
                response.usage.input_tokens as usize,
                response.usage.output_tokens as usize,
            ),
            finish_reason: response.stop_reason,
        })
    }

    /// 流式 LLM 调用
    async fn chat_stream(
        &self,
        spec: &AgentRunSpec,
        messages: &[ChatMessage],
        hook: &mut dyn RunHooks,
        cancel: CancellationToken,
    ) -> Result<LLMResponse, AppError> {
        let mut stream = self
            .provider
            .chat_stream(ChatRequest {
                model: &spec.model,
                messages,
                system: Some(&spec.system_prompt),
                tools: &spec.tools,
                tool_choice: ToolChoice::Auto,
                max_tokens: spec.max_tokens.map(|n| n as u32),
                cancel,
            })
            .await?;

        let mut content = String::new();
        let mut tool_calls = Vec::new();
        let mut usage = TokenUsage::default();
        let mut finish_reason = String::new();

        while let Some(event) = stream.next().await {
            match event? {
                ProviderEvent::TextDelta { text } => {
                    content.push_str(&text);
                    hook.on_stream(&text);
                }
                ProviderEvent::ToolCall {
                    id,
                    name,
                    arguments_json,
                } => {
                    tool_calls.push(ToolCall {
                        id,
                        name,
                        arguments: arguments_json,
                    });
                }
                ProviderEvent::Finished {
                    usage: round_usage,
                    stop_reason,
                } => {
                    usage = TokenUsage::new(
                        round_usage.input_tokens as usize,
                        round_usage.output_tokens as usize,
                    );
                    finish_reason = stop_reason;
                    break;
                }
            }
        }

        Ok(LLMResponse {
            content,
            has_tool_calls: !tool_calls.is_empty(),
            tool_calls,
            usage,
            finish_reason,
        })
    }

    /// 执行工具调用
    async fn execute_tools(
        &self,
        spec: &AgentRunSpec,
        calls: &[ToolCall],
        cancel: CancellationToken,
    ) -> Vec<ToolResultMessage> {
        if spec.parallel_tools {
            self.tool_registry
                .execute_calls(calls.to_vec(), cancel)
                .await
                .unwrap_or_else(|e| {
                    tracing::error!(error = %e, "tool_execution_failed");
                    calls
                        .iter()
                        .map(|c| ToolResultMessage {
                            tool_call_id: c.id.clone(),
                            content: format!("Error: {}", e),
                            is_error: true,
                        })
                        .collect()
                })
        } else {
            // 顺序执行
            let mut results = Vec::new();
            for call in calls {
                let result = self.execute_single_tool(call, cancel.clone()).await;
                results.push(result);
            }
            results
        }
    }

    /// 执行单个工具
    async fn execute_single_tool(
        &self,
        call: &ToolCall,
        cancel: CancellationToken,
    ) -> ToolResultMessage {
        let calls = vec![call.clone()];
        let results = self.tool_registry.execute_calls(calls, cancel).await;

        match results {
            Ok(mut r) if !r.is_empty() => r.remove(0),
            Ok(_) => ToolResultMessage {
                tool_call_id: call.id.clone(),
                content: "No result".to_string(),
                is_error: true,
            },
            Err(e) => ToolResultMessage {
                tool_call_id: call.id.clone(),
                content: format!("Error: {}", e),
                is_error: true,
            },
        }
    }

    /// 构建 assistant 消息（含 tool_calls）
    fn build_assistant_message(&self, text: &str, tool_calls: &[ToolCall]) -> ChatMessage {
        let mut parts = Vec::new();

        if !text.is_empty() {
            parts.push(MessageContent::Text {
                text: text.to_string(),
            });
        }

        for call in tool_calls {
            parts.push(MessageContent::ToolUse {
                id: call.id.clone(),
                name: call.name.clone(),
                input: call.arguments.clone(),
            });
        }

        ChatMessage::assistant_parts(parts)
    }
}

/// LLM 响应内部结构
struct LLMResponse {
    content: String,
    tool_calls: Vec<ToolCall>,
    has_tool_calls: bool,
    usage: TokenUsage,
    #[allow(dead_code)]
    finish_reason: String,
}

/// 截断字符串
fn truncate(input: &str, max_len: usize) -> String {
    if input.chars().count() <= max_len {
        return input.to_string();
    }
    input.chars().take(max_len).collect()
}
