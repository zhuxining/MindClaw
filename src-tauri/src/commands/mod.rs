use crate::config::AppConfig;
use crate::error::AppError;
use crate::services::acp_client::AcpClient;
use crate::services::gateway::GatewayRegistry;
use crate::services::im_channel::feishu::{FeishuClient, TokenManager};
use crate::services::im_channel::telegram::{TelegramClient, TelegramTokenManager};
use crate::services::message_bus::{
    AgentRequest, AgentResponse, ChannelMessage, MessageBus, RouteRule,
};
use crate::storage::MessageStore;
use std::sync::{Arc, Mutex};
use tauri::State;

/// 应用全局状态
pub struct AppState {
    pub config: Mutex<AppConfig>,
    pub gateway_registry: Arc<GatewayRegistry>,
    pub message_bus: Arc<MessageBus>,
    pub acp_client: Arc<AcpClient>,
    pub message_store: Arc<MessageStore>,
}

impl AppState {
    pub fn new() -> Self {
        let config = AppConfig::load().unwrap_or_default();
        let message_bus = Arc::new(MessageBus::new());
        let acp_client = Arc::new(AcpClient::new(
            config.acp_agent.agent_path.clone(),
            config.acp_agent.timeout_secs,
        ));
        let message_store = Arc::new(MessageStore::new());

        // 初始化渠道注册中心，注册飞书 Gateway
        let gateway_registry = Arc::new(GatewayRegistry::new());
        let token_manager = Arc::new(TokenManager::new());
        let feishu_client = Arc::new(FeishuClient::new(token_manager));
        gateway_registry.register(feishu_client);

        // 注册 Telegram Gateway
        let tg_token_manager = Arc::new(TelegramTokenManager::new());
        let telegram_client = Arc::new(TelegramClient::new(tg_token_manager));
        gateway_registry.register(telegram_client);

        Self {
            config: Mutex::new(config),
            gateway_registry,
            message_bus,
            acp_client,
            message_store,
        }
    }
}

// ── 渠道凭证管理（泛化）────────────────────────────────────────

/// 设置指定渠道的凭证
///
/// `credentials` 为渠道特有格式的 JSON：
/// - 飞书: `{"app_id": "...", "app_secret": "..."}`
#[tauri::command]
pub async fn set_channel_credentials(
    channel: String,
    credentials: serde_json::Value,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    state
        .gateway_registry
        .set_credentials(&channel, credentials)
        .await
        .map_err(|e| AppError::Gateway(e.to_string()))
}

/// 测试渠道连接
#[tauri::command]
pub async fn test_channel_connection(
    channel: String,
    state: State<'_, AppState>,
) -> Result<bool, AppError> {
    match state.gateway_registry.test_connection(&channel).await {
        Ok(()) => Ok(true),
        Err(e) => Err(AppError::Gateway(e.to_string())),
    }
}

/// 获取渠道连接状态
#[tauri::command]
pub async fn get_channel_connection_status(
    channel: String,
    state: State<'_, AppState>,
) -> Result<bool, AppError> {
    Ok(state.gateway_registry.has_credentials(&channel).await)
}

/// 列出所有已注册的渠道
#[tauri::command]
pub async fn list_channels(state: State<'_, AppState>) -> Result<Vec<String>, AppError> {
    Ok(state.gateway_registry.list_channels().await)
}

// ── 兼容旧版飞书命令（deprecated，映射到泛化接口）───────────────

#[tauri::command]
pub async fn set_feishu_credentials(
    app_id: String,
    app_secret: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let credentials = serde_json::json!({
        "app_id": app_id,
        "app_secret": app_secret,
    });
    state
        .gateway_registry
        .set_credentials("feishu", credentials)
        .await
        .map_err(|e| AppError::Gateway(e.to_string()))
}

#[tauri::command]
pub async fn test_feishu_connection(state: State<'_, AppState>) -> Result<bool, AppError> {
    test_channel_connection("feishu".to_string(), state).await
}

#[tauri::command]
pub async fn get_feishu_connection_status(state: State<'_, AppState>) -> Result<bool, AppError> {
    get_channel_connection_status("feishu".to_string(), state).await
}

// ── 消息轮询（泛化）───────────────────────────────────────────

