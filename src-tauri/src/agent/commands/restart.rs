use super::traits::{AgentAction, AgentCommand, AgentCommandContext};
use crate::error::AppResult;

/// /restart — 重启 Agent 服务（重新初始化）
pub struct RestartCommand;

#[async_trait::async_trait]
impl AgentCommand for RestartCommand {
    fn name(&self) -> &str {
        "restart"
    }
    fn description(&self) -> &str {
        "Restart the agent service and reinitialize"
    }

    async fn execute(&self, _ctx: AgentCommandContext) -> AppResult<AgentAction> {
        Ok(AgentAction::RestartAgent)
    }
}
