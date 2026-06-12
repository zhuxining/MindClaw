use crate::error::AppError;
use agent_client_protocol::schema::{EnvVariable, McpServer, McpServerStdio};
use serde::{Deserialize, Serialize};

const ALLOWED_COMMANDS: &[&str] = &["agent", "claude", "claude-code", "gemini", "qwen", "codex"];
const ALLOWED_ENV_PREFIXES: &[&str] = &[
    "ACP_",
    "ANTHROPIC_",
    "CLAUDE_",
    "GEMINI_",
    "GOOGLE_",
    "MINDCLAW_",
    "OPENAI_",
];
const BLOCKED_ENV_NAMES: &[&str] = &[
    "DYLD_INSERT_LIBRARIES",
    "IFS",
    "LD_AUDIT",
    "LD_LIBRARY_PATH",
    "LD_PRELOAD",
    "NODE_OPTIONS",
    "PATH",
    "PYTHONPATH",
    "RUSTFLAGS",
    "SHELL",
];
const MAX_ARG_LEN: usize = 512;
const MAX_ENV_VALUE_LEN: usize = 4096;
const MAX_TIMEOUT_SECS: u64 = 600;

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

    pub fn validate(&self) -> Result<(), AppError> {
        validate_command(&self.command)?;
        for arg in &self.args {
            validate_arg(arg)?;
        }
        for var in &self.env_vars {
            validate_env_var(var)?;
        }
        if self.timeout_secs == 0 || self.timeout_secs > MAX_TIMEOUT_SECS {
            return Err(AppError::Config(format!(
                "ACP Server timeout_secs 必须在 1..={MAX_TIMEOUT_SECS} 范围内"
            )));
        }
        Ok(())
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

fn validate_command(command: &str) -> Result<(), AppError> {
    let command = command.trim();
    if command.is_empty() || command.contains(std::path::MAIN_SEPARATOR) {
        return Err(AppError::Config(
            "ACP Server command 必须是允许的命令名".into(),
        ));
    }
    if !command
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
    {
        return Err(AppError::Config("ACP Server command 包含非法字符".into()));
    }
    if !ALLOWED_COMMANDS.contains(&command) {
        return Err(AppError::Config(format!(
            "ACP Server command 不在允许列表中: {command}"
        )));
    }
    Ok(())
}

fn validate_arg(arg: &str) -> Result<(), AppError> {
    if arg.is_empty() || arg.len() > MAX_ARG_LEN {
        return Err(AppError::Config(format!(
            "ACP Server 参数长度必须在 1..={MAX_ARG_LEN} 范围内"
        )));
    }
    if arg.chars().any(|ch| ch.is_control()) {
        return Err(AppError::Config("ACP Server 参数不能包含控制字符".into()));
    }
    Ok(())
}

fn validate_env_var(var: &EnvVar) -> Result<(), AppError> {
    if !is_valid_env_name(&var.name) {
        return Err(AppError::Config(format!(
            "ACP Server 环境变量名称非法: {}",
            var.name
        )));
    }
    if BLOCKED_ENV_NAMES.contains(&var.name.as_str()) || var.name.starts_with("DYLD_") {
        return Err(AppError::Config(format!(
            "ACP Server 环境变量不允许覆盖: {}",
            var.name
        )));
    }
    if !ALLOWED_ENV_PREFIXES
        .iter()
        .any(|prefix| var.name.starts_with(prefix))
    {
        return Err(AppError::Config(format!(
            "ACP Server 环境变量不在允许前缀中: {}",
            var.name
        )));
    }
    if var.value.len() > MAX_ENV_VALUE_LEN
        || var.value.chars().any(|ch| matches!(ch, '\0' | '\n' | '\r'))
    {
        return Err(AppError::Config(format!(
            "ACP Server 环境变量值非法: {}",
            var.name
        )));
    }
    Ok(())
}

fn is_valid_env_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_uppercase() || first == '_')
        && chars.all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_accepts_default_local_server() {
        assert!(AcpServer::default_local().validate().is_ok());
    }

    #[test]
    fn validate_rejects_shell_command_payload() {
        let mut server = AcpServer::default_local();
        server.command = "agent; rm -rf /".to_string();

        assert!(server.validate().is_err());
    }

    #[test]
    fn validate_rejects_command_path() {
        let mut server = AcpServer::default_local();
        server.command = "/bin/sh".to_string();

        assert!(server.validate().is_err());
    }

    #[test]
    fn validate_rejects_dangerous_env_vars() {
        let mut server = AcpServer::default_local();
        server.env_vars.push(EnvVar {
            name: "LD_PRELOAD".to_string(),
            value: "evil.dylib".to_string(),
        });

        assert!(server.validate().is_err());
    }

    #[test]
    fn validate_accepts_allowlisted_env_prefixes() {
        let mut server = AcpServer::default_local();
        server.env_vars.push(EnvVar {
            name: "ANTHROPIC_API_KEY".to_string(),
            value: "test-key".to_string(),
        });

        assert!(server.validate().is_ok());
    }
}
