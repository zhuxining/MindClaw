use super::traits::{AgentAction, AgentCommand, AgentCommandContext};
use crate::error::AppResult;

/// /new — 创建新会话（关闭当前 Session）
pub struct NewCommand;

#[async_trait::async_trait]
impl AgentCommand for NewCommand {
    fn name(&self) -> &str {
        "new"
    }
    fn description(&self) -> &str {
        "Start a new conversation session"
    }

    async fn execute(&self, _ctx: AgentCommandContext) -> AppResult<AgentAction> {
        Ok(AgentAction::NewSession)
    }
}
