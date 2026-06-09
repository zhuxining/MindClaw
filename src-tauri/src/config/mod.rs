use crate::services::message_bus::RouteRule;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 应用配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// 飞书配置（deprecated，保留向后兼容，v2 迁移到 channels["feishu"]）
    pub feishu: FeishuConfig,
    /// 渠道通用配置（key = 渠道名称如 "feishu"、"dingtalk"）
    pub channels: HashMap<String, ChannelConfig>,
    /// 消息总线配置
    pub message_bus: MessageBusConfig,
    /// ACP Agent 配置
    pub acp_agent: AcpAgentConfig,
}

/// 渠道配置（通用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelConfig {
    /// 是否启用此渠道
    pub enabled: bool,
    /// 消息轮询间隔（秒）
    pub poll_interval_secs: u64,
    /// 每页拉取消息数
    pub page_size: i32,
    /// 是否自动回复
    pub auto_reply: bool,
    /// 渠道特有额外配置（JSON）
    #[serde(default)]
    pub extra: serde_json::Value,
}

/// 飞书配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeishuConfig {
    /// 消息轮询间隔（秒）
    pub poll_interval_secs: u64,
    /// 每页拉取消息数
    pub page_size: i32,
    /// 是否自动回复飞书
    pub auto_reply: bool,
}

/// MessageBus 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageBusConfig {
    /// 默认路由规则
    pub default_rules: Vec<RouteRule>,
}

/// ACP Agent 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpAgentConfig {
    /// Agent 可执行文件路径
    pub agent_path: String,
    /// 请求超时时间（秒）
    pub timeout_secs: u64,
}

impl Default for AppConfig {
    fn default() -> Self {
        let mut channels = HashMap::new();
        channels.insert(
            "feishu".to_string(),
            ChannelConfig {
                enabled: true,
                poll_interval_secs: 30,
                page_size: 20,
                auto_reply: true,
                extra: serde_json::Value::Null,
            },
        );

        Self {
            feishu: FeishuConfig {
                poll_interval_secs: 30,
                page_size: 20,
                auto_reply: true,
            },
            channels,
            message_bus: MessageBusConfig {
                default_rules: vec![],
            },
            acp_agent: AcpAgentConfig {
                agent_path: "agent".to_string(),
                timeout_secs: 120,
            },
        }
    }
}

impl AppConfig {
    /// 从配置文件加载
    pub fn load() -> Result<Self, crate::error::AppError> {
        // v1: 使用默认配置
        // v2: 从文件系统加载
        Ok(Self::default())
    }

    /// 获取指定渠道的通用配置（回退到飞书配置以保持向后兼容）
    #[allow(dead_code)]
    pub fn get_channel_config(&self, channel: &str) -> ChannelConfig {
        self.channels.get(channel).cloned().unwrap_or_else(|| {
            // 向后兼容：对 "feishu" 使用旧配置
            if channel == "feishu" {
                ChannelConfig {
                    enabled: true,
                    poll_interval_secs: self.feishu.poll_interval_secs,
                    page_size: self.feishu.page_size,
                    auto_reply: self.feishu.auto_reply,
                    extra: serde_json::Value::Null,
                }
            } else {
                ChannelConfig {
                    enabled: false,
                    poll_interval_secs: 30,
                    page_size: 20,
                    auto_reply: false,
                    extra: serde_json::Value::Null,
                }
            }
        })
    }
}
