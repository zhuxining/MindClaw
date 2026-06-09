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
