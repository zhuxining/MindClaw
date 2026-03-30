pub mod new;
pub mod restart;
pub mod status;
pub mod stop;
pub mod traits;

use std::collections::HashMap;
use traits::AgentCommand;
use {new::NewCommand, restart::RestartCommand, status::StatusCommand, stop::StopCommand};

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

    pub fn get(&self, name: &str) -> Option<&(dyn AgentCommand + Send + Sync)> {
        self.commands.get(name).map(|command| command.as_ref())
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

impl AgentCommandRegistry {
    pub fn with_builtins() -> Self {
        let mut registry = Self::new();
        registry.register(Box::new(NewCommand));
        registry.register(Box::new(StopCommand));
        registry.register(Box::new(RestartCommand));
        registry.register(Box::new(StatusCommand));
        registry
    }
}
