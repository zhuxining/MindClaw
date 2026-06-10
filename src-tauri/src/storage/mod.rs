use crate::services::message_bus::ChannelMessage;
use std::collections::HashSet;
use std::sync::Mutex;

/// 简单内存存储（v1），后续迁移到 SQLite
pub struct MessageStore {
    /// 已处理的消息 ID 集合（用于去重）
    seen_ids: Mutex<HashSet<String>>,
    /// 消息列表（内存中）
    messages: Mutex<Vec<ChannelMessage>>,
}

impl MessageStore {
    pub fn new() -> Self {
        Self {
            seen_ids: Mutex::new(HashSet::new()),
            messages: Mutex::new(Vec::new()),
        }
    }

    /// 检查并标记消息为已处理，返回 true 表示是新消息
    pub fn check_and_mark_seen(&self, message_id: &str) -> bool {
        let mut seen = self.seen_ids.lock().unwrap();
        if seen.contains(message_id) {
            false
        } else {
            seen.insert(message_id.to_string());
            true
        }
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
    }
}

impl Default for MessageStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
