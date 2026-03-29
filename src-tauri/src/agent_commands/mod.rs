pub mod new;
pub mod restart;
pub mod status;
pub mod stop;
pub mod traits;

use std::collections::HashMap;
use traits::AgentCommand;

/// Agent 控制指令注册表：注册、解析、分发 /xxx 指令
pub struct AgentCommandRegistry {
    commands: HashMap<String, Box<dyn AgentCommand + Send + Sync>>,
}

impl Default for AgentCommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentCommandRegistry {
    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
        }
    }

    pub fn register(&mut self, command: Box<dyn AgentCommand + Send + Sync>) {
        self.commands.insert(command.name().to_string(), command);
    }

    pub fn parse(input: &str) -> Option<(&str, &str)> {
        let trimmed = input.trim();
        if let Some(rest) = trimmed.strip_prefix('/') {
            let (cmd, args) = rest.split_once(' ').unwrap_or((rest, ""));
            Some((cmd, args))
        } else {
            None
        }
    }
}
