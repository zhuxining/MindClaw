use super::types::{AgentSkillBinding, Skill};
use std::collections::HashMap;
use std::sync::RwLock;

pub struct SkillStore {
    skills: RwLock<HashMap<String, Skill>>,
    bindings: RwLock<Vec<AgentSkillBinding>>,
}

impl SkillStore {
    pub fn new() -> Self {
        Self {
            skills: RwLock::new(HashMap::new()),
            bindings: RwLock::new(Vec::new()),
        }
    }

    pub fn list_skills(&self) -> Vec<Skill> {
        self.skills.read().unwrap().values().cloned().collect()
    }

    pub fn get_skill(&self, id: &str) -> Option<Skill> {
        self.skills.read().unwrap().get(id).cloned()
    }

    pub fn save_skill(&self, skill: Skill) {
        self.skills.write().unwrap().insert(skill.id.clone(), skill);
    }

    pub fn bind(&self, agent_id: String, skill_id: String) {
        let mut bindings = self.bindings.write().unwrap();
        if !bindings
            .iter()
            .any(|binding| binding.agent_id == agent_id && binding.skill_id == skill_id)
        {
            bindings.push(AgentSkillBinding { agent_id, skill_id });
        }
    }

    pub fn agent_has_skill(&self, agent_id: &str, skill_id: &str) -> bool {
        self.bindings
            .read()
            .unwrap()
            .iter()
            .any(|binding| binding.agent_id == agent_id && binding.skill_id == skill_id)
    }
}
