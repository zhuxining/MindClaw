use crate::config::AppConfig;
use crate::error::AppError;
use crate::services::acp_client::{AcpServer, AcpServerStatus};
use crate::services::agent::{Agent, ConversationExecutionState, Skill, SlashCommand};
use crate::services::core::{AgentResponse, ChannelMessage};
use crate::services::gateway::GatewaySupervisor;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;

/// 应用全局状态。
pub struct AppState {
    pub gateway: Arc<GatewaySupervisor>,
}

fn app_database_path() -> Option<PathBuf> {
    let project_dirs = directories::ProjectDirs::from("com", "mindclaw", "MindClaw")?;
    let data_dir = project_dirs.data_local_dir();
    std::fs::create_dir_all(data_dir).ok()?;
    Some(data_dir.join("mindclaw.sqlite3"))
}

impl AppState {
    pub fn new() -> Self {
        let config = AppConfig::load().unwrap_or_default();
        let gateway = match app_database_path()
            .and_then(|path| GatewaySupervisor::new_persistent(config.clone(), path).ok())
        {
            Some(gateway) => gateway,
            None => {
                eprintln!("ConversationExecutionState 持久化初始化失败，回退到内存存储");
                GatewaySupervisor::new(config)
            }
        };

        Self {
            gateway: Arc::new(gateway),
        }
    }

    pub async fn start(self: &Arc<Self>) -> Result<(), AppError> {
        self.gateway.start_default().await
    }
}

// ── 渠道凭证管理 ────────────────────────────────────────────────

#[tauri::command]
pub async fn set_channel_credentials(
    channel: String,
    credentials: serde_json::Value,
    state: State<'_, Arc<AppState>>,
) -> Result<(), AppError> {
    state
        .gateway
        .set_channel_credentials(&channel, credentials)
        .await
}

#[tauri::command]
pub async fn test_channel_connection(
    channel: String,
    state: State<'_, Arc<AppState>>,
) -> Result<bool, AppError> {
    state.gateway.test_channel_connection(&channel).await
}

#[tauri::command]
pub async fn get_channel_connection_status(
    channel: String,
    state: State<'_, Arc<AppState>>,
) -> Result<bool, AppError> {
    Ok(state.gateway.channel_has_credentials(&channel).await)
}

#[tauri::command]
pub async fn list_channels(state: State<'_, Arc<AppState>>) -> Result<Vec<String>, AppError> {
    Ok(state.gateway.list_channels().await)
}

// ── 消息 ────────────────────────────────────────────────────────

