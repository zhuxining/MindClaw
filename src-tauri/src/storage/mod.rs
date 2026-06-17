use crate::services::core::ChannelMessage;
use rusqlite::{params, Connection};
use std::collections::HashSet;
use std::path::Path;
use std::sync::{Arc, Mutex};

pub type SharedDatabase = Arc<Mutex<Connection>>;

pub fn open_database(path: impl AsRef<Path>) -> Result<SharedDatabase, rusqlite::Error> {
    Ok(Arc::new(Mutex::new(Connection::open(path)?)))
}

/// 消息存储。
///
/// 内存模式使用 HashSet 去重；SQLite 模式按需查询/写入，不在启动时全量加载历史。
pub struct MessageStore {
    seen_ids: Mutex<HashSet<String>>,
    messages: Mutex<Vec<ChannelMessage>>,
    database: Option<SharedDatabase>,
}

impl MessageStore {
    pub fn new() -> Self {
        Self {
            seen_ids: Mutex::new(HashSet::new()),
            messages: Mutex::new(Vec::new()),
            database: None,
        }
    }

    #[allow(dead_code)]
    pub fn new_persistent(path: impl AsRef<Path>) -> Result<Self, rusqlite::Error> {
        Self::new_with_database(open_database(path)?)
    }

    pub fn new_with_database(database: SharedDatabase) -> Result<Self, rusqlite::Error> {
        database.lock().unwrap().execute_batch(
            "CREATE TABLE IF NOT EXISTS seen_message_ids (
                message_id TEXT PRIMARY KEY,
                seen_at TEXT NOT NULL DEFAULT (datetime('now'))
            );",
        )?;

        Ok(Self {
            seen_ids: Mutex::new(HashSet::new()),
            messages: Mutex::new(Vec::new()),
            database: Some(database),
        })
    }

    /// 检查并标记消息为已处理，返回 true 表示是新消息。
    pub fn check_and_mark_seen(&self, message_id: &str) -> bool {
        if let Some(database) = &self.database {
            return check_and_mark_seen_in_database(&database.lock().unwrap(), message_id);
        }

        let mut seen = self.seen_ids.lock().unwrap();
        if seen.contains(message_id) {
            false
        } else {
            seen.insert(message_id.to_string());
            true
        }
    }

    /// 批量过滤并标记新消息。
    #[allow(dead_code)]
    pub fn filter_new_messages(&self, messages: Vec<ChannelMessage>) -> Vec<ChannelMessage> {
        if let Some(database) = &self.database {
            return filter_new_messages_in_database(&mut database.lock().unwrap(), messages);
        }

        let mut seen = self.seen_ids.lock().unwrap();
        messages
            .into_iter()
            .filter(|message| {
                if seen.contains(&message.message_id) {
                    false
                } else {
                    seen.insert(message.message_id.clone());
                    true
                }
            })
            .collect()
    }

    /// 保存消息
    pub fn save_message(&self, msg: ChannelMessage) {
        let mut messages = self.messages.lock().unwrap();
        messages.push(msg);
    }

    /// 获取所有消息
    pub fn get_messages(&self) -> Vec<ChannelMessage> {
        self.messages.lock().unwrap().clone()
    }

    /// 获取最近 N 条消息
    pub fn get_recent_messages(&self, limit: usize) -> Vec<ChannelMessage> {
        let messages = self.messages.lock().unwrap();
        let start = if messages.len() > limit {
            messages.len() - limit
        } else {
            0
        };
        messages[start..].to_vec()
    }

    /// 清除所有数据
    pub fn clear(&self) {
        self.seen_ids.lock().unwrap().clear();
        self.messages.lock().unwrap().clear();
        if let Some(database) = &self.database {
            let _ = database
                .lock()
                .unwrap()
                .execute("DELETE FROM seen_message_ids", []);
        }
    }
}

impl Default for MessageStore {
    fn default() -> Self {
        Self::new()
    }
}

