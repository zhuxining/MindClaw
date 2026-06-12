use crate::services::acp_client::AcpServer;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub description: String,
    pub identity: Identity,
    pub default_acp_server_id: String,
    pub default_skill_id: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Identity {
    pub system_prompt: String,
    pub style: Option<String>,
    pub safety_policy: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Skill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub instruction: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentSkillBinding {
    pub agent_id: String,
    pub skill_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SlashCommand {
    pub command: String,
    pub description: String,
    pub agent_id: String,
    pub skill_id: Option<String>,
    pub scope: SlashCommandScope,
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SlashCommandScope {
    OneShot,
    StickyConversation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ConversationKey {
    pub channel: String,
    pub conversation_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConversationExecutionState {
    pub key: ConversationKey,
    pub agent_id: String,
    pub skill_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub agent: Agent,
    pub skill: Option<Skill>,
    /// 保留给后续直接按 ACP Server 调用的场景（当前由 AcpClient 独立管理连接）
    #[allow(dead_code)]
    pub acp_server: AcpServer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlashCommandAction {
    Execute {
        agent_id: String,
        skill_id: Option<String>,
        content: String,
    },
    SwitchAgent {
        agent_id: String,
    },
    SelectSkill {
        skill_id: String,
    },
    ResetConversation,
    Help,
    PlainText(String),
}

impl Agent {
    pub fn default_local() -> Self {
        Self {
            id: "default-agent".to_string(),
            name: "默认 Agent".to_string(),
            description: "处理未指定命令的默认消息".to_string(),
            identity: Identity {
                system_prompt: "你是 MindClaw 的默认本地助手。".to_string(),
                style: None,
                safety_policy: None,
            },
            default_acp_server_id: "local-default".to_string(),
            default_skill_id: None,
            enabled: true,
        }
    }
}
