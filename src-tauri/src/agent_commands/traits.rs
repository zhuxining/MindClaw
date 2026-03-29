use crate::error::AppResult;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCommandContext {
    pub session_id: String,
    pub sender: String,
    pub args: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AgentAction {
    Reply { content: String },
    NewSession,
    StopAll,
    RestartAgent,
    Status { info: String },
}

/// AgentCommand trait：对话内 /xxx 指令实现此接口
#[async_trait::async_trait]
pub trait AgentCommand: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    async fn execute(&self, ctx: AgentCommandContext) -> AppResult<AgentAction>;
}
