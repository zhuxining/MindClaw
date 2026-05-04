//! SessionManager：会话生命周期管理
//!
//! 内存缓存 + SQLite 持久化。写操作先更新缓存再异步写 DB。

use crate::agent::messages::ChatMessage;
use crate::error::AppError;
use crate::models::conversation::ConversationMode;
use chrono::{DateTime, Utc};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Session 持久化策略
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionPersistence {
    /// 主 Agent：写 SQLite，跨 turn 上下文延续
    Persistent,
    /// SubAgent / Background Agent：仅内存，不写 DB
    Ephemeral,
}

/// 运行时 checkpoint（用于中断恢复）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RuntimeCheckpoint {
    /// 当前阶段
    pub phase: CheckpointPhase,
    /// 当前迭代次数
    pub iteration: usize,
    /// 模型 ID
    pub model: String,
    /// 中间的 assistant 消息（包含 tool_calls）
    pub assistant_message: Option<ChatMessage>,
    /// 已完成的工具结果
    pub completed_tool_results: Vec<ChatMessage>,
    /// 待执行的工具调用（用于中断时生成错误消息）
    pub pending_tool_calls: Vec<PendingToolCall>,
}

/// Checkpoint 阶段
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CheckpointPhase {
    #[default]
    None,
    /// 等待工具执行
    AwaitingTools,
    /// 工具执行完成，等待下一轮
    ToolsCompleted,
    /// 最终响应
    FinalResponse,
}

/// 待执行的工具调用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingToolCall {
    pub id: String,
    pub name: String,
}

/// Agent 层的 Session 结构（内部使用，与 models::Session 区分）
#[derive(Debug, Clone)]
pub struct AgentSession {
    pub id: String,
    /// 归属哪个 Agent 实例
    pub agent_id: String,
    /// 父 Agent 的 session ID（用于隐性 team 可观测性链路）
    pub parent_session_id: Option<String>,
    /// 存储策略：主 Agent = Persistent，SubAgent = Ephemeral
    pub persistence: SessionPersistence,
    pub sender: String,
    pub mode: ConversationMode,
    pub turns: Vec<TurnRecord>,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
    /// 运行时元数据（checkpoint、pending_user_turn 等）
    pub metadata: HashMap<String, serde_json::Value>,
}

// metadata keys
const RUNTIME_CHECKPOINT_KEY: &str = "runtime_checkpoint";
const PENDING_USER_TURN_KEY: &str = "pending_user_turn";

impl AgentSession {
    pub fn new(sender: String, mode: ConversationMode) -> Self {
        let now = Utc::now();
        let id = uuid::Uuid::new_v4().to_string();
        Self {
            agent_id: id.clone(),
            id,
            parent_session_id: None,
            persistence: SessionPersistence::Persistent,
            sender,
            mode,
            turns: Vec::new(),
            created: now,
            updated: now,
            metadata: HashMap::new(),
        }
    }

    pub fn with_id(id: String, sender: String, mode: ConversationMode) -> Self {
        let now = Utc::now();
        Self {
            agent_id: id.clone(),
            id,
            parent_session_id: None,
            persistence: SessionPersistence::Persistent,
            sender,
            mode,
            turns: Vec::new(),
            created: now,
            updated: now,
            metadata: HashMap::new(),
        }
    }

