use super::GatewayError;
use crate::config::AppConfig;
use crate::error::AppError;
use crate::services::acp_client::{AcpClient, AcpServer, AcpServerRegistry};
use crate::services::agent::{
    Agent, AgentResolver, AgentStore, ConversationExecutionState, ConversationKey, Skill,
    SlashCommand,
};
use crate::services::channels::ChannelRegistry;
use crate::services::core::{AgentResponse, ChannelMessage, ResponseStatus};
use crate::services::event_bus::{EventBus, RuntimeEvent};
use crate::services::im_channel::feishu::{FeishuClient, TokenManager};
use crate::services::im_channel::telegram::{TelegramClient, TelegramTokenManager};
use crate::services::session_dispatcher::SessionDispatcher;
use crate::storage::{open_database, MessageStore, SharedDatabase};
use std::path::Path;
use std::sync::{Arc, Mutex};

pub struct GatewaySupervisor {
    config: Mutex<AppConfig>,
    channels: Arc<ChannelRegistry>,
    acp_servers: Arc<AcpServerRegistry>,
    agents: Arc<AgentStore>,
    acp_client: Mutex<Option<Arc<AcpClient>>>,
    resolver: Arc<AgentResolver>,
    dispatcher: Mutex<Option<Arc<SessionDispatcher>>>,
    messages: Arc<MessageStore>,
    event_bus: Arc<EventBus>,
}

impl GatewaySupervisor {
    pub fn new(config: AppConfig) -> Self {
        let channels = init_channels();
        let agents = Arc::new(
            init_agent_store(&config, None)
                .unwrap_or_else(|_| AgentStore::new(Agent::default_local())),
        );
        let messages = Arc::new(MessageStore::new());
        let event_bus = Arc::new(EventBus::new());

        event_bus.publish(RuntimeEvent::RuntimeStarted);
        Self::from_parts(config, channels, agents, messages, event_bus)
    }

    pub fn new_persistent(
        config: AppConfig,
        database_path: impl AsRef<Path>,
    ) -> Result<Self, AppError> {
        let path = database_path.as_ref();
        let database = open_database(path)?;
        let channels = init_channels();
        let agents = Arc::new(init_agent_store(&config, Some(database.clone()))?);
        let messages = Arc::new(MessageStore::new_with_database(database)?);
        let event_bus = Arc::new(EventBus::new());

        event_bus.publish(RuntimeEvent::RuntimeStarted);
        Ok(Self::from_parts(
            config, channels, agents, messages, event_bus,
        ))
    }

    fn from_parts(
        config: AppConfig,
        channels: Arc<ChannelRegistry>,
        agents: Arc<AgentStore>,
        messages: Arc<MessageStore>,
        event_bus: Arc<EventBus>,
    ) -> Self {
        let acp_servers = Arc::new(AcpServerRegistry::new(configured_acp_servers(&config)));
        let resolver = Arc::new(AgentResolver::new(agents.clone(), acp_servers.clone()));

        Self {
            config: Mutex::new(config),
            channels,
            acp_servers,
            agents,
            acp_client: Mutex::new(None),
            resolver,
            dispatcher: Mutex::new(None),
            messages,
            event_bus,
        }
    }

    pub async fn start_default(&self) -> Result<(), AppError> {
        let server = self
            .acp_servers
            .list()
            .into_iter()
            .find(|server| server.enabled)
            .ok_or_else(|| AppError::Agent("没有可用的 ACP Server".to_string()))?;

        let client = AcpClient::connect(&server)
            .await
            .map_err(|e| AppError::AcpClient(format!("连接 ACP Server 失败: {e}")))?;
        let client = Arc::new(client);
        let dispatcher = Arc::new(SessionDispatcher::new(
            self.resolver.clone(),
            client.clone(),
            self.event_bus.clone(),
        ));

        *self.acp_client.lock().unwrap() = Some(client);
        *self.dispatcher.lock().unwrap() = Some(dispatcher);
        Ok(())
    }

    fn dispatcher(&self) -> Arc<SessionDispatcher> {
        self.dispatcher
            .lock()
            .unwrap()
            .clone()
            .expect("GatewaySupervisor 尚未启动，请先调用 start_default()")
    }

    pub fn get_config(&self) -> AppConfig {
        let mut config = self.config.lock().unwrap().clone();
        config.acp_servers = self.acp_servers.list();
        config.agents = self.agents.list_agents();
        config.skills = self.agents.list_skills();
        config.slash_commands = self.agents.list_commands();
        config.default_agent_id = self.agents.default_agent_id();
        config
    }

    pub fn update_feishu_poll_interval(&self, interval_secs: u64) {
        if let Some(config) = self.config.lock().unwrap().channels.get_mut("feishu") {
            config.poll_interval_secs = interval_secs;
        }
    }

    pub async fn set_channel_credentials(
        &self,
        channel: &str,
        credentials: serde_json::Value,
    ) -> Result<(), AppError> {
        self.channels
            .set_credentials(channel, credentials)
            .await
            .map_err(to_gateway_error)
    }

    pub async fn test_channel_connection(&self, channel: &str) -> Result<bool, AppError> {
        self.channels
            .test_connection(channel)
            .await
            .map(|_| true)
            .map_err(to_gateway_error)
    }

    pub async fn channel_has_credentials(&self, channel: &str) -> bool {
        self.channels.has_credentials(channel).await
    }

    pub async fn list_channels(&self) -> Vec<String> {
        self.channels.list_channels().await
    }

