use super::traits::{AgentAction, AgentCommand, AgentCommandContext};
use crate::error::AppResult;

/// /status — 查看 Agent 状态、连接、队列
pub struct StatusCommand;

#[async_trait::async_trait]
impl AgentCommand for StatusCommand {
    fn name(&self) -> &str {
        "status"
    }
    fn description(&self) -> &str {
        "Show agent status, connections, and queue info"
    }

    async fn execute(&self, _ctx: AgentCommandContext) -> AppResult<AgentAction> {
        let info = "Agent: running | Channels: desktop | Queue: 0".to_string();
        Ok(AgentAction::Status { info })
    }
}
