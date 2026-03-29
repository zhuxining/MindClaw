//! SessionManager：会话生命周期管理
//!
//! 会话管理器保存一个完整 turn 的执行结果，支持恢复、调试和历史压缩

use crate::error::AppError;
use crate::models::conversation::ConversationMode;
use crate::providers::ChatMessage;
use chrono::{DateTime, Utc};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Agent 层的 Session 结构（内部使用，与 models::Session 区分）
#[derive(Debug, Clone)]
pub struct AgentSession {
    pub id: String,
    pub sender: String,
    pub mode: ConversationMode,
    /// 对话回合记录
    pub turns: Vec<TurnRecord>,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
}

impl AgentSession {
    /// 创建新会话
    pub fn new(sender: String, mode: ConversationMode) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            sender,
            mode,
            turns: Vec::new(),
            created: now,
            updated: now,
        }
    }

    /// 从已有 ID 创建（恢复会话）
    pub fn with_id(id: String, sender: String, mode: ConversationMode) -> Self {
        let now = Utc::now();
        Self {
            id,
            sender,
            mode,
            turns: Vec::new(),
            created: now,
            updated: now,
        }
    }

    /// 追加回合
    pub fn append_turn(&mut self, turn: TurnRecord) {
        self.turns.push(turn);
        self.updated = Utc::now();
    }

    /// 压缩后的历史（近 N 轮完整 + 早期摘要）
    pub fn compressed_history(&self, keep_recent: usize) -> Vec<ChatMessage> {
        let total = self.turns.len();
        if total <= keep_recent {
            // 全部保留
            self.turns_to_messages(&self.turns)
        } else {
            // 早期摘要 + 近期完整
            let early = &self.turns[..total - keep_recent];
            let recent = &self.turns[total - keep_recent..];

            let mut messages = vec![ChatMessage::system(format!(
                "Earlier conversation ({} turns summarized)...",
                early.len()
            ))];
            messages.extend(self.turns_to_messages(recent));
            messages
        }
    }

    fn turns_to_messages(&self, turns: &[TurnRecord]) -> Vec<ChatMessage> {
        let mut messages = Vec::new();
        for turn in turns {
            messages.push(turn.user_message.clone());
            if let Some(assistant) = &turn.assistant_message {
                messages.push(assistant.clone());
            }
        }
        messages
    }
}

/// 单次对话回合记录
#[derive(Debug, Clone)]
pub struct TurnRecord {
    /// 用户消息
    pub user_message: ChatMessage,
    /// Assistant 回复（可能失败/取消则无）
    pub assistant_message: Option<ChatMessage>,
    /// 工具执行记录
    pub tool_trace: Vec<ToolTrace>,
    /// 本次 run 状态
    pub run_status: RunStatus,
    pub created: DateTime<Utc>,
}

/// 工具执行记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolTrace {
    pub tool_name: String,
    /// 截断/脱敏后的输入（≤500 chars）
    pub input_summary: String,
    /// 截断后的输出（≤1000 chars）
    pub output_summary: String,
    pub duration_ms: u64,
    pub success: bool,
    /// 第几轮工具循环（0-indexed）
    pub round: u32,
}

/// Run 执行状态
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Success,
    Failed(String),
    Cancelled,
}

/// 会话管理器
pub struct SessionManager {
    /// 数据库连接（TODO: 实现持久化后移除 allow）
    #[allow(dead_code)]
    db: Arc<Mutex<Connection>>,
    /// 最大保留回合数
    max_turns: usize,
    /// 近期完整保留数
    keep_recent: usize,
    /// 内存中的活跃会话缓存
    sessions: Mutex<std::collections::HashMap<String, AgentSession>>,
}

impl SessionManager {
    /// 创建新的 SessionManager
    pub fn new(db: Arc<Mutex<Connection>>) -> Self {
        Self {
            db,
            max_turns: 100,
            keep_recent: 5,
            sessions: Mutex::new(std::collections::HashMap::new()),
        }
    }

