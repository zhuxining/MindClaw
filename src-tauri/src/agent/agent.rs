//! Agent 定义与主入口
//!
//! Agent — 整个项目的主 Agent，自包含初始化，持有 loop、runner、subagent
//! AgentProfile — Agent 静态配置（模型、策略、权限）

use crate::agent::context::ContextPipeline;
use crate::agent::loop_::AgentLoop;
use crate::agent::messages::{ChatMessage, ToolChoice, ToolSchema};
use crate::agent::retry::RetryMode;
use crate::agent::runner::AgentRunner;
use crate::agent::session::SessionManager;
use crate::agent::spec::{AgentRunSpec, InvocationMode};
use crate::agent::subagent::AgentSpawnDispatcher;
use crate::bus::MessageBus;
use crate::error::{AppError, AppResult};
use crate::runtime::config::AppConfig;
use std::sync::Arc;
use tokio::sync::Semaphore;

pub const MAIN_AGENT_ID: &str = "main";
pub const SUBAGENT_AGENT_ID: &str = "subagent";

// ============================================================================
// AgentProfile
// ============================================================================

/// Agent 静态配置
///
/// 不是运行时实体——不持有 Session、Provider 或运行时状态。
/// 描述"应该如何运行"，而非"正在运行什么"。
#[derive(Debug, Clone)]
pub struct AgentProfile {
    pub id: String,
    pub model: String,
    pub max_iterations: usize,
    pub temperature: Option<f32>,
    pub max_tokens: Option<usize>,
    pub tool_choice: ToolChoice,
    pub fail_on_tool_error: bool,
    pub allow_subagents: bool,
    pub allow_background_delegation: bool,
}

impl AgentProfile {
    /// 创建 Main Agent Profile
    pub fn main(config: &AppConfig, model: String) -> Self {
        Self {
            id: MAIN_AGENT_ID.to_string(),
            model,
            max_iterations: config.agent_max_iterations,
            temperature: config.agent_temperature,
            max_tokens: config.agent_max_tokens,
            tool_choice: ToolChoice::Auto,
            fail_on_tool_error: false,
            allow_subagents: true,
            allow_background_delegation: true,
        }
    }

    /// 创建 SubAgent Profile（固定配置）
    pub fn subagent(light_model: String) -> Self {
        Self {
            id: SUBAGENT_AGENT_ID.to_string(),
            model: light_model,
            max_iterations: 15,
            temperature: Some(0.0),
            max_tokens: Some(2000),
            tool_choice: ToolChoice::Auto,
            fail_on_tool_error: true,
            allow_subagents: false,
            allow_background_delegation: false,
        }
    }

    /// 检查是否允许委托
    pub fn can_delegate_to(&self, child_id: &str, invocation: InvocationMode) -> bool {
        if child_id == MAIN_AGENT_ID {
            return false;
        }
        match invocation {
            InvocationMode::InlineChild => self.allow_subagents,
            InvocationMode::Detached => self.allow_background_delegation,
            InvocationMode::Interactive => false,
        }
    }

    /// 构建 AgentRunSpec
    pub fn build_run_spec(
        &self,
        run_id: String,
        session_id: String,
        invocation: InvocationMode,
        system_prompt: String,
        messages: Vec<ChatMessage>,
        tools: Vec<ToolSchema>,
    ) -> AgentRunSpec {
        AgentRunSpec {
            run_id,
            session_id,
            agent_id: self.id.clone(),
            invocation,
            system_prompt,
            messages,
            tools,
            model: self.model.clone(),
            max_iterations: self.max_iterations,
            temperature: self.temperature,
            max_tokens: self.max_tokens,
            parallel_tools: true,
            tool_choice: self.tool_choice.clone(),
            fail_on_tool_error: self.fail_on_tool_error,
            retry_mode: RetryMode::Standard,
            context_window_tokens: None,
        }
    }
}

// ============================================================================
// Agent — 主 Agent 入口
// ============================================================================

/// 主 Agent — 整个项目的 Agent 入口
///
/// 自包含初始化，持有 loop_（会话级编排）、runner（工具循环级执行）、
/// spawn_dispatcher（子 Agent 委派）等所有核心组件。
///
/// ```text
/// Agent::init()
///   ├─ ContextPipeline::build_default()
///   ├─ AgentSpawnDispatcher::new(...)
///   └─ AgentLoop::new(...)
///
/// Agent::run()
///   └─ AgentLoop::run()  // 消费 MessageBus 消息
/// ```
pub struct Agent {
    loop_: Arc<AgentLoop>,
    runner: Arc<AgentRunner>,
    #[allow(dead_code)]
    spawn_dispatcher: Arc<AgentSpawnDispatcher>,
    bus: Arc<MessageBus>,
    session_mgr: Arc<SessionManager>,
    config: Arc<AppConfig>,
}

impl Agent {
    /// 自包含初始化：创建 ContextPipeline、AgentSpawnDispatcher、AgentLoop
    pub async fn init(
        config: Arc<AppConfig>,
        bus: Arc<MessageBus>,
        session_mgr: Arc<SessionManager>,
        runner: Arc<AgentRunner>,
        main_agent: Arc<AgentProfile>,
        light_model: String,
    ) -> AppResult<Self> {
        let concurrency_gate = Arc::new(Semaphore::new(config.llm_concurrency));

        let context_pipeline = ContextPipeline::build_default(&config);

        let spawn_dispatcher = Arc::new(AgentSpawnDispatcher::new(
            Arc::clone(&runner),
            Arc::clone(&bus),
            config.data_dir().clone(),
            Arc::clone(&concurrency_gate),
            Arc::clone(&main_agent),
            light_model,
            config.context_token_limit,
        ));

        let loop_ = Arc::new(AgentLoop::new(
            Arc::clone(&bus),
            Arc::clone(&session_mgr),
            context_pipeline,
            Arc::clone(&runner),
            Arc::clone(&config),
            main_agent,
            Arc::clone(&spawn_dispatcher),
            concurrency_gate,
        ));

        Ok(Self {
            loop_,
            runner,
            spawn_dispatcher,
            bus,
            session_mgr,
            config,
        })
    }

    /// 启动消息循环（阻塞，消费 MessageBus 入站消息）
    pub async fn run(self: Arc<Self>) -> Result<(), AppError> {
        AgentLoop::run(self.loop_.clone()).await
    }

    // --- Accessors ---

    pub fn bus(&self) -> &Arc<MessageBus> {
        &self.bus
    }

    pub fn session_mgr(&self) -> &Arc<SessionManager> {
        &self.session_mgr
    }

    pub fn config(&self) -> &Arc<AppConfig> {
        &self.config
    }

    pub fn loop_(&self) -> &Arc<AgentLoop> {
        &self.loop_
    }

    pub fn runner(&self) -> &Arc<AgentRunner> {
        &self.runner
    }
}
