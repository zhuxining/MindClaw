use crate::services::agent::ConversationKey;

/// 记忆检索结果。
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct MemoryContext {
    /// 当前会话最近的消息历史
    pub recent_messages: Vec<String>,
    /// 长期记忆片段（跨会话保留）
    pub long_term_notes: Vec<String>,
}

/// 记忆数据源接口。
///
/// 实现此 trait 可以提供短期（session history）和长期记忆注入。
#[allow(dead_code)]
#[async_trait::async_trait]
pub trait MemorySource: Send + Sync {
    /// 获取指定会话的记忆上下文。
    async fn fetch(&self, key: &ConversationKey) -> MemoryContext;

    /// 记录一条新的记忆（长期记忆）。
    async fn remember(&self, key: &ConversationKey, note: String);
}

/// 空记忆源 — 不注入任何记忆。
#[allow(dead_code)]
pub struct NoopMemory;

#[async_trait::async_trait]
impl MemorySource for NoopMemory {
    async fn fetch(&self, _key: &ConversationKey) -> MemoryContext {
        MemoryContext {
            recent_messages: Vec::new(),
            long_term_notes: Vec::new(),
        }
    }

    async fn remember(&self, _key: &ConversationKey, _note: String) {}
}