    /// 获取或创建会话
    pub async fn get_or_create(
        &self,
        sender: &str,
        mode: &ConversationMode,
        session_id: Option<&str>,
    ) -> Result<AgentSession, AppError> {
        let mut cache = self.sessions.lock().await;

        // 优先从缓存获取
        if let Some(id) = session_id {
            if let Some(session) = cache.get(id) {
                return Ok(session.clone());
            }
            // 尝试从数据库加载（TODO: 实现持久化）
        }

        // 创建新会话
        let session = match session_id {
            Some(id) => AgentSession::with_id(id.to_string(), sender.to_string(), mode.clone()),
            None => AgentSession::new(sender.to_string(), mode.clone()),
        };
        cache.insert(session.id.clone(), session.clone());

        Ok(session)
    }

    /// 获取会话（仅缓存）
    pub async fn get(&self, session_id: &str) -> Option<AgentSession> {
        let cache = self.sessions.lock().await;
        cache.get(session_id).cloned()
    }

    /// 创建新的会话并写入缓存
    pub async fn create_new(&self, sender: &str, mode: &ConversationMode) -> AgentSession {
        let session = AgentSession::new(sender.to_string(), mode.clone());
        let mut cache = self.sessions.lock().await;
        cache.insert(session.id.clone(), session.clone());
        session
    }

    /// 成功完成后追加 turn
    pub async fn append_turn(
        &self,
        session_id: &str,
        user_msg: ChatMessage,
        assistant_msg: Option<ChatMessage>,
        tool_trace: Vec<ToolTrace>,
    ) -> Result<(), AppError> {
        let mut cache = self.sessions.lock().await;

        let session = cache
            .get_mut(session_id)
            .ok_or_else(|| AppError::NotFound(format!("Session not found: {}", session_id)))?;

        let turn = TurnRecord {
            user_message: user_msg,
            assistant_message: assistant_msg,
            tool_trace,
            run_status: RunStatus::Success,
            created: Utc::now(),
        };

        session.append_turn(turn);

        // 如果超出最大回合数，进行裁剪
        if session.turns.len() > self.max_turns {
            session.turns = session.compressed_turns(self.keep_recent);
        }

        // TODO: 持久化到数据库

        Ok(())
    }

    /// 记录失败元数据（不保存半成品）
    pub async fn mark_failed(
        &self,
        session_id: &str,
        _request_id: &str,
        error: &str,
    ) -> Result<(), AppError> {
        let mut cache = self.sessions.lock().await;

        if let Some(session) = cache.get_mut(session_id) {
            // 更新最后访问时间
            session.updated = Utc::now();
            // TODO: 记录失败元数据到数据库
            tracing::error!(
                session_id = %session_id,
                error = %error,
                "session_marked_failed"
            );
        }

        Ok(())
    }

    /// 标记取消
    pub async fn mark_cancelled(&self, session_id: &str) -> Result<(), AppError> {
        let mut cache = self.sessions.lock().await;

        if let Some(session) = cache.get_mut(session_id) {
            session.updated = Utc::now();
            // TODO: 记录取消状态到数据库
            tracing::info!(session_id = %session_id, "session_marked_cancelled");
        }

        Ok(())
    }

    /// 获取会话的压缩后历史（用于构建 LLM 上下文）
    pub async fn get_compressed_history(
        &self,
        session_id: &str,
    ) -> Result<Vec<ChatMessage>, AppError> {
        let cache = self.sessions.lock().await;

        let session = cache
            .get(session_id)
            .cloned()
            .ok_or_else(|| AppError::NotFound(format!("Session not found: {}", session_id)))?;

        Ok(session.compressed_history(self.keep_recent))
    }

    /// 清理过期会话（释放内存）
    pub async fn prune_inactive_sessions(&self, max_age: chrono::Duration) {
        let mut cache = self.sessions.lock().await;
        let now = Utc::now();
        cache.retain(|_, session| now - session.updated < max_age);
    }
}

impl AgentSession {
    /// 压缩回合记录（保留近期完整，早期转为摘要）
    fn compressed_turns(&self, keep_recent: usize) -> Vec<TurnRecord> {
        let total = self.turns.len();
        if total <= keep_recent {
            return self.turns.clone();
        }

        // 只保留近期
        self.turns[total - keep_recent..].to_vec()
    }
}
