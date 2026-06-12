use super::store::AgentStore;
use super::types::{ConversationExecutionState, ConversationKey, ExecutionContext};
use crate::error::AppError;
use crate::services::acp_client::AcpServerRegistry;
use std::sync::Arc;

pub struct AgentResolver {
    store: Arc<AgentStore>,
    acp_servers: Arc<AcpServerRegistry>,
}

impl AgentResolver {
    pub fn new(store: Arc<AgentStore>, acp_servers: Arc<AcpServerRegistry>) -> Self {
        Self { store, acp_servers }
    }

    pub fn default_context(&self) -> Result<ExecutionContext, AppError> {
        let agent = self
            .store
            .default_agent()
            .ok_or_else(|| AppError::Agent("未配置默认 Agent".to_string()))?;
        self.context_for_agent(&agent.id, agent.default_skill_id.as_deref())
    }

    pub fn context_for_conversation(
        &self,
        key: &ConversationKey,
    ) -> Result<ExecutionContext, AppError> {
        if let Some(state) = self.store.get_conversation_state(key) {
            self.context_for_agent(&state.agent_id, state.skill_id.as_deref())
        } else {
            self.default_context()
        }
    }

    #[allow(dead_code)]
    pub fn context_for_command(&self, command: &str) -> Result<ExecutionContext, AppError> {
        if let Some(slash_command) = self.store.get_command(command) {
            if !slash_command.enabled {
                return Err(AppError::Agent(format!("SlashCommand 已禁用: /{command}")));
            }
            return self
                .context_for_agent(&slash_command.agent_id, slash_command.skill_id.as_deref());
        }

        self.context_for_agent(command, None)
    }

    pub fn context_for_agent(
        &self,
        agent_id: &str,
        skill_id: Option<&str>,
    ) -> Result<ExecutionContext, AppError> {
        let agent = self
            .store
            .get_agent(agent_id)
            .ok_or_else(|| AppError::Agent(format!("未知 Agent: {agent_id}")))?;

        if !agent.enabled {
            return Err(AppError::Agent(format!("Agent 已禁用: {}", agent.name)));
        }

        let resolved_skill_id = skill_id.or(agent.default_skill_id.as_deref());
        let skill = if let Some(skill_id) = resolved_skill_id {
            if !self.store.agent_has_skill(&agent.id, skill_id) {
                return Err(AppError::Agent(format!(
                    "Agent {} 未关联 Skill: {skill_id}",
                    agent.name
                )));
            }
            let skill = self
                .store
                .get_skill(skill_id)
                .ok_or_else(|| AppError::Agent(format!("未知 Skill: {skill_id}")))?;
            if !skill.enabled {
                return Err(AppError::Agent(format!("Skill 已禁用: {}", skill.name)));
            }
            Some(skill)
        } else {
            None
        };

        let acp_server = self
            .acp_servers
            .get(&agent.default_acp_server_id)
            .ok_or_else(|| {
                AppError::Agent(format!("未知 ACP Server: {}", agent.default_acp_server_id))
            })?;
        if !acp_server.enabled {
            return Err(AppError::Agent(format!(
                "Agent {} 绑定的 ACP Server 已禁用: {}",
                agent.name, acp_server.name
            )));
        }

        Ok(ExecutionContext {
            agent,
            skill,
            acp_server,
        })
    }

    pub fn switch_conversation(
        &self,
        key: ConversationKey,
        agent_id: String,
    ) -> Result<ConversationExecutionState, AppError> {
        let context = self.context_for_agent(&agent_id, None)?;
        let state = ConversationExecutionState {
            key,
            agent_id: context.agent.id,
            skill_id: context.skill.map(|skill| skill.id),
        };
        self.store.save_conversation_state(state.clone());
        Ok(state)
    }

    pub fn select_skill(
        &self,
        key: ConversationKey,
        skill_id: String,
    ) -> Result<ConversationExecutionState, AppError> {
        let base = self
            .store
            .get_conversation_state(&key)
            .map(|state| state.agent_id)
            .or_else(|| self.store.default_agent().map(|agent| agent.id))
            .ok_or_else(|| AppError::Agent("未配置默认 Agent".to_string()))?;

        self.context_for_agent(&base, Some(&skill_id))?;
        let state = ConversationExecutionState {
            key,
            agent_id: base,
            skill_id: Some(skill_id),
        };
        self.store.save_conversation_state(state.clone());
        Ok(state)
    }

    pub fn reset_conversation(&self, key: &ConversationKey) {
        self.store.reset_conversation_state(key);
    }
}
