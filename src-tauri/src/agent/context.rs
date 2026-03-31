//! ContextPipeline：可插拔上下文管线
//!
//! 上下文组装采用可插拔 ContextSource 管线，调用时机固定为：
//! Session 解析完成、Agent Command 未命中之后，由 run_once() 显式触发

use crate::agent::session::AgentSession;
use crate::bus::events::InboundMessage;
use crate::error::AppError;
use crate::providers::{ChatMessage, MessageRole as ProviderMessageRole};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

/// 消息角色
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

impl std::fmt::Display for MessageRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageRole::System => write!(f, "system"),
            MessageRole::User => write!(f, "user"),
            MessageRole::Assistant => write!(f, "assistant"),
        }
    }
}

/// 上下文片段
#[derive(Debug, Clone)]
pub struct ContextFragment {
    /// 消息角色
    pub role: MessageRole,
    /// 内容文本
    pub content: String,
    /// Token 估算
    pub token_estimate: usize,
    /// 来源标签
    pub label: String,
}

impl ContextFragment {
    /// 创建新的上下文片段
    pub fn new(
        role: MessageRole,
        content: impl Into<String>,
        token_estimate: usize,
        label: impl Into<String>,
    ) -> Self {
        Self {
            role,
            content: content.into(),
            token_estimate,
            label: label.into(),
        }
    }

    /// 转换为 ChatMessage
    pub fn to_chat_message(&self) -> ChatMessage {
        match self.role {
            MessageRole::System => ChatMessage::system(&self.content),
            MessageRole::User => ChatMessage::user(&self.content),
            MessageRole::Assistant => ChatMessage::assistant_text(&self.content),
        }
    }
}

/// 上下文源 trait
///
/// 每个 ContextSource 负责注入一类上下文片段
#[async_trait]
pub trait ContextSource: Send + Sync {
    /// 来源名称
    fn name(&self) -> &str;
    /// 优先级（数值越小优先级越高）
    fn priority(&self) -> i32;
    /// 是否启用
    fn enabled(&self) -> bool {
        true
    }
    /// 注入上下文片段
    ///
    /// # Arguments
    /// * `ctx` - 构建上下文
    /// * `budget` - Token 预算
    async fn inject(
        &self,
        ctx: &ContextBuildContext,
        budget: usize,
    ) -> Result<Vec<ContextFragment>, AppError>;
}

/// 上下文构建上下文
///
/// 使用 Arc 而非引用：避免 async_trait + 多重借用的生命周期冲突
/// AgentLoop 本身通过 Arc 持有这些资源，clone Arc 成本可忽略
pub struct ContextBuildContext {
    /// 入站消息
    pub inbound: InboundMessage,
    /// 当前会话
    pub session: Arc<AgentSession>,
    // TODO: 添加 MemoryManager, ServiceContainer, DbState
}

impl ContextBuildContext {
    /// 创建新的构建上下文
    pub fn new(inbound: InboundMessage, session: Arc<AgentSession>) -> Self {
        Self { inbound, session }
    }
}

/// 构建完成的上下文
#[derive(Debug, Clone)]
pub struct BuiltContext {
    /// 系统提示词
    pub system_prompt: String,
    /// 消息列表（供 LLM 使用）
    pub messages: Vec<ChatMessage>,
    /// 所有片段（用于调试/审计）
    pub fragments: Vec<ContextFragment>,
}

impl BuiltContext {
    /// 创建空的构建上下文
    pub fn empty() -> Self {
        Self {
            system_prompt: String::new(),
            messages: Vec::new(),
            fragments: Vec::new(),
        }
    }

    /// 估算总 Token 数（粗略估计）
    pub fn estimated_tokens(&self) -> usize {
        let system_tokens = self.system_prompt.len() / 4;
        let message_tokens: usize = self.fragments.iter().map(|f| f.token_estimate).sum();
        system_tokens + message_tokens
    }
}

/// 上下文管线
pub struct ContextPipeline {
    /// 上下文源列表（按优先级排序）
    sources: Vec<Arc<dyn ContextSource>>,
    /// 总 Token 预算
    total_budget: usize,
    /// 各来源的预算分配
    budget_allocations: HashMap<String, usize>,
}

impl ContextPipeline {
    /// 创建新的上下文管线
    pub fn new(total_budget: usize) -> Self {
        Self {
            sources: Vec::new(),
            total_budget,
            budget_allocations: HashMap::new(),
        }
    }

    /// 添加上下文源
    pub fn add_source(&mut self, source: Arc<dyn ContextSource>) {
        self.sources.push(source);
        // 按优先级排序
        self.sources.sort_by_key(|s| s.priority());
    }

    /// 设置特定来源的预算
    pub fn set_budget(&mut self, source_name: &str, budget: usize) {
        self.budget_allocations
            .insert(source_name.to_string(), budget);
    }

    /// 获取来源的预算
    fn get_budget(&self, source_name: &str) -> usize {
        self.budget_allocations
            .get(source_name)
            .copied()
            .unwrap_or(self.total_budget / self.sources.len().max(1))
    }

