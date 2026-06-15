use crate::services::core::ChannelMessage;
use tokio::sync::oneshot;

use crate::error::AppError;
use crate::services::core::AgentResponse;

pub(crate) struct DispatchCommand {
    pub(crate) message: ChannelMessage,
    pub(crate) responder: oneshot::Sender<Result<AgentResponse, AppError>>,
}
