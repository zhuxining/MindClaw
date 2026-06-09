use serde::{Deserialize, Serialize};

/// 统一渠道消息 — 所有渠道的消息都转换为此格式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelMessage {
    /// 唯一消息 ID（来自原始渠道，用于去重）
    pub message_id: String,
    /// 来源渠道标识（如 "feishu"）
    pub channel: String,
    /// 会话/群聊 ID
    pub conversation_id: String,
    /// 发送者标识
    pub sender_id: String,
    /// 发送者显示名称
    pub sender_name: String,
    /// 消息文本内容
    pub content: String,
    /// 消息时间戳（Unix 秒）
    pub timestamp: i64,
    /// 是否为 Agent 回复（回写时标记）
    pub is_reply: bool,
    /// 引用消息 ID（回复时使用）
    pub reply_to: Option<String>,
}

/// 消息路由规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteRule {
    /// 规则唯一 ID
    pub rule_id: String,
    /// 规则名称
    pub name: String,
    /// 匹配条件
    pub condition: RouteCondition,
    /// 目标 Agent ID
    pub agent_id: String,
    /// 优先级（数值越小优先级越高）
    pub priority: u32,
    /// 是否启用
    pub enabled: bool,
}

/// 路由匹配条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteCondition {
    /// 匹配渠道（None = 所有渠道）
    pub channel: Option<String>,
    /// 匹配发送者 ID（None = 所有发送者）
    pub sender_id: Option<String>,
    /// 关键词匹配（消息内容包含任一关键词即匹配，None = 不限制）
    pub keywords: Option<Vec<String>>,
    /// 关键词匹配模式：contains（包含）/ not_contains（不包含）/ regex（正则，v2）
    pub keyword_mode: MatchMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MatchMode {
    #[serde(rename = "contains")]
    Contains,
    #[serde(rename = "not_contains")]
    NotContains,
}

/// 发送给 Agent 的处理请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRequest {
    /// 请求唯一 ID
    pub request_id: String,
    /// 目标 Agent ID
    pub agent_id: String,
    /// 原始消息内容
    pub message: ChannelMessage,
    /// 处理提示词（可选）
    pub prompt: Option<String>,
}

/// Agent 处理结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResponse {
    /// 对应的请求 ID
    pub request_id: String,
    /// 处理状态
    pub status: ResponseStatus,
    /// 输出内容
    pub output: String,
    /// 错误信息（status 为 Error 时）
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ResponseStatus {
    #[serde(rename = "success")]
    Success,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "timeout")]
    Timeout,
}

impl RouteCondition {
    /// 检查消息是否匹配此条件
    pub fn matches(&self, msg: &ChannelMessage) -> bool {
        // 渠道匹配
        if let Some(ref channel) = self.channel {
            if &msg.channel != channel {
                return false;
            }
        }

        // 发送者匹配
        if let Some(ref sender_id) = self.sender_id {
            if &msg.sender_id != sender_id {
                return false;
            }
        }

        // 关键词匹配
        if let Some(ref keywords) = self.keywords {
            if keywords.is_empty() {
                return true;
            }
            let content_lower = msg.content.to_lowercase();
            match self.keyword_mode {
                MatchMode::Contains => {
                    if !keywords
                        .iter()
                        .any(|kw| content_lower.contains(&kw.to_lowercase()))
                    {
                        return false;
                    }
                }
                MatchMode::NotContains => {
                    if keywords
                        .iter()
                        .any(|kw| content_lower.contains(&kw.to_lowercase()))
                    {
                        return false;
                    }
                }
            }
        }

        true
    }
}