fn check_and_mark_seen_in_database(connection: &Connection, message_id: &str) -> bool {
    connection
        .execute(
            "INSERT OR IGNORE INTO seen_message_ids (message_id) VALUES (?1)",
            params![message_id],
        )
        .map(|rows| rows > 0)
        .unwrap_or(false)
}

#[allow(dead_code)]
fn filter_new_messages_in_database(
    connection: &mut Connection,
    messages: Vec<ChannelMessage>,
) -> Vec<ChannelMessage> {
    let Ok(transaction) = connection.transaction() else {
        return messages
            .into_iter()
            .filter(|message| check_and_mark_seen_in_database(connection, &message.message_id))
            .collect();
    };

    let mut new_messages = Vec::new();
    for message in messages {
        let is_new = transaction
            .execute(
                "INSERT OR IGNORE INTO seen_message_ids (message_id) VALUES (?1)",
                params![message.message_id],
            )
            .map(|rows| rows > 0)
            .unwrap_or(false);
        if is_new {
            new_messages.push(message);
        }
    }
    let _ = transaction.commit();
    new_messages
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn message(id: &str) -> ChannelMessage {
        ChannelMessage {
            message_id: id.to_string(),
            channel: "feishu".to_string(),
            conversation_id: "chat-1".to_string(),
            sender_id: "user-1".to_string(),
            sender_name: "User".to_string(),
            content: format!("message {id}"),
            timestamp: 1,
            is_reply: false,
            reply_to: None,
        }
    }

    #[test]
    fn check_and_mark_seen_returns_false_for_duplicate_message_id() {
        let store = MessageStore::new();

        assert!(store.check_and_mark_seen("msg-1"));
        assert!(!store.check_and_mark_seen("msg-1"));
    }

    #[test]
    fn save_message_keeps_messages_in_insert_order() {
        let store = MessageStore::new();

        store.save_message(message("msg-1"));
        store.save_message(message("msg-2"));

        let messages = store.get_messages();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].message_id, "msg-1");
        assert_eq!(messages[1].message_id, "msg-2");
    }

    #[test]
    fn get_recent_messages_returns_requested_tail() {
        let store = MessageStore::new();

        store.save_message(message("msg-1"));
        store.save_message(message("msg-2"));
        store.save_message(message("msg-3"));

        let recent = store.get_recent_messages(2);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].message_id, "msg-2");
        assert_eq!(recent[1].message_id, "msg-3");
    }

    #[test]
    fn get_recent_messages_returns_all_when_limit_exceeds_len() {
        let store = MessageStore::new();

        store.save_message(message("msg-1"));
        store.save_message(message("msg-2"));

        let recent = store.get_recent_messages(10);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].message_id, "msg-1");
        assert_eq!(recent[1].message_id, "msg-2");
    }

    #[test]
    fn clear_removes_messages_and_seen_ids() {
        let store = MessageStore::new();

        assert!(store.check_and_mark_seen("msg-1"));
        store.save_message(message("msg-1"));
        store.clear();

        assert!(store.get_messages().is_empty());
        assert!(store.check_and_mark_seen("msg-1"));
    }

    #[test]
    fn persistent_store_restores_seen_ids_after_reopen() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_path_buf();

        let store = MessageStore::new_persistent(&path).unwrap();
        assert!(store.check_and_mark_seen("msg-1"));
        assert!(!store.check_and_mark_seen("msg-1"));
        drop(store);

        let reopened = MessageStore::new_persistent(&path).unwrap();
        assert!(!reopened.check_and_mark_seen("msg-1"));
        assert!(reopened.check_and_mark_seen("msg-2"));
    }

    #[test]
    fn persistent_clear_removes_seen_ids_from_database() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_path_buf();

        let store = MessageStore::new_persistent(&path).unwrap();
        assert!(store.check_and_mark_seen("msg-1"));
        store.clear();
        drop(store);

        let reopened = MessageStore::new_persistent(&path).unwrap();
        assert!(reopened.check_and_mark_seen("msg-1"));
    }
}
