pub mod agent_loop;
pub mod commands;
pub mod context;
pub mod events;
pub mod hooks;
pub mod observer;
pub mod session;
pub mod sub_agent;
pub mod tools;

// 重新导出主要类型
pub use agent_loop::AgentLoop;
pub use context::{
    BuiltContext, ContextBuildContext, ContextFragment, ContextPipeline, ContextSource,
    ConversationHistorySource, MessageRole, SystemPromptSource, UserMessageSource,
};
pub use events::{AgentEvent, ProviderEvent, RunPhase, UsageStats, UserVisiblePhase};
pub use observer::{AgentObserver, CompositeObserver, TracingObserver};
pub use session::{AgentSession, RunStatus, SessionManager, ToolTrace, TurnRecord};
pub use sub_agent::{
    SubAgentContext, SubAgentDef, SubAgentInfo, SubAgentMode, SubAgentRegistry, SubAgentResult,
};