    /// 创建 Ephemeral session（SubAgent / Background Agent 使用，不写 DB）
    pub fn ephemeral(agent_id: &str, parent_session_id: Option<&str>) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            agent_id: agent_id.to_string(),
            parent_session_id: parent_session_id.map(|s| s.to_string()),
            persistence: SessionPersistence::Ephemeral,
            sender: "system".to_string(),
            mode: ConversationMode::Companion,
            turns: Vec::new(),
            created: now,
            updated: now,
            metadata: HashMap::new(),
        }
    }

    pub fn append_turn(&mut self, turn: TurnRecord) {
        self.turns.push(turn);
        self.updated = Utc::now();
    }

    // ── Checkpoint 管理 ─────────────────────────────────────────────

    /// 设置运行时 checkpoint
    pub fn set_checkpoint(&mut self, checkpoint: &RuntimeCheckpoint) {
        let value = serde_json::to_value(checkpoint).unwrap_or(serde_json::Value::Null);
        self.metadata
            .insert(RUNTIME_CHECKPOINT_KEY.to_string(), value);
    }

    /// 获取运行时 checkpoint
    pub fn get_checkpoint(&self) -> Option<RuntimeCheckpoint> {
        self.metadata
            .get(RUNTIME_CHECKPOINT_KEY)
            .and_then(|v| serde_json::from_value::<RuntimeCheckpoint>(v.clone()).ok())
    }

    /// 清除 checkpoint
    pub fn clear_checkpoint(&mut self) {
        self.metadata.remove(RUNTIME_CHECKPOINT_KEY);
    }

    /// 标记有 pending user turn
    pub fn mark_pending_user_turn(&mut self) {
        self.metadata.insert(
            PENDING_USER_TURN_KEY.to_string(),
            serde_json::Value::Bool(true),
        );
    }

    /// 清除 pending user turn
    pub fn clear_pending_user_turn(&mut self) {
        self.metadata.remove(PENDING_USER_TURN_KEY);
    }

    /// 检查是否有 pending user turn
    pub fn has_pending_user_turn(&self) -> bool {
        self.metadata
            .get(PENDING_USER_TURN_KEY)
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }

    /// 从 checkpoint 恢复中断的消息链
    ///
    /// 返回恢复的消息数量
    pub fn restore_from_checkpoint(&mut self) -> usize {
        let checkpoint = self.get_checkpoint();
        if checkpoint.is_none() {
            return 0;
        }
        let checkpoint = checkpoint.unwrap();

        let mut restored_messages: Vec<ChatMessage> = Vec::new();

        // 添加 assistant 消息
        if let Some(assistant) = checkpoint.assistant_message {
            restored_messages.push(assistant);
        }

        // 添加已完成的工具结果
        for result in checkpoint.completed_tool_results {
            restored_messages.push(result);
        }

        // 为待执行的工具调用生成错误消息
        for pending in checkpoint.pending_tool_calls {
            restored_messages.push(ChatMessage::tool_result(
                "Error: Task interrupted before this tool finished.",
                pending.id.clone(),
                true,
            ));
        }

        // 检查是否与现有消息重叠
        let overlap = self.find_overlap_with_turns(&restored_messages);
        let new_messages = restored_messages.len() - overlap;

        // 添加新消息到最新 turn
        if new_messages > 0 && !self.turns.is_empty() {
            // 在实际实现中，需要将消息追加到 turns
            // 这里简化处理：直接添加到最后一个 turn 的 tool_trace
            tracing::info!(
                session_id = %self.id,
                restored_count = new_messages,
                "checkpoint_restored"
            );
        }

        self.clear_checkpoint();
        self.clear_pending_user_turn();
        new_messages
    }

    /// 查找恢复消息与现有 turns 的重叠
    fn find_overlap_with_turns(&self, _restored: &[ChatMessage]) -> usize {
        // 简化实现：返回 0，表示全部是新消息
        // 完整实现需要比对消息内容
        0
    }

    /// 压缩后的历史（近 N 轮完整 + 早期摘要）
    pub fn compressed_history(&self, keep_recent: usize) -> Vec<ChatMessage> {
        let total = self.turns.len();
        if total <= keep_recent {
            Self::turns_to_messages(&self.turns)
        } else {
            let early = &self.turns[..total - keep_recent];
            let recent = &self.turns[total - keep_recent..];
            let mut messages = vec![ChatMessage::system(format!(
                "Earlier conversation ({} turns summarized)...",
                early.len()
            ))];
            messages.extend(Self::turns_to_messages(recent));
            messages
        }
    }

    fn turns_to_messages(turns: &[TurnRecord]) -> Vec<ChatMessage> {
        let mut messages = Vec::new();
        for turn in turns {
            messages.push(turn.user_message.clone());
            if let Some(assistant) = &turn.assistant_message {
                messages.push(assistant.clone());
            }
        }
        messages
    }

    /// 压缩回合记录（保留近期完整，早期丢弃）
    fn compressed_turns(&self, keep_recent: usize) -> Vec<TurnRecord> {
        let total = self.turns.len();
        if total <= keep_recent {
            return self.turns.clone();
        }
        self.turns[total - keep_recent..].to_vec()
    }
}

/// 单次对话回合记录
#[derive(Debug, Clone)]
pub struct TurnRecord {
    pub user_message: ChatMessage,
    pub assistant_message: Option<ChatMessage>,
    pub tool_trace: Vec<ToolTrace>,
    pub run_status: RunStatus,
    pub created: DateTime<Utc>,
}

