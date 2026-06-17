use serde::{Deserialize, Serialize};

/// Runtime 内部事件。
///
/// EventBus 只负责广播这些事件，不参与消息调度。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuntimeEvent {
    RuntimeStarted,
    RuntimeStopped,
    ChannelStarted {
        channel: String,
    },
    ChannelStopped {
        channel: String,
    },
    ChannelReconnecting {
        channel: String,
        reason: String,
    },
    ChannelPollStarted {
        channel: String,
    },
    ChannelPollSucceeded {
        channel: String,
        count: usize,
    },
    ChannelPollFailed {
        channel: String,
        error: String,
    },
    MessageReceived {
        message_id: String,
        channel: String,
        conversation_id: String,
    },
    MessageDeduplicated {
        message_id: String,
    },
    DispatchStarted {
        message_id: String,
        key: String,
    },
    DispatchSucceeded {
        message_id: String,
    },
    DispatchFailed {
        message_id: String,
        error: String,
    },
    ReplySent {
        message_id: String,
        channel: String,
        conversation_id: String,
    },
    ReplyFailed {
        message_id: String,
        error: String,
    },
}