    /// 构建上下文
    ///
    /// 按优先级顺序遍历 source，在各自 budget 内注入 fragment
    /// 超预算时优先削减 RAG，再压缩历史，最后截断观察类内容
    pub async fn build(&self, ctx: &ContextBuildContext) -> Result<BuiltContext, AppError> {
        let mut fragments = Vec::new();
        let mut remaining_budget = self.total_budget;

        for source in &self.sources {
            if !source.enabled() {
                continue;
            }

            let budget = self.get_budget(source.name()).min(remaining_budget);
            if budget == 0 {
                break;
            }

            match source.inject(ctx, budget).await {
                Ok(source_fragments) => {
                    let used_tokens: usize =
                        source_fragments.iter().map(|f| f.token_estimate).sum();
                    remaining_budget = remaining_budget.saturating_sub(used_tokens);
                    fragments.extend(source_fragments);
                }
                Err(e) => {
                    tracing::warn!(
                        source = %source.name(),
                        error = %e,
                        "context_source_inject_failed"
                    );
                    // 继续处理其他来源
                }
            }
        }

        // 分离系统提示词和普通消息
        let mut system_prompt = String::new();
        let mut messages = Vec::new();

        for fragment in &fragments {
            match fragment.role {
                MessageRole::System if system_prompt.is_empty() => {
                    system_prompt = fragment.content.clone();
                }
                _ => {
                    messages.push(fragment.to_chat_message());
                }
            }
        }

        Ok(BuiltContext {
            system_prompt,
            messages,
            fragments,
        })
    }
}

// ============================================================================
// 内置 ContextSource 实现
// ============================================================================

/// 系统提示词来源（固定层）
///
/// 优先级：0，预算：~2K tokens
/// 加载 SOUL.md + IDENTITY.md + USER.md + Tool Schema
pub struct SystemPromptSource {
    prompt: String,
}

impl SystemPromptSource {
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
        }
    }
}

#[async_trait]
impl ContextSource for SystemPromptSource {
    fn name(&self) -> &str {
        "system_prompt"
    }

    fn priority(&self) -> i32 {
        0
    }

    async fn inject(
        &self,
        _ctx: &ContextBuildContext,
        _budget: usize,
    ) -> Result<Vec<ContextFragment>, AppError> {
        let token_estimate = self.prompt.len() / 4;
        Ok(vec![ContextFragment::new(
            MessageRole::System,
            &self.prompt,
            token_estimate,
            "system_prompt",
        )])
    }
}

/// 用户消息来源
///
/// 优先级：100，预算：~1K tokens
/// 始终最后注入
pub struct UserMessageSource;

#[async_trait]
impl ContextSource for UserMessageSource {
    fn name(&self) -> &str {
        "user_message"
    }

    fn priority(&self) -> i32 {
        100
    }

    async fn inject(
        &self,
        ctx: &ContextBuildContext,
        _budget: usize,
    ) -> Result<Vec<ContextFragment>, AppError> {
        let content = &ctx.inbound.content;
        let token_estimate = content.len() / 4;
        Ok(vec![ContextFragment::new(
            MessageRole::User,
            content,
            token_estimate,
            "user_message",
        )])
    }
}

/// 对话历史来源
///
/// 优先级：30，预算：~50K tokens
/// 近 5 轮完整 + 早期摘要
pub struct ConversationHistorySource {
    keep_recent: usize,
}

impl ConversationHistorySource {
    pub fn new(keep_recent: usize) -> Self {
        Self { keep_recent }
    }
}

// ============================================================================
// 便捷构建方法
// ============================================================================

impl ContextPipeline {
    /// 根据配置构建默认的上下文管线
    ///
    /// 包含三个默认来源：
    /// 1. SystemPromptSource - 系统提示词
    /// 2. ConversationHistorySource - 对话历史（近 5 轮）
    /// 3. UserMessageSource - 用户消息
    pub fn build_default(config: &crate::runtime::config::AppConfig) -> Arc<Self> {
        let mut pipeline = Self::new(config.context_token_limit);
        pipeline.add_source(Arc::new(SystemPromptSource::new(&config.system_prompt)));
        pipeline.add_source(Arc::new(ConversationHistorySource::new(5)));
        pipeline.add_source(Arc::new(UserMessageSource));
        Arc::new(pipeline)
    }
}

impl Default for ConversationHistorySource {
    fn default() -> Self {
        Self::new(5)
    }
}

#[async_trait]
impl ContextSource for ConversationHistorySource {
    fn name(&self) -> &str {
        "conversation_history"
    }

    fn priority(&self) -> i32 {
        30
    }

    async fn inject(
        &self,
        ctx: &ContextBuildContext,
        budget: usize,
    ) -> Result<Vec<ContextFragment>, AppError> {
        let history = ctx.session.compressed_history(self.keep_recent);
        let mut fragments = Vec::new();
        let mut used_tokens = 0usize;

        for msg in history {
            let token_estimate = msg.content.len() / 4;
            if used_tokens + token_estimate > budget {
                break;
            }
            used_tokens += token_estimate;

            let role = match msg.role {
                ProviderMessageRole::System => MessageRole::System,
                ProviderMessageRole::Assistant => MessageRole::Assistant,
                ProviderMessageRole::User => MessageRole::User,
            };

            fragments.push(ContextFragment::new(
                role,
                msg.text_content(),
                token_estimate,
                "history",
            ));
        }

        Ok(fragments)
    }
}
