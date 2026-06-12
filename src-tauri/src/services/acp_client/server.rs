use agent_client_protocol::schema::{EnvVariable, McpServer, McpServerStdio};
use serde::{Deserialize, Serialize};

/// MindClaw 中注册的 ACP Server 元数据。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AcpServer {
    pub id: String,
    pub name: String,
    pub description: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env_vars: Vec<EnvVar>,
    #[serde(default)]
    pub timeout_secs: u64,
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EnvVar {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AcpServerStatus {
    Available,
    Unavailable { reason: String },
    Disabled,
    Unknown,
}

impl AcpServer {
    pub fn default_local() -> Self {
        Self {
            id: "local-default".to_string(),
            name: "本地默认 ACP".to_string(),
            description: "MindClaw 默认本地 ACP Server".to_string(),
            command: "agent".to_string(),
            args: Vec::new(),
            env_vars: Vec::new(),
            timeout_secs: 120,
            enabled: true,
        }
    }

    /// 将 MindClaw 的 AcpServer 转换为 agent_client_protocol 的 McpServer。
    pub fn to_mcp_server(&self) -> McpServer {
        McpServer::Stdio(
            McpServerStdio::new(&self.name, std::path::PathBuf::from(&self.command))
                .args(self.args.clone())
                .env(
                    self.env_vars
                        .iter()
                        .map(|var| EnvVariable::new(&var.name, &var.value))
                        .collect(),
                )
                .meta(None),
        )
    }
}

impl Default for AcpServer {
    fn default() -> Self {
        Self::default_local()
    }
}