#[tauri::command]
pub async fn poll_channel_messages(
    channel: String,
    page_token: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<ChannelMessage>, AppError> {
    state
        .gateway
        .poll_channel_messages(&channel, page_token)
        .await
}

#[tauri::command]
pub fn get_messages(
    limit: Option<usize>,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<ChannelMessage>, AppError> {
    Ok(state.gateway.get_messages(limit))
}

#[tauri::command]
pub fn clear_messages(state: State<'_, Arc<AppState>>) -> Result<(), AppError> {
    state.gateway.clear_messages();
    Ok(())
}

#[tauri::command]
pub async fn process_message(
    message: ChannelMessage,
    state: State<'_, Arc<AppState>>,
) -> Result<AgentResponse, AppError> {
    state.gateway.dispatch_message(message).await
}

// ── ACP Server ─────────────────────────────────────────────────

#[tauri::command]
pub fn list_acp_servers(state: State<'_, Arc<AppState>>) -> Result<Vec<AcpServer>, AppError> {
    Ok(state.gateway.list_acp_servers())
}

#[tauri::command]
pub fn save_acp_server(server: AcpServer, state: State<'_, Arc<AppState>>) -> Result<(), AppError> {
    server.validate()?;
    state.gateway.save_acp_server(server)
}

#[tauri::command]
pub fn get_acp_server_status(
    server_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<AcpServerStatus, AppError> {
    Ok(state.gateway.get_acp_server_status(server_id))
}

// ── ACP Registry ────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RegistryAgent {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub repository: Option<String>,
    pub website: Option<String>,
    pub authors: Vec<String>,
    pub license: String,
    pub icon: Option<String>,
    pub distribution: serde_json::Value,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AcpRegistry {
    pub version: String,
    pub agents: Vec<RegistryAgent>,
}

#[tauri::command]
pub async fn fetch_acp_registry() -> Result<AcpRegistry, AppError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| AppError::Config(format!("创建 HTTP 客户端失败: {}", e)))?;

    let response = client
        .get("https://cdn.agentclientprotocol.com/registry/v1/latest/registry.json")
        .send()
        .await
        .map_err(|e| AppError::Config(format!("获取 ACP 注册表失败: {}", e)))?;

    let registry = response
        .json::<AcpRegistry>()
        .await
        .map_err(|e| AppError::Config(format!("解析 ACP 注册表失败: {}", e)))?;

    Ok(registry)
}

#[tauri::command]
pub async fn install_acp_agent(
    registry_agent: RegistryAgent,
    state: State<'_, Arc<AppState>>,
) -> Result<(), AppError> {
    use crate::services::acp_client::server::{validate_npm_package, EnvVar};

    // 将 RegistryAgent 转换为 AcpServer
    let distribution = registry_agent.distribution;

    // 优先使用 npx 分发方式
    let (command, args, env_vars) = if let Some(npx) = distribution.get("npx") {
        let package = npx
            .get("package")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::Config("ACP Agent npx package 不存在".into()))?;

        // 严格验证 npm 包名，防止包名注入
        validate_npm_package(package)?;

        let args_from_registry = npx
            .get("args")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|v| v.as_str().map(ToString::to_string))
            .collect::<Vec<_>>();

        // 仅允许特定的 ACP 相关环境变量
        let env_from_registry = npx
            .get("env")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|(k, v)| {
                // 仅允许以 ACP_ 开头的环境变量
                if k.starts_with("ACP_") {
                    v.as_str().map(|s| EnvVar {
                        name: k,
                        value: s.to_string(),
                    })
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        let mut final_args = vec!["-y".to_string(), package.to_string()];
        final_args.extend(args_from_registry);

        ("npx".to_string(), final_args, env_from_registry)
    } else if distribution.get("binary").is_some() {
        // 对于 binary 分发方式，提示用户需要手动安装
        return Err(AppError::Config(
            "Binary 分发方式需要手动安装，请参考项目文档。".into(),
        ));
    } else {
        return Err(AppError::Config(
            "不支持的分发方式，仅支持 npx 分发。".into(),
        ));
    };

    let server = AcpServer {
        id: registry_agent.id,
        name: registry_agent.name,
        description: registry_agent.description,
        command,
        args,
        env_vars,
        timeout_secs: 120,
        enabled: true,
    };

    server.validate()?;
    state.gateway.save_acp_server(server)
}

// ── Agent / Skill / SlashCommand ───────────────────────────────

#[tauri::command]
pub fn list_agents(state: State<'_, Arc<AppState>>) -> Result<Vec<Agent>, AppError> {
    Ok(state.gateway.list_agents())
}

#[tauri::command]
pub fn save_agent(agent: Agent, state: State<'_, Arc<AppState>>) -> Result<(), AppError> {
    state.gateway.save_agent(agent);
    Ok(())
}

#[tauri::command]
pub fn set_default_agent(
    agent_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), AppError> {
    state.gateway.set_default_agent(agent_id);
    Ok(())
}

#[tauri::command]
pub fn list_skills(state: State<'_, Arc<AppState>>) -> Result<Vec<Skill>, AppError> {
    Ok(state.gateway.list_skills())
}

#[tauri::command]
pub fn save_skill(skill: Skill, state: State<'_, Arc<AppState>>) -> Result<(), AppError> {
    state.gateway.save_skill(skill);
    Ok(())
}

#[tauri::command]
pub fn bind_skill(
    agent_id: String,
    skill_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), AppError> {
    state.gateway.bind_skill(agent_id, skill_id);
    Ok(())
}

#[tauri::command]
pub fn list_slash_commands(state: State<'_, Arc<AppState>>) -> Result<Vec<SlashCommand>, AppError> {
    Ok(state.gateway.list_slash_commands())
}

#[tauri::command]
pub fn save_slash_command(
    command: SlashCommand,
    state: State<'_, Arc<AppState>>,
) -> Result<(), AppError> {
    state.gateway.save_slash_command(command);
    Ok(())
}

#[tauri::command]
pub fn get_conversation_execution_state(
    channel: String,
    conversation_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Option<ConversationExecutionState>, AppError> {
    Ok(state
        .gateway
        .get_conversation_execution_state(channel, conversation_id))
}

// ── 配置 ────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_config(state: State<'_, Arc<AppState>>) -> Result<AppConfig, AppError> {
    Ok(state.gateway.get_config())
}

#[tauri::command]
pub fn update_feishu_poll_interval(
    interval_secs: u64,
    state: State<'_, Arc<AppState>>,
) -> Result<(), AppError> {
    state.gateway.update_feishu_poll_interval(interval_secs);
    Ok(())
}
