use super::command_store::CommandStore;
use super::skill_store::SkillStore;
use super::state_store::ConversationStateStore;
use super::types::{Agent, ConversationExecutionState, ConversationKey, Skill, SlashCommand};
use crate::error::AppError;
use crate::storage::{open_database, SharedDatabase};
use std::collections::HashMap;
use std::path::Path;
use std::sync::RwLock;

pub struct AgentStore {
    agents: AgentDataStore,
    skills: SkillStore,
    commands: CommandStore,
    conversation_states: ConversationStateStore,
    default_agent_id: RwLock<String>,
}

impl AgentStore {
    pub fn new(default_agent: Agent) -> Self {
        let default_agent_id = default_agent.id.clone();
        let agents = AgentDataStore::new(default_agent);

        Self::from_parts(agents, ConversationStateStore::new(), default_agent_id)
    }

    #[allow(dead_code)]
    pub fn new_persistent(
        default_agent: Agent,
        database_path: impl AsRef<Path>,
    ) -> Result<Self, AppError> {
        Self::new_with_database(default_agent, open_database(database_path)?)
    }

    pub fn new_with_database(
        default_agent: Agent,
        database: SharedDatabase,
    ) -> Result<Self, AppError> {
        let default_agent_id = default_agent.id.clone();
        let agents = AgentDataStore::new(default_agent);
        let conversation_states = ConversationStateStore::new_with_database(database)?;

        Ok(Self::from_parts(
            agents,
            conversation_states,
            default_agent_id,
        ))
    }

    fn from_parts(
        agents: AgentDataStore,
        conversation_states: ConversationStateStore,
        default_agent_id: String,
    ) -> Self {
        Self {
            agents,
            skills: SkillStore::new(),
            commands: CommandStore::new(),
            conversation_states,
            default_agent_id: RwLock::new(default_agent_id),
        }
    }

    pub fn list_agents(&self) -> Vec<Agent> {
        self.agents.list()
    }

    pub fn get_agent(&self, id: &str) -> Option<Agent> {
        self.agents.get(id)
    }

    pub fn save_agent(&self, agent: Agent) {
        self.agents.save(agent);
    }

    pub fn default_agent(&self) -> Option<Agent> {
        let id = self.default_agent_id.read().unwrap().clone();
        self.get_agent(&id)
    }

    pub fn set_default_agent(&self, agent_id: String) {
        *self.default_agent_id.write().unwrap() = agent_id;
    }

    pub fn default_agent_id(&self) -> String {
        self.default_agent_id.read().unwrap().clone()
    }

    pub fn list_skills(&self) -> Vec<Skill> {
        self.skills.list_skills()
    }

    pub fn get_skill(&self, id: &str) -> Option<Skill> {
        self.skills.get_skill(id)
    }

    pub fn save_skill(&self, skill: Skill) {
        self.skills.save_skill(skill);
    }

    pub fn bind_skill(&self, agent_id: String, skill_id: String) {
        self.skills.bind(agent_id, skill_id);
    }

    pub fn agent_has_skill(&self, agent_id: &str, skill_id: &str) -> bool {
        self.skills.agent_has_skill(agent_id, skill_id)
    }

    pub fn list_commands(&self) -> Vec<SlashCommand> {
        self.commands.list()
    }

    #[allow(dead_code)]
    pub fn get_command(&self, command: &str) -> Option<SlashCommand> {
        self.commands.get(command)
    }

    pub fn save_command(&self, command: SlashCommand) {
        self.commands.save(command);
    }

    pub fn get_conversation_state(
        &self,
        key: &ConversationKey,
    ) -> Option<ConversationExecutionState> {
        self.conversation_states.get(key)
    }

    pub fn save_conversation_state(&self, state: ConversationExecutionState) {
        self.conversation_states.save(state);
    }

    pub fn reset_conversation_state(&self, key: &ConversationKey) {
        self.conversation_states.reset(key);
    }
}

struct AgentDataStore {
    agents: RwLock<HashMap<String, Agent>>,
}

impl AgentDataStore {
    fn new(default_agent: Agent) -> Self {
        let mut agents = HashMap::new();
        agents.insert(default_agent.id.clone(), default_agent);
        Self {
            agents: RwLock::new(agents),
        }
    }

    fn list(&self) -> Vec<Agent> {
        self.agents.read().unwrap().values().cloned().collect()
    }

    fn get(&self, id: &str) -> Option<Agent> {
        self.agents.read().unwrap().get(id).cloned()
    }

    fn save(&self, agent: Agent) {
        self.agents.write().unwrap().insert(agent.id.clone(), agent);
    }
}
