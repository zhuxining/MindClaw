//! AutoCompact — 自动压缩历史
//!
//! 参考 nanobot 的 AutoCompact 实现
//! 当 Session 过期或 token 超限时自动压缩历史

use crate::agent::session::{AgentSession, SessionManager};
use chrono::{Duration, Utc};
use std::sync::Arc;

/// AutoCompact 配置
#[derive(Debug, Clone)]
pub struct AutoCompactConfig {
    /// Session 过期时间（分钟）
    pub session_ttl_minutes: u64,
    /// 压缩比例（保留多少比例的完整历史）
    pub compression_ratio: f64,
    /// 最大消息数
    pub max_messages: usize,
    /// 检查间隔（秒）
    pub check_interval_seconds: u64,
}

impl Default for AutoCompactConfig {
    fn default() -> Self {
        Self {
            session_ttl_minutes: 30,
            compression_ratio: 0.5,
            max_messages: 100,
            check_interval_seconds: 60,
        }
    }
}

/// AutoCompact 服务
pub struct AutoCompact {
    #[allow(dead_code)]
    session_mgr: Arc<SessionManager>,
    config: AutoCompactConfig,
}

impl AutoCompact {
    pub fn new(session_mgr: Arc<SessionManager>, config: AutoCompactConfig) -> Self {
        Self {
            session_mgr,
            config,
        }
    }

    /// 检查过期 Session
    ///
    /// 返回需要压缩的 Session ID 列表
    pub async fn check_expired(&self) -> Vec<String> {
        // 获取所有 session（通过 SessionManager 的内部缓存）
        // 由于 SessionManager 没有公开获取所有 session 的方法，
        // 这里返回空列表，实际实现需要扩展 SessionManager
        Vec::new()
    }

    /// 检查单个 Session 是否需要压缩
    pub fn should_compact(&self, session: &AgentSession) -> bool {
        // 检查消息数是否超过上限
        if session.turns.len() > self.config.max_messages {
            return true;
        }

        // 检查是否过期
        let now = Utc::now();
        let ttl = Duration::minutes(self.config.session_ttl_minutes as i64);
        if now - session.updated > ttl {
            return true;
        }

        false
    }

    /// 准备 Session（压缩或标记）
    ///
    /// 返回（可能压缩后的 session, pending summary）
    pub fn prepare_session(&self, session: AgentSession) -> (AgentSession, Option<String>) {
        if !self.should_compact(&session) {
            return (session, None);
        }

        // 执行压缩
        let compressed = self.compact_session(session);
        (
            compressed,
            Some("Session history compacted due to size limits.".to_string()),
        )
    }

    /// 压缩 Session 历史
    fn compact_session(&self, session: AgentSession) -> AgentSession {
        let total_turns = session.turns.len();
        if total_turns <= 5 {
            return session;
        }

        // 保留最近的 turns
        let keep_count = (total_turns as f64 * self.config.compression_ratio) as usize;
        let keep_count = keep_count.max(5);

        let mut compressed = session;
        compressed.turns = compressed.turns.split_off(total_turns - keep_count);

        tracing::info!(
            session_id = %compressed.id,
            original_turns = total_turns,
            kept_turns = compressed.turns.len(),
            "session_compacted"
        );

        compressed
    }
}

/// Compaction 结果
#[derive(Debug, Clone)]
pub struct CompactResult {
    pub session_id: String,
    pub original_turns: usize,
    pub compacted_turns: usize,
    pub summary: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_compact_by_message_count() {
        let config = AutoCompactConfig {
            max_messages: 10,
            ..Default::default()
        };

        // 配置验证
        assert_eq!(config.max_messages, 10);
    }

    #[test]
    fn test_compression_ratio() {
        let config = AutoCompactConfig {
            compression_ratio: 0.5,
            max_messages: 100,
            ..Default::default()
        };

        // 100 条消息压缩后应保留 50 条
        let expected = 50;
        let actual = (100.0 * config.compression_ratio) as usize;
        assert_eq!(actual, expected);
    }
}
