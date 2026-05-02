//! Agent 核心模块
//!
//! 双层解耦架构：
//! - AgentLoop（业务编排层）：消息消费、会话管理、上下文构建、流式分发
//! - AgentRunner（Rig 执行内核）：LLM 迭代循环、工具执行
//!
//! 核心数据类型：
//! - AgentRunSpec：声明式执行配置
//! - AgentRunResult：结构化执行结果
//!
//! 桥接机制：
//! - RunHooks：生命周期钩子，连接业务层与执行层

pub mod agents;
pub mod compact;
pub mod context;
pub mod events;
pub mod hooks;
pub mod loop_;
pub mod memory;
pub mod messages;
pub mod retry;
pub mod runner;
pub mod session;
pub mod skills;
pub mod spawn;
pub mod spec;
pub mod tools;

// 重新导出主要类型
pub use agents::{AgentKind, AgentProfile, AgentRegistry, ModelRouter};
pub use compact::{AutoCompact, AutoCompactConfig, CompactResult};
pub use context::{
    BuiltContext, ContextBuildState, ContextFragment, ContextLayer, ContextPipeline, ContextSource,
    ConversationHistorySource, SystemPromptSource, UserMessageSource,
};
pub use events::UserVisiblePhase;
pub use hooks::{
    strip_think_tags, CompositeRunHooks, InteractiveRunHooks, IterationFinishContext,
    IterationStartContext, ModelRequestContext, ModelResponseContext, NoopRunHooks, RunAbortReason,
    RunHookPublisher, RunHooks, RunStartContext, StreamingMode,
};
pub use loop_::AgentLoop;
pub use memory::{Memory, MemoryCategory, MemoryStore, UpsertMemoryInput};
pub use messages::{ChatMessage, MessageContent, MessageRole, ToolChoice, ToolSchema};
pub use retry::{extract_retry_after, RetryMode, RetryPolicy};
pub use runner::AgentRunner;
pub use session::{
    AgentSession, CheckpointPhase, PendingToolCall, RunStatus, RuntimeCheckpoint, SessionManager,
    ToolTrace, TurnRecord,
};
pub use skills::{SkillManifest, SkillMetadata, SkillsRegistry};
pub use spawn::{
    AgentSpawnDispatcher, CapabilityProfile, RoutingContext, SpawnSource, SubAgentDefinition,
    SubAgentInfo, SubAgentMode, SubAgentResult,
};
pub use spec::{
    AgentRunResult, AgentRunSpec, InvocationMode, IterationState, StopReason, TokenUsage, ToolEvent,
};
