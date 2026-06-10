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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::message_bus::types::{MatchMode, ResponseStatus, RouteCondition};
    use std::sync::atomic::{AtomicBool, Ordering};

    fn message(content: &str) -> ChannelMessage {
        ChannelMessage {
            message_id: "msg-1".to_string(),
            channel: "feishu".to_string(),
            conversation_id: "chat-1".to_string(),
            sender_id: "user-1".to_string(),
            sender_name: "User".to_string(),
            content: content.to_string(),
            timestamp: 1,
            is_reply: false,
            reply_to: None,
        }
    }

    fn rule(rule_id: &str, agent_id: &str, priority: u32, keyword: &str) -> RouteRule {
        RouteRule {
            rule_id: rule_id.to_string(),
            name: rule_id.to_string(),
            condition: RouteCondition {
                channel: None,
                sender_id: None,
                keywords: Some(vec![keyword.to_string()]),
                keyword_mode: MatchMode::Contains,
            },
            agent_id: agent_id.to_string(),
            priority,
            enabled: true,
        }
    }

    #[tokio::test]
    async fn register_rule_replaces_existing_rule_and_sorts_by_priority() {
        let bus = MessageBus::new();

        bus.register_rule(rule("r-1", "slow", 20, "help")).await;
        bus.register_rule(rule("r-2", "fast", 10, "urgent")).await;
        bus.register_rule(rule("r-1", "replaced", 30, "other"))
            .await;

        let rules = bus.get_rules().await;
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].rule_id, "r-2");
        assert_eq!(rules[1].rule_id, "r-1");
        assert_eq!(rules[1].agent_id, "replaced");
    }

    #[tokio::test]
    async fn remove_rule_drops_rule_by_id() {
        let bus = MessageBus::new();

        bus.register_rule(rule("r-1", "agent-1", 10, "help")).await;
        bus.remove_rule("r-1").await;

        assert!(bus.get_rules().await.is_empty());
    }

    #[tokio::test]
    async fn match_route_uses_first_enabled_matching_rule() {
        let bus = MessageBus::new();
        bus.register_rule(rule("r-1", "low", 20, "help")).await;
        bus.register_rule(rule("r-2", "high", 10, "help")).await;

        assert_eq!(bus.match_route(&message("help me")).await, "high");
    }

    #[tokio::test]
    async fn match_route_ignores_disabled_rules() {
        let bus = MessageBus::new();
        let mut disabled = rule("r-1", "disabled", 10, "help");
        disabled.enabled = false;
        bus.register_rule(disabled).await;
        bus.register_rule(rule("r-2", "enabled", 20, "help")).await;

        assert_eq!(bus.match_route(&message("help me")).await, "enabled");
    }

    #[tokio::test]
    async fn match_route_uses_default_rule_then_default_agent() {
        let bus = MessageBus::new();

        assert_eq!(bus.match_route(&message("hello")).await, "default");

        bus.set_default_rule(Some(rule("default-rule", "fallback", 100, "anything")))
            .await;

        assert_eq!(bus.match_route(&message("hello")).await, "fallback");
    }

    #[tokio::test]
    async fn process_message_calls_agent_and_sends_successful_reply() {
        let bus = MessageBus::new();
        let reply_called = Arc::new(AtomicBool::new(false));
        let reply_called_for_callback = reply_called.clone();

        let agent_callback: AgentCallback = Arc::new(|request| {
            Ok(AgentResponse {
                request_id: request.request_id,
                status: ResponseStatus::Success,
                output: "reply".to_string(),
                error_message: None,
            })
        });
        let reply_callback: ReplyCallback = Arc::new(move |reply| {
            assert_eq!(reply.channel, "feishu");
            assert_eq!(reply.conversation_id, "chat-1");
            assert_eq!(reply.sender_id, "agent");
            assert_eq!(reply.content, "reply");
            assert_eq!(reply.reply_to.as_deref(), Some("msg-1"));
            reply_called_for_callback.store(true, Ordering::SeqCst);
            Ok(())
        });

        let response = bus
            .process_message(message("hello"), &agent_callback, &reply_callback)
            .await
            .unwrap();

        assert_eq!(response.status, ResponseStatus::Success);
        assert!(reply_called.load(Ordering::SeqCst));
    }
}