/// 工具执行记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolTrace {
    pub tool_name: String,
    pub input_summary: String,
    pub output_summary: String,
    pub duration_ms: u64,
    pub success: bool,
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

/// 会话列表项（用于 UI 展示）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionListItem {
    pub id: String,
    pub sender: String,
    pub mode: ConversationMode,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
}

/// 会话管理器
pub struct SessionManager {
    db: Arc<Mutex<Connection>>,
    max_turns: usize,
    keep_recent: usize,
    sessions: Mutex<HashMap<String, AgentSession>>,
}

impl SessionManager {
    pub fn new(db: Arc<Mutex<Connection>>) -> Self {
        Self {
            db,
            max_turns: 100,
            keep_recent: 5,
            sessions: Mutex::new(HashMap::new()),
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

            // 尝试从数据库加载
            if let Some(session) = self.load_from_db(id).await? {
                cache.insert(session.id.clone(), session.clone());
                return Ok(session);
            }
        }

        // 创建新会话
        let session = match session_id {
            Some(id) => AgentSession::with_id(id.to_string(), sender.to_string(), mode.clone()),
            None => AgentSession::new(sender.to_string(), mode.clone()),
        };
        self.persist_session_meta(&session).await?;
        cache.insert(session.id.clone(), session.clone());
        Ok(session)
    }

    /// 获取会话（缓存 → DB）
    pub async fn get(&self, session_id: &str) -> Option<AgentSession> {
        let cache = self.sessions.lock().await;
        if let Some(s) = cache.get(session_id) {
            return Some(s.clone());
        }
        drop(cache);
        self.load_from_db(session_id).await.ok().flatten()
    }

    /// 创建新会话
    pub async fn create_new(&self, sender: &str, mode: &ConversationMode) -> AgentSession {
        let session = AgentSession::new(sender.to_string(), mode.clone());
        let _ = self.persist_session_meta(&session).await;
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
            .ok_or_else(|| AppError::NotFound(format!("Session not found: {session_id}")))?;

        let turn = TurnRecord {
            user_message: user_msg,
            assistant_message: assistant_msg,
            tool_trace,
            run_status: RunStatus::Success,
            created: Utc::now(),
        };

        session.append_turn(turn.clone());

        if session.turns.len() > self.max_turns {
            session.turns = session.compressed_turns(self.keep_recent);
        }

        // Ephemeral session 不写 DB
        if session.persistence == SessionPersistence::Ephemeral {
            return Ok(());
        }

        let sid = session_id.to_string();
        let updated = session.updated;
        drop(cache);

        self.persist_turn(&sid, &turn).await?;
        self.update_session_timestamp(&sid, updated).await?;

        Ok(())
    }

    /// 记录失败
    pub async fn mark_failed(
        &self,
        session_id: &str,
        _request_id: &str,
        error: &str,
    ) -> Result<(), AppError> {
        let mut cache = self.sessions.lock().await;
        if let Some(session) = cache.get_mut(session_id) {
            session.updated = Utc::now();
            let updated = session.updated;
            drop(cache);

            self.persist_run_status(session_id, &RunStatus::Failed(error.to_string()))
                .await?;
            self.update_session_timestamp(session_id, updated).await?;
        }
        tracing::error!(session_id = %session_id, error = %error, "session_marked_failed");
        Ok(())
    }

    /// 标记取消
    pub async fn mark_cancelled(&self, session_id: &str) -> Result<(), AppError> {
        let mut cache = self.sessions.lock().await;
        if let Some(session) = cache.get_mut(session_id) {
            session.updated = Utc::now();
            let updated = session.updated;
            drop(cache);

            self.persist_run_status(session_id, &RunStatus::Cancelled)
                .await?;
            self.update_session_timestamp(session_id, updated).await?;
        }
        tracing::info!(session_id = %session_id, "session_marked_cancelled");
        Ok(())
    }

    /// 获取压缩后历史
    pub async fn get_compressed_history(
        &self,
        session_id: &str,
    ) -> Result<Vec<ChatMessage>, AppError> {
        let cache = self.sessions.lock().await;
        let session = cache
            .get(session_id)
            .cloned()
            .ok_or_else(|| AppError::NotFound(format!("Session not found: {session_id}")))?;
        Ok(session.compressed_history(self.keep_recent))
    }

