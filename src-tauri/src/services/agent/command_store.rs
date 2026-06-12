use super::types::SlashCommand;
use std::collections::HashMap;
use std::sync::RwLock;

pub struct CommandStore {
    commands: RwLock<HashMap<String, SlashCommand>>,
}

impl CommandStore {
    pub fn new() -> Self {
        Self {
            commands: RwLock::new(HashMap::new()),
        }
    }

    pub fn list(&self) -> Vec<SlashCommand> {
        self.commands.read().unwrap().values().cloned().collect()
    }

    pub fn get(&self, command: &str) -> Option<SlashCommand> {
        self.commands.read().unwrap().get(command).cloned()
    }

    pub fn save(&self, command: SlashCommand) {
        self.commands
            .write()
            .unwrap()
            .insert(command.command.clone(), command);
    }
}