/// 轮询指定渠道的消息
///
/// `channel`: 渠道名称，如 "feishu"。目前仅支持飞书。
#[tauri::command]
pub async fn poll_channel_messages(
    channel: String,
    page_token: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<ChannelMessage>, AppError> {
    let gw = state
        .gateway_registry
        .get(&channel)
        .await
        .ok_or_else(|| AppError::Gateway(format!("未知渠道: {}", channel)))?;

    let page_size = state.config.lock().unwrap().feishu.page_size;

    let (messages, _next_token) = gw
        .poll_messages(page_size, page_token.as_deref())
        .map_err(|e| AppError::Gateway(e.to_string()))?;

    // 去重并保存新消息
    let new_messages: Vec<ChannelMessage> = messages
        .into_iter()
        .filter(|msg| state.message_store.check_and_mark_seen(&msg.message_id))
        .collect();

    for msg in &new_messages {
        state.message_store.save_message(msg.clone());
    }

    Ok(new_messages)
}

/// 兼容旧版飞书拉取（deprecated）
#[tauri::command]
pub async fn poll_feishu_messages(
    container_id_type: String,
    page_token: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<ChannelMessage>, AppError> {
    // 忽略 container_id_type，使用新接口
    let _ = container_id_type;
    poll_channel_messages("feishu".to_string(), page_token, state).await
}

// ── 消息管理 ──────────────────────────────────────────────────

#[tauri::command]
pub async fn get_messages(
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<ChannelMessage>, AppError> {
    let messages = if let Some(limit) = limit {
        state.message_store.get_recent_messages(limit)
    } else {
        state.message_store.get_messages()
    };
    Ok(messages)
}

#[tauri::command]
pub async fn clear_messages(state: State<'_, AppState>) -> Result<(), AppError> {
    state.message_store.clear();
    Ok(())
}

// ── 消息处理 ──────────────────────────────────────────────────

#[tauri::command]
pub async fn process_message(
    message: ChannelMessage,
    state: State<'_, AppState>,
) -> Result<AgentResponse, AppError> {
    let bus = state.message_bus.clone();
    let acp = state.acp_client.clone();
    let registry = state.gateway_registry.clone();
    let auto_reply = state.config.lock().unwrap().feishu.auto_reply;

    // Agent 回调：调用 ACP Client
    let agent_callback: crate::services::message_bus::router::AgentCallback = Arc::new(
        move |req: AgentRequest| -> Result<AgentResponse, AppError> {
            let acp = acp.clone();
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async { acp.send(req).await })
            })
        },
    );

    // 回复回调：通过 GatewayRegistry 回写到消息来源渠道
    let reply_callback: crate::services::message_bus::router::ReplyCallback =
        Arc::new(move |msg: ChannelMessage| -> Result<(), AppError> {
            if !auto_reply {
                return Ok(());
            }
            let registry = registry.clone();
            let channel = msg.channel.clone();
            let conversation_id = msg.conversation_id.clone();
            let content = msg.content.clone();
            let reply_to = msg.reply_to.clone();
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    registry
                        .send_message(&channel, &conversation_id, &content, reply_to.as_deref())
                        .await
                        .map_err(|e| AppError::Gateway(e.to_string()))
                })
            })
        });

    bus.process_message(message, &agent_callback, &reply_callback)
        .await
}

// ── 路由规则管理 ──────────────────────────────────────────────

#[tauri::command]
pub async fn get_route_rules(state: State<'_, AppState>) -> Result<Vec<RouteRule>, AppError> {
    Ok(state.message_bus.get_rules().await)
}

#[tauri::command]
pub async fn add_route_rule(rule: RouteRule, state: State<'_, AppState>) -> Result<(), AppError> {
    state.message_bus.register_rule(rule).await;
    Ok(())
}

#[tauri::command]
pub async fn remove_route_rule(
    rule_id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    state.message_bus.remove_rule(&rule_id).await;
    Ok(())
}

// ── 配置管理 ──────────────────────────────────────────────────

#[tauri::command]
pub async fn get_config(state: State<'_, AppState>) -> Result<AppConfig, AppError> {
    Ok(state.config.lock().unwrap().clone())
}

#[tauri::command]
pub async fn update_feishu_poll_interval(
    interval_secs: u64,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let mut config = state.config.lock().unwrap();
    config.feishu.poll_interval_secs = interval_secs;
    Ok(())
}
