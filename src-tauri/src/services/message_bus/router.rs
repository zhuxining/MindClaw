use super::types::{AgentRequest, AgentResponse, ChannelMessage, RouteRule};
use crate::error::AppError;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 消息处理结果回调：将回复消息回写到对应渠道
pub type ReplyCallback = Arc<dyn Fn(ChannelMessage) -> Result<(), AppError> + Send + Sync>;

/// Agent 调用接口：MessageBus 通过此 trait 调用 Agent
pub type AgentCallback = Arc<dyn Fn(AgentRequest) -> Result<AgentResponse, AppError> + Send + Sync>;

/// MessageBus 核心路由器
pub struct MessageBus {
    /// 路由规则列表
    rules: RwLock<Vec<RouteRule>>,
    /// 默认路由规则（无匹配规则时使用）
    default_rule: RwLock<Option<RouteRule>>,
}

impl MessageBus {
    pub fn new() -> Self {
        Self {
            rules: RwLock::new(Vec::new()),
            default_rule: RwLock::new(None),
        }
    }

    /// 注册路由规则
    pub async fn register_rule(&self, rule: RouteRule) {
        let mut rules = self.rules.write().await;
        // 去重：如果已存在同 ID 规则则替换
        if let Some(pos) = rules.iter().position(|r| r.rule_id == rule.rule_id) {
            rules[pos] = rule;
        } else {
            rules.push(rule);
        }
        // 按优先级排序
        rules.sort_by_key(|r| r.priority);
    }

    /// 移除路由规则
    pub async fn remove_rule(&self, rule_id: &str) {
        let mut rules = self.rules.write().await;
        rules.retain(|r| r.rule_id != rule_id);
    }

    /// 设置默认路由规则
    #[allow(dead_code)]
    pub async fn set_default_rule(&self, rule: Option<RouteRule>) {
        let mut default = self.default_rule.write().await;
        *default = rule;
    }

    /// 获取所有路由规则
    pub async fn get_rules(&self) -> Vec<RouteRule> {
        self.rules.read().await.clone()
    }

    /// 处理消息：匹配规则 → 调用 Agent → 回写回复
    pub async fn process_message(
        &self,
        msg: ChannelMessage,
        agent_callback: &AgentCallback,
        reply_callback: &ReplyCallback,
    ) -> Result<AgentResponse, AppError> {
        // 匹配路由规则
        let agent_id = self.match_route(&msg).await;

        let request = AgentRequest {
            request_id: uuid::Uuid::new_v4().to_string(),
            agent_id,
            message: msg.clone(),
            prompt: None,
        };

        // 调用 Agent
        let response = agent_callback(request)?;

        // 如果 Agent 返回了输出内容，回写到渠道
        if response.status == super::types::ResponseStatus::Success && !response.output.is_empty() {
            let reply = ChannelMessage {
                message_id: uuid::Uuid::new_v4().to_string(),
                channel: msg.channel.clone(),
                conversation_id: msg.conversation_id.clone(),
                sender_id: "agent".to_string(),
                sender_name: "MindClaw Agent".to_string(),
                content: response.output.clone(),
                timestamp: chrono::Utc::now().timestamp(),
                is_reply: true,
                reply_to: Some(msg.message_id.clone()),
            };
            // 回写失败不阻断主流程
            let _ = reply_callback(reply);
        }

        Ok(response)
    }

    /// 匹配消息到目标 Agent
    async fn match_route(&self, msg: &ChannelMessage) -> String {
        let rules = self.rules.read().await;
        for rule in rules.iter().filter(|r| r.enabled) {
            if rule.condition.matches(msg) {
                return rule.agent_id.clone();
            }
        }

        // 无匹配规则时使用默认规则
        let default = self.default_rule.read().await;
        if let Some(ref rule) = *default {
            return rule.agent_id.clone();
        }

        // 最终回退：使用 "default" Agent
        "default".to_string()
    }
}

impl Default for MessageBus {
    fn default() -> Self {
        Self::new()
    }
}