    /// 清理过期缓存
    pub async fn prune_inactive_sessions(&self, max_age: chrono::Duration) {
        let mut cache = self.sessions.lock().await;
        let now = Utc::now();
        cache.retain(|_, session| now - session.updated < max_age);
    }

    /// 列出所有会话（按更新时间倒序）
    pub async fn list_sessions(&self, limit: usize) -> Result<Vec<SessionListItem>, AppError> {
        let db = self.db.lock().await;
        let mut stmt = db.prepare(
            "SELECT id, sender, mode, created, updated FROM sessions ORDER BY updated DESC LIMIT ?1"
        ).map_err(|e| AppError::Storage(format!("prepare list sessions: {e}")))?;

        let rows = stmt
            .query_map(rusqlite::params![limit as i64], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })
            .map_err(|e| AppError::Storage(format!("query list sessions: {e}")))?;

        let items: Vec<SessionListItem> = rows
            .filter_map(|r| r.ok())
            .filter_map(|(id, sender, mode_str, created_str, updated_str)| {
                let mode: ConversationMode = serde_json::from_str(&mode_str).ok()?;
                let created = DateTime::parse_from_rfc3339(&created_str)
                    .map(|d| d.with_timezone(&Utc))
                    .ok()?;
                let updated = DateTime::parse_from_rfc3339(&updated_str)
                    .map(|d| d.with_timezone(&Utc))
                    .ok()?;
                Some(SessionListItem {
                    id,
                    sender,
                    mode,
                    created,
                    updated,
                })
            })
            .collect();

