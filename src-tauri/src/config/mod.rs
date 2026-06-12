use crate::services::acp_client::AcpServer;
use crate::services::agent::{Agent, Skill, SlashCommand};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 应用配置。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub channels: HashMap<String, ChannelConfig>,
    pub acp_servers: Vec<AcpServer>,
    pub agents: Vec<Agent>,
    pub skills: Vec<Skill>,
    pub slash_commands: Vec<SlashCommand>,
    pub default_agent_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelConfig {
    pub enabled: bool,
    pub poll_interval_secs: u64,
    pub page_size: i32,
    pub auto_reply: bool,
    #[serde(default)]
    pub extra: serde_json::Value,
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
        channels.insert(
            "telegram".to_string(),
            ChannelConfig {
                enabled: false,
                poll_interval_secs: 30,
                page_size: 20,
                auto_reply: false,
                extra: serde_json::Value::Null,
            },
        );

        let default_agent = Agent::default_local();

        Self {
            channels,
            acp_servers: vec![AcpServer::default_local()],
            agents: vec![default_agent.clone()],
            skills: Vec::new(),
            slash_commands: Vec::new(),
            default_agent_id: default_agent.id,
        }
    }
}

impl AppConfig {
    pub fn load() -> Result<Self, crate::error::AppError> {
        Ok(Self::default())
    }

    pub fn get_channel_config(&self, channel: &str) -> ChannelConfig {
        self.channels
            .get(channel)
            .cloned()
            .unwrap_or(ChannelConfig {
                enabled: false,
                poll_interval_secs: 30,
                page_size: 20,
                auto_reply: false,
                extra: serde_json::Value::Null,
            })
    }
}
