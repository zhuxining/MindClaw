//! Agent 核心模块
//!
//! 双层解耦架构：
//! - AgentLoop（业务编排层）：消息消费、会话管理、上下文构建、流式分发
//! - AgentRunner（纯执行层）：LLM 迭代循环、工具执行
//!
//! 核心数据类型：
//! - AgentRunSpec：声明式执行配置
//! - AgentRunResult：结构化执行结果
//!
//! 桥接机制：
//! - AgentHook：生命周期钩子，连接业务层与执行层

pub mod agent_loop;
pub mod builder;
pub mod commands;
pub mod context;
pub mod events;
pub mod hook;
pub mod memory;
pub mod observer;
pub mod runner;
pub mod session;
pub mod skills;
pub mod spec;
pub mod subagent;
pub mod tools;

// 重新导出主要类型
pub use agent_loop::AgentLoop;
pub use builder::AgentBuilder;
pub use context::{
    BuiltContext, ContextBuildContext, ContextFragment, ContextLayer, ContextPipeline,
    ContextSource, ConversationHistorySource, MessageRole, SystemPromptSource, UserMessageSource,
};
pub use events::{AgentEvent, ProviderEvent, RunPhase, UsageStats, UserVisiblePhase};
pub use hook::{AgentHook, LoopHook, NoOpHook, TestHook};
pub use memory::recall;
pub use observer::{AgentObserver, CompositeObserver, TracingObserver};
pub use runner::AgentRunner;
pub use session::{AgentSession, RunStatus, SessionManager, ToolTrace, TurnRecord};
pub use skills::{SkillManifest, SkillMetadata, SkillsRegistry};
pub use spec::{AgentRunResult, AgentRunSpec, IterationState, StopReason, TokenUsage, ToolEvent};
pub use subagent::{SubAgentDef, SubAgentInfo, SubAgentManager, SubAgentMode, SubAgentResult};