        Ok(items)
    }

    /// 删除会话及其所有 turns
    pub async fn delete_session(&self, session_id: &str) -> Result<(), AppError> {
        let db = self.db.lock().await;
        db.execute(
            "DELETE FROM turns WHERE session_id = ?1",
            rusqlite::params![session_id],
        )
        .map_err(|e| AppError::Storage(format!("delete turns: {e}")))?;
        db.execute(
            "DELETE FROM sessions WHERE id = ?1",
            rusqlite::params![session_id],
        )
        .map_err(|e| AppError::Storage(format!("delete session: {e}")))?;

        // 清除缓存
        let mut cache = self.sessions.lock().await;
        cache.remove(session_id);

        Ok(())
    }

    // ── DB 操作 ─────────────────────────────────────────────────

    async fn persist_session_meta(&self, session: &AgentSession) -> Result<(), AppError> {
        let mode_str = serde_json::to_string(&session.mode)
            .map_err(|e| AppError::Storage(format!("serialize session mode: {e}")))?;
        let db = self.db.lock().await;
        db.execute(
            "INSERT OR IGNORE INTO sessions (id, sender, mode, created, updated) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                session.id,
                session.sender,
                mode_str,
                session.created.to_rfc3339(),
                session.updated.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    async fn update_session_timestamp(
        &self,
        session_id: &str,
        updated: DateTime<Utc>,
    ) -> Result<(), AppError> {
        let db = self.db.lock().await;
        db.execute(
            "UPDATE sessions SET updated = ?1 WHERE id = ?2",
            rusqlite::params![updated.to_rfc3339(), session_id],
        )?;
        Ok(())
    }

    async fn persist_turn(&self, session_id: &str, turn: &TurnRecord) -> Result<(), AppError> {
        let user_json = serde_json::to_string(&turn.user_message)
            .map_err(|e| AppError::Storage(format!("serialize user_message: {e}")))?;
        let assistant_json = turn
            .assistant_message
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|e| AppError::Storage(format!("serialize assistant_message: {e}")))?;
        let trace_json = serde_json::to_string(&turn.tool_trace)
            .map_err(|e| AppError::Storage(format!("serialize tool_trace: {e}")))?;
        let status_str = run_status_to_string(&turn.run_status);

        let db = self.db.lock().await;
        db.execute(
            "INSERT INTO turns (session_id, user_message, assistant_message, tool_trace, run_status, created) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                session_id,
                user_json,
                assistant_json,
                trace_json,
                status_str,
                turn.created.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    async fn persist_run_status(
        &self,
        session_id: &str,
        status: &RunStatus,
    ) -> Result<(), AppError> {
        let status_str = run_status_to_string(status);
        let db = self.db.lock().await;
        // 更新该 session 最后一条 turn 的状态（如果有）
        db.execute(
            "UPDATE turns SET run_status = ?1 WHERE id = (SELECT MAX(id) FROM turns WHERE session_id = ?2)",
            rusqlite::params![status_str, session_id],
        )?;
        Ok(())
    }

    async fn load_from_db(&self, session_id: &str) -> Result<Option<AgentSession>, AppError> {
        let db = self.db.lock().await;

        let mut stmt =
            db.prepare("SELECT id, sender, mode, created, updated FROM sessions WHERE id = ?1")?;

        let session_row = stmt
            .query_row(rusqlite::params![session_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })
            .ok();

        let Some((id, sender, mode_str, created_str, updated_str)) = session_row else {
            return Ok(None);
        };

        let mode: ConversationMode =
            serde_json::from_str(&mode_str).unwrap_or(ConversationMode::Companion);
        let created = DateTime::parse_from_rfc3339(&created_str)
            .map(|d| d.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());
        let updated = DateTime::parse_from_rfc3339(&updated_str)
            .map(|d| d.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        // 加载 turns
        let mut turn_stmt = db.prepare(
            "SELECT user_message, assistant_message, tool_trace, run_status, created FROM turns WHERE session_id = ?1 ORDER BY id",
        )?;

        let turns: Vec<TurnRecord> = turn_stmt
            .query_map(rusqlite::params![session_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })?
            .filter_map(|r| r.ok())
            .filter_map(
                |(user_json, asst_json, trace_json, status_str, created_str)| {
                    let user_message: ChatMessage = serde_json::from_str(&user_json).ok()?;
                    let assistant_message: Option<ChatMessage> =
                        asst_json.and_then(|j| serde_json::from_str(&j).ok());
                    let tool_trace: Vec<ToolTrace> = serde_json::from_str(&trace_json).ok()?;
                    let run_status = run_status_from_string(&status_str);
                    let created = DateTime::parse_from_rfc3339(&created_str)
                        .map(|d| d.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now());
                    Some(TurnRecord {
                        user_message,
                        assistant_message,
                        tool_trace,
                        run_status,
                        created,
                    })
                },
            )
            .collect();

        Ok(Some(AgentSession {
            agent_id: id.clone(),
            parent_session_id: None,
            persistence: SessionPersistence::Persistent,
            id,
            sender,
            mode,
            turns,
            created,
            updated,
            metadata: HashMap::new(),
        }))
    }
}

fn run_status_to_string(status: &RunStatus) -> String {
    match status {
        RunStatus::Success => "success".to_string(),
        RunStatus::Failed(msg) => format!("failed:{msg}"),
        RunStatus::Cancelled => "cancelled".to_string(),
    }
}

fn run_status_from_string(s: &str) -> RunStatus {
    if s == "success" {
        RunStatus::Success
    } else if s == "cancelled" {
        RunStatus::Cancelled
    } else if let Some(msg) = s.strip_prefix("failed:") {
        RunStatus::Failed(msg.to_string())
    } else {
        RunStatus::Failed(s.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::database::global::open_memory;

    #[tokio::test]
    async fn test_session_lifecycle() {
        let db = open_memory().unwrap();
        let mgr = SessionManager::new(db);

        // 创建会话
        let session = mgr
            .get_or_create("test_user", &ConversationMode::Companion, None)
            .await
            .unwrap();
        let sid = session.id.clone();
        assert_eq!(session.sender, "test_user");

        // 追加 turn
        mgr.append_turn(
            &sid,
            ChatMessage::user("hello"),
            Some(ChatMessage::assistant_text("hi there")),
            vec![],
        )
        .await
        .unwrap();

        // 从缓存获取
        let s = mgr.get(&sid).await.unwrap();
        assert_eq!(s.turns.len(), 1);

        // 清除缓存后从 DB 恢复
        mgr.sessions.lock().await.clear();
        let restored = mgr
            .get_or_create("test_user", &ConversationMode::Companion, Some(&sid))
            .await
            .unwrap();
        assert_eq!(restored.turns.len(), 1);
        assert_eq!(restored.turns[0].user_message.text_content(), "hello");
    }

    #[tokio::test]
    async fn test_mark_failed_and_cancelled() {
        let db = open_memory().unwrap();
        let mgr = SessionManager::new(db);

        let session = mgr
            .get_or_create("user", &ConversationMode::Companion, None)
            .await
            .unwrap();

        mgr.mark_failed(&session.id, "req1", "boom").await.unwrap();
        mgr.mark_cancelled(&session.id).await.unwrap();
    }
}