    pub async fn poll_channel_messages(
        &self,
        channel: &str,
        page_token: Option<String>,
    ) -> Result<Vec<ChannelMessage>, AppError> {
        let gateway = self
            .channels
            .get(channel)
            .await
            .ok_or_else(|| AppError::Gateway(format!("未知渠道: {channel}")))?;
        let page_size = self
            .config
            .lock()
            .unwrap()
            .get_channel_config(channel)
            .page_size;
        let (messages, _) = gateway
            .poll_messages(page_size, page_token.as_deref())
            .map_err(to_gateway_error)?;

        let store = self.messages.clone();
        let new_messages = tokio::task::spawn_blocking(move || store.filter_new_messages(messages))
            .await
            .unwrap_or_default();
        for message in &new_messages {
            self.messages.save_message(message.clone());
        }
        Ok(new_messages)
    }

    pub fn get_messages(&self, limit: Option<usize>) -> Vec<ChannelMessage> {
        match limit {
            Some(limit) => self.messages.get_recent_messages(limit),
            None => self.messages.get_messages(),
        }
    }

    pub fn clear_messages(&self) {
        self.messages.clear();
    }

    async fn check_and_mark_seen(&self, message_id: String) -> bool {
        let messages = self.messages.clone();
        tokio::task::spawn_blocking(move || messages.check_and_mark_seen(&message_id))
            .await
            .unwrap_or(false)
    }

    pub async fn dispatch_message(
        &self,
        message: ChannelMessage,
    ) -> Result<AgentResponse, AppError> {
        if !self.check_and_mark_seen(message.message_id.clone()).await {
            return Ok(AgentResponse {
                request_id: message.message_id,
                status: ResponseStatus::Success,
                output: "消息已处理，跳过重复分发".to_string(),
                error_message: None,
            });
        }

        self.messages.save_message(message.clone());
        let response = self.dispatcher().dispatch(message.clone()).await?;
        self.send_agent_reply(&message, &response).await?;
        Ok(response)
    }

    async fn send_agent_reply(
        &self,
        message: &ChannelMessage,
        response: &AgentResponse,
    ) -> Result<(), AppError> {
        let auto_reply = self
            .config
            .lock()
            .unwrap()
            .get_channel_config(&message.channel)
            .auto_reply;
        if !auto_reply || response.status != ResponseStatus::Success || response.output.is_empty() {
            return Ok(());
        }

        match self
            .channels
            .send_message(
                &message.channel,
                &message.conversation_id,
                &response.output,
                Some(&message.message_id),
            )
            .await
        {
            Ok(()) => {
                self.event_bus.publish(RuntimeEvent::ReplySent {
                    message_id: message.message_id.clone(),
                    channel: message.channel.clone(),
                    conversation_id: message.conversation_id.clone(),
                });
                Ok(())
            }
            Err(error) => {
                self.event_bus.publish(RuntimeEvent::ReplyFailed {
                    message_id: message.message_id.clone(),
                    error: error.to_string(),
                });
                Err(to_gateway_error(error))
            }
        }
    }

    pub fn list_acp_servers(&self) -> Vec<AcpServer> {
        self.acp_servers.list()
    }

    pub fn get_acp_server_status(
        &self,
        server_id: String,
    ) -> crate::services::acp_client::AcpServerStatus {
        self.acp_servers.status(&server_id)
    }

    pub fn save_acp_server(&self, server: AcpServer) {
        self.acp_servers.save(server);
    }

    pub fn list_agents(&self) -> Vec<Agent> {
        self.agents.list_agents()
    }

    pub fn save_agent(&self, agent: Agent) {
        self.agents.save_agent(agent);
    }

    pub fn set_default_agent(&self, agent_id: String) {
        self.agents.set_default_agent(agent_id);
    }

    pub fn list_skills(&self) -> Vec<Skill> {
        self.agents.list_skills()
    }

    pub fn save_skill(&self, skill: Skill) {
        self.agents.save_skill(skill);
    }

    pub fn bind_skill(&self, agent_id: String, skill_id: String) {
        self.agents.bind_skill(agent_id, skill_id);
    }

    pub fn list_slash_commands(&self) -> Vec<SlashCommand> {
        self.agents.list_commands()
    }

    pub fn save_slash_command(&self, command: SlashCommand) {
        self.agents.save_command(command);
    }

    pub fn get_conversation_execution_state(
        &self,
        channel: String,
        conversation_id: String,
    ) -> Option<ConversationExecutionState> {
        self.agents.get_conversation_state(&ConversationKey {
            channel,
            conversation_id,
        })
    }
}

fn init_channels() -> Arc<ChannelRegistry> {
    let channels = Arc::new(ChannelRegistry::new());
    channels.register(Arc::new(FeishuClient::new(Arc::new(TokenManager::new()))));
    channels.register(Arc::new(TelegramClient::new(Arc::new(
        TelegramTokenManager::new(),
    ))));
    channels
}

fn configured_acp_servers(config: &AppConfig) -> Vec<AcpServer> {
    if config.acp_servers.is_empty() {
        vec![AcpServer::default_local()]
    } else {
        config.acp_servers.clone()
    }
}

fn init_agent_store(
    config: &AppConfig,
    database: Option<SharedDatabase>,
) -> Result<AgentStore, AppError> {
    let default_agent = config
        .agents
        .iter()
        .find(|agent| agent.id == config.default_agent_id)
        .cloned()
        .or_else(|| config.agents.first().cloned())
        .unwrap_or_else(Agent::default_local);

    let store = match database {
        Some(database) => AgentStore::new_with_database(default_agent, database)?,
        None => AgentStore::new(default_agent),
    };

    for agent in &config.agents {
        store.save_agent(agent.clone());
    }
    for skill in &config.skills {
        store.save_skill(skill.clone());
    }
    for command in &config.slash_commands {
        store.save_command(command.clone());
    }
    store.set_default_agent(config.default_agent_id.clone());

    Ok(store)
}

fn to_gateway_error(error: GatewayError) -> AppError {
    AppError::Gateway(error.to_string())
}
