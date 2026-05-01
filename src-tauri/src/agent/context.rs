//! ContextPipeline：三层上下文架构
//!
//! 核心命题：记忆是 Agent 的，知识是共同的
//!
//! - 第 1 层 Core：缓存稳定（SOUL.md / USER.md / AGENTS.md / 工具 Schema）
//! - 第 2 层 Dynamic：每次请求变化（Identity / Memory / Vault / Skills）
//! - 第 3 层 User：运行时上下文 + 用户输入

use crate::agent::session::AgentSession;
use crate::bus::events::InboundMessage;
use crate::error::AppError;
use crate::providers::{ChatMessage, MessageRole as ProviderMessageRole};
use async_trait::async_trait;
use std::sync::Arc;

// ============================================================================
// ContextLayer - 三层架构
// ============================================================================

/// 上下文层级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ContextLayer {
    /// 第 1 层：核心稳定（缓存稳定）
    /// SOUL.md / USER.md / AGENTS.md / 工具 Schema
    Core,

    /// 第 2 层：动态上下文（每次请求变化）
    /// Identity / Memory / Vault / Skills
    Dynamic,

    /// 第 3 层：用户消息
    /// 运行时上下文 + 用户输入
    User,
}

// ============================================================================
// ContextFragment - 上下文片段
// ============================================================================

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
    /// 层级
    pub layer: ContextLayer,
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
    pub fn new(
        layer: ContextLayer,
        role: MessageRole,
        content: impl Into<String>,
        label: impl Into<String>,
    ) -> Self {
        let content = content.into();
        let token_estimate = content.len() / 4;
        Self {
            layer,
            role,
            content,
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

// ============================================================================
// ContextSource - 上下文源 Trait
// ============================================================================

/// 上下文源 trait
///
/// 每个 ContextSource 负责注入一类上下文片段
#[async_trait]
pub trait ContextSource: Send + Sync {
    /// 来源名称
    fn name(&self) -> &str;

    /// 层级
    fn layer(&self) -> ContextLayer;

    /// 优先级（同层级内，数值越小优先级越高）
    fn priority(&self) -> i32;

    /// 是否启用
    fn enabled(&self) -> bool {
        true
    }

    /// 注入上下文片段
    async fn inject(&self, ctx: &ContextBuildContext) -> Result<Vec<ContextFragment>, AppError>;
}

// ============================================================================
// ContextBuildContext - 构建上下文
// ============================================================================

/// 上下文构建上下文
///
/// 使用 Arc 而非引用：避免 async_trait + 多重借用的生命周期冲突
pub struct ContextBuildContext {
    /// 入站消息
    pub inbound: InboundMessage,
    /// 当前会话
    pub session: Arc<AgentSession>,
    // TODO: 添加 MemoryManager, VaultStore
}

impl ContextBuildContext {
    pub fn new(inbound: InboundMessage, session: Arc<AgentSession>) -> Self {
        Self { inbound, session }
    }
}

// ============================================================================
// BuiltContext - 构建结果
// ============================================================================

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
    pub fn empty() -> Self {
        Self {
            system_prompt: String::new(),
            messages: Vec::new(),
            fragments: Vec::new(),
        }
    }

    /// 估算总 Token 数
    pub fn estimated_tokens(&self) -> usize {
        let system_tokens = self.system_prompt.len() / 4;
        let message_tokens: usize = self.fragments.iter().map(|f| f.token_estimate).sum();
        system_tokens + message_tokens
    }
}

// ============================================================================
// ContextPipeline - 上下文管线
// ============================================================================

/// 上下文管线
///
/// 按三层架构顺序构建消息列表
pub struct ContextPipeline {
    /// 上下文源列表
    sources: Vec<Arc<dyn ContextSource>>,
    /// Core 层缓存
    #[allow(dead_code)]
    core_cache: CoreLayerCache,
    /// Token 预算
    total_budget: usize,
}

impl ContextPipeline {
    pub fn new(total_budget: usize) -> Self {
        Self {
            sources: Vec::new(),
            core_cache: CoreLayerCache::new(),
            total_budget,
        }
    }

    /// 添加上下文源
    pub fn add_source(&mut self, source: Arc<dyn ContextSource>) {
        self.sources.push(source);
        // 按层级和优先级排序
        self.sources.sort_by(|a, b| {
            a.layer()
                .cmp(&b.layer())
                .then_with(|| a.priority().cmp(&b.priority()))
        });
    }

    /// 构建上下文
    ///
    /// 按三层架构顺序构建：
    /// 1. Core 层（缓存）
    /// 2. Dynamic 层
    /// 3. User 层
    pub async fn build(&self, ctx: &ContextBuildContext) -> Result<BuiltContext, AppError> {
        let mut fragments = Vec::new();
        let mut system_prompt = String::new();
        let mut messages = Vec::new();

        // 按层级分组处理
        let mut remaining_budget = self.total_budget;

        // 第 1 层：Core（缓存稳定）
        for source in &self.sources {
            if source.layer() != ContextLayer::Core {
                continue;
            }
            if !source.enabled() {
                continue;
            }

            let source_fragments = source.inject(ctx).await?;
            for f in source_fragments {
                if f.role == MessageRole::System && system_prompt.is_empty() {
                    system_prompt = f.content.clone();
                }
                remaining_budget = remaining_budget.saturating_sub(f.token_estimate);
                fragments.push(f);
            }
        }

        // 第 2 层：Dynamic（每次请求变化）
        for source in &self.sources {
            if source.layer() != ContextLayer::Dynamic {
                continue;
            }
            if !source.enabled() {
                continue;
            }

            let source_fragments = source.inject(ctx).await?;
            for f in source_fragments {
                if remaining_budget == 0 {
                    break;
                }
                remaining_budget = remaining_budget.saturating_sub(f.token_estimate);
                messages.push(f.to_chat_message());
                fragments.push(f);
            }
        }

        // 第 3 层：User（运行时上下文 + 用户输入）
        for source in &self.sources {
            if source.layer() != ContextLayer::User {
                continue;
            }
            if !source.enabled() {
                continue;
            }

            let source_fragments = source.inject(ctx).await?;
            for f in source_fragments {
                fragments.push(f.clone());
                messages.push(f.to_chat_message());
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
// CoreLayerCache - 核心层缓存
// ============================================================================

/// Core 层缓存
///
/// Core 层内容稳定，支持缓存以优化 Token 使用和 KV-cache 复用
pub struct CoreLayerCache {
    /// 缓存的消息
    cached_system_prompt: Option<String>,
    /// 内容哈希（用于检测变化）
    hash: String,
}

impl CoreLayerCache {
    pub fn new() -> Self {
        Self {
            cached_system_prompt: None,
            hash: String::new(),
        }
    }

    /// 获取或更新缓存
    pub fn get_or_update(&mut self, content: &str, hash: &str) -> String {
        if self.hash != hash {
            self.cached_system_prompt = Some(content.to_string());
            self.hash = hash.to_string();
        }
        self.cached_system_prompt.clone().unwrap_or_default()
    }

    /// 计算内容哈希
    pub fn compute_hash(content: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}

impl Default for CoreLayerCache {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 内置 ContextSource 实现
// ============================================================================

/// 系统提示词来源（Core 层）
///
/// 加载 SOUL.md + USER.md + AGENTS.md
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

    fn layer(&self) -> ContextLayer {
        ContextLayer::Core
    }

    fn priority(&self) -> i32 {
        0
    }

    async fn inject(&self, _ctx: &ContextBuildContext) -> Result<Vec<ContextFragment>, AppError> {
        Ok(vec![ContextFragment::new(
            ContextLayer::Core,
            MessageRole::System,
            &self.prompt,
            "system_prompt",
        )])
    }
}

/// 用户消息来源（User 层）
///
/// 始终最后注入，包含运行时上下文
pub struct UserMessageSource;

#[async_trait]
impl ContextSource for UserMessageSource {
    fn name(&self) -> &str {
        "user_message"
    }

    fn layer(&self) -> ContextLayer {
        ContextLayer::User
    }

    fn priority(&self) -> i32 {
        100
    }

    async fn inject(&self, ctx: &ContextBuildContext) -> Result<Vec<ContextFragment>, AppError> {
        // 注入运行时上下文
        let runtime_context = format!(
            "[当前时间：{}，平台：{}]",
            chrono::Local::now().format("%Y-%m-%d %H:%M %A %z"),
            std::env::consts::OS,
        );

        let content = format!("{}\n\n{}", runtime_context, ctx.inbound.content);

        Ok(vec![ContextFragment::new(
            ContextLayer::User,
            MessageRole::User,
            content,
            "user_message",
        )])
    }
}

/// 对话历史来源（Dynamic 层）
///
/// 近 N 轮完整 + 早期摘要
pub struct ConversationHistorySource {
    keep_recent: usize,
}

impl ConversationHistorySource {
    pub fn new(keep_recent: usize) -> Self {
        Self { keep_recent }
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

    fn layer(&self) -> ContextLayer {
        ContextLayer::Dynamic
    }

    fn priority(&self) -> i32 {
        30
    }

    async fn inject(&self, ctx: &ContextBuildContext) -> Result<Vec<ContextFragment>, AppError> {
        let history = ctx.session.compressed_history(self.keep_recent);
        let mut fragments = Vec::new();

        for msg in history {
            let role = match msg.role {
                ProviderMessageRole::System => MessageRole::System,
                ProviderMessageRole::Assistant => MessageRole::Assistant,
                ProviderMessageRole::User => MessageRole::User,
            };

            fragments.push(ContextFragment::new(
                ContextLayer::Dynamic,
                role,
                msg.text_content(),
                "history",
            ));
        }

        Ok(fragments)
    }
}

// ============================================================================
// 便捷构建方法
// ============================================================================

impl ContextPipeline {
    /// 根据配置构建默认的上下文管线
    ///
    /// 包含三个默认来源：
    /// 1. SystemPromptSource - 系统提示词（Core 层）
    /// 2. ConversationHistorySource - 对话历史（Dynamic 层）
    /// 3. UserMessageSource - 用户消息（User 层）
    pub fn build_default(config: &crate::runtime::config::AppConfig) -> Arc<Self> {
        let mut pipeline = Self::new(config.context_token_limit);
        pipeline.add_source(Arc::new(SystemPromptSource::new(&config.system_prompt)));
        pipeline.add_source(Arc::new(ConversationHistorySource::new(5)));
        pipeline.add_source(Arc::new(UserMessageSource));
        Arc::new(pipeline)
    }
}
