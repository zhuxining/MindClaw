use super::GatewayError;
use crate::config::AppConfig;
use crate::error::AppError;
use crate::services::acp_client::{AcpClient, AcpServer, AcpServerRegistry};
use crate::services::agent::{
    Agent, AgentResolver, AgentStore, ConversationExecutionState, ConversationKey, Skill,
    SlashCommand,
};
use crate::services::channels::feishu::FeishuFactory;
use crate::services::channels::telegram::TelegramFactory;
use crate::services::channels::{ChannelDeps, ChannelManager, ChannelRegistry, MessageBus};
use crate::services::core::{ChannelMessage, SecretStore};
use crate::services::event_bus::{EventBus, RuntimeEvent};
use crate::services::session_dispatcher::SessionDispatcher;
use crate::storage::{open_database, MessageStore, SharedDatabase};
use std::path::Path;
use std::sync::{Arc, Mutex};

pub struct GatewaySupervisor {
    config: Mutex<AppConfig>,
    channels: Arc<ChannelRegistry>,
    channel_manager: Arc<ChannelManager>,
    bus: Arc<MessageBus>,
    secrets: Arc<dyn SecretStore>,
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
        let agents = Arc::new(
            init_agent_store(&config, None)
                .unwrap_or_else(|_| AgentStore::new(Agent::default_local())),
        );
        let messages = Arc::new(MessageStore::new());
        let event_bus = Arc::new(EventBus::new());
        event_bus.publish(RuntimeEvent::RuntimeStarted);
        let bus = Arc::new(MessageBus::new());
        let channels = init_registry();
        let secrets: Arc<dyn SecretStore> = Arc::new(crate::secret_store::MemorySecretStore::new());
        Self::from_parts(config, channels, agents, messages, event_bus, bus, secrets)
    }

    pub fn new_persistent(
        config: AppConfig,
        database_path: impl AsRef<Path>,
    ) -> Result<Self, AppError> {
        let path = database_path.as_ref();
        let database = open_database(path)?;
        let agents = Arc::new(init_agent_store(&config, Some(database.clone()))?);
        let messages = Arc::new(MessageStore::new_with_database(database)?);
        let event_bus = Arc::new(EventBus::new());
        event_bus.publish(RuntimeEvent::RuntimeStarted);
        let bus = Arc::new(MessageBus::new());
        let channels = init_registry();
        let secrets: Arc<dyn SecretStore> = Arc::new(crate::secret_store::MemorySecretStore::new());
        Ok(Self::from_parts(
            config, channels, agents, messages, event_bus, bus, secrets,
        ))
    }

    fn from_parts(
        config: AppConfig,
        channels: Arc<ChannelRegistry>,
        agents: Arc<AgentStore>,
        messages: Arc<MessageStore>,
        event_bus: Arc<EventBus>,
        bus: Arc<MessageBus>,
        secrets: Arc<dyn SecretStore>,
    ) -> Self {
        let acp_servers = Arc::new(AcpServerRegistry::new(configured_acp_servers(&config)));
        let resolver = Arc::new(AgentResolver::new(agents.clone(), acp_servers.clone()));
        let channel_manager = Arc::new(ChannelManager::new(bus.clone(), event_bus.clone()));

        // 按配置 enabled 列表构造渠道实例
        let enabled: Vec<String> = config
            .channels
            .iter()
            .filter(|(_, c)| c.enabled)
            .map(|(k, _)| k.clone())
            .collect();
        let deps = ChannelDeps::new(reqwest::Client::new(), secrets.clone(), event_bus.clone());
        channels.build_all(&deps, &enabled);

        Self {
            config: Mutex::new(config),
            channels,
            channel_manager,
            bus,
            secrets,
            acp_servers,
            agents,
            acp_client: Mutex::new(None),
            resolver,
            dispatcher: Mutex::new(None),
            messages,
            event_bus,
        }
    }

    /// 启动 ACP 连接 + 渠道运行时。
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
        *self.dispatcher.lock().unwrap() = Some(dispatcher.clone());

        // 启动 ChannelRuntime（消费 inbound，分发到 dispatcher，产出 outbound）
        let runtime = Arc::new(crate::services::channels::runtime::ChannelRuntime::new(
            self.bus.clone(),
            dispatcher,
            self.messages.clone(),
            self.channel_manager.clone(),
            self.event_bus.clone(),
            Arc::new(Mutex::new(self.get_config())),
        ));
        runtime.spawn(tokio_util::sync::CancellationToken::new());

        // 启动渠道管理器
        self.channel_manager
            .start_all(self.channels.instances())
            .await;
        Ok(())
    }

    #[allow(dead_code)]
    fn dispatcher(&self) -> Arc<SessionDispatcher> {
        self.dispatcher
            .lock()
            .unwrap()
            .clone()
            .expect("GatewaySupervisor 尚未启动")
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

    #[allow(dead_code)]
    pub fn event_bus(&self) -> Arc<EventBus> {
        self.event_bus.clone()
    }

    // ── 渠道凭证 ───────────────────────────────────────────

    pub async fn set_channel_credentials(
        &self,
        channel: &str,
        credentials: serde_json::Value,
    ) -> Result<(), AppError> {
        let gw = self
            .channels
            .get(channel)
            .ok_or_else(|| AppError::Gateway(format!("未知渠道: {channel}")))?;
        gw.credentials()
            .set_credentials(credentials, self.secrets.as_ref())
            .await
            .map_err(to_gateway_error)
    }

    pub async fn test_channel_connection(&self, channel: &str) -> Result<bool, AppError> {
        let gw = self
            .channels
            .get(channel)
            .ok_or_else(|| AppError::Gateway(format!("未知渠道: {channel}")))?;
        gw.credentials()
            .test_connection()
            .await
            .map(|_| true)
            .map_err(to_gateway_error)
    }

    pub async fn channel_has_credentials(&self, channel: &str) -> bool {
        if let Some(gw) = self.channels.get(channel) {
            gw.credentials()
                .has_credentials(self.secrets.as_ref())
                .await
        } else {
            false
        }
    }

    // ── 描述符 ─────────────────────────────────────────────

    pub fn list_channel_descriptors(&self) -> Vec<&crate::services::channels::ChannelDescriptor> {
        self.channels.list_descriptors()
    }

    pub fn list_channels(&self) -> Vec<String> {
        self.channels.list_channels()
    }

    // ── 消息 ───────────────────────────────────────────────

    /// 历史快照读模型：按最迟 limit 条返回，不再标记 seen。
    pub fn get_messages(&self, limit: Option<usize>) -> Vec<ChannelMessage> {
        match limit {
            Some(limit) => self.messages.get_recent_messages(limit),
            None => self.messages.get_messages(),
        }
    }

    pub fn clear_messages(&self) {
        self.messages.clear();
    }

    /// 渠道运行时状态。
    pub async fn channels_status(&self) -> Vec<crate::services::channels::manager::ChannelStatus> {
        self.channel_manager.status().await
    }

    // ── ACP / Agent / Skill ────────────────────────────────

    pub fn list_acp_servers(&self) -> Vec<AcpServer> {
        self.acp_servers.list()
    }

    pub fn get_acp_server_status(
        &self,
        server_id: String,
    ) -> crate::services::acp_client::AcpServerStatus {
        self.acp_servers.status(&server_id)
    }

    pub fn save_acp_server(&self, server: AcpServer) -> Result<(), AppError> {
        server.validate()?;
        self.acp_servers.save(server);
        Ok(())
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

fn init_registry() -> Arc<ChannelRegistry> {
    let registry = Arc::new(ChannelRegistry::new());
    registry.register_factory(Arc::new(FeishuFactory));
    registry.register_factory(Arc::new(TelegramFactory));
    registry
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
