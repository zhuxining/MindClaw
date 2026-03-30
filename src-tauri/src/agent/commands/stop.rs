use super::traits::{AgentAction, AgentCommand, AgentCommandContext};
use crate::error::AppResult;

/// /stop — 停止所有进行中操作（取消 SubAgent 任务）
pub struct StopCommand;

#[async_trait::async_trait]
impl AgentCommand for StopCommand {
    fn name(&self) -> &str {
        "stop"
    }
    fn description(&self) -> &str {
        "Stop all in-progress operations and cancel sub-agent tasks"
    }

    async fn execute(&self, _ctx: AgentCommandContext) -> AppResult<AgentAction> {
        Ok(AgentAction::StopAll)
    }
}
