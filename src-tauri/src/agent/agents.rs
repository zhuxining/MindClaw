//! Agent 定义层
//!
//! 当前文件负责定义 Agent 的静态配置，而不是一次运行时的聚合实体。
//!
//! 当前阶段已完成：
//! - 将原 `types.rs` 中的运行时 `Agent` 收敛为定义态 `AgentProfile`
//! - 将 model / execution 默认值集中到 profile
//! - 提供从 profile 构建 `AgentRunSpec` 的最小桥接方法
//! - 按单文件优先原则承接 definition 边界；相关类型默认就近放置
//!
//! 当前仍未完成：
//! - PromptPolicy / ToolPolicy / SecurityPolicy 等更细粒度策略尚未拆出
//! - ModelRouter 当前仍是最小实现，只负责从 profile 取模型

use crate::agent::spec::AgentRunSpec;
use crate::providers::{ChatMessage, ToolSchema};
use crate::runtime::config::AppConfig;
use std::collections::HashMap;
use std::sync::Arc;

pub const MAIN_AGENT_ID: &str = "main";
pub const SUBAGENT_AGENT_ID: &str = "subagent-inline";
pub const BACKGROUND_AGENT_ID: &str = "background-agent";

/// Agent 形态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentKind {
    Main,
    SubAgent,
    BackgroundAgent,
}

/// Agent 静态定义
#[derive(Debug, Clone)]
pub struct AgentProfile {
    /// Profile 唯一标识
    pub id: String,
    /// Agent 形态
    pub kind: AgentKind,
    /// 可选的展示名
    pub name: Option<String>,
    /// 默认模型
    pub model: String,
    /// 执行迭代上限
    pub max_iterations: usize,
    /// 默认温度
    pub temperature: Option<f32>,
    /// 默认最大输出
    pub max_tokens: Option<usize>,
}

impl AgentProfile {
    pub fn new(id: impl Into<String>, kind: AgentKind, model: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            kind,
            name: None,
            model: model.into(),
            max_iterations: 8,
            temperature: None,
            max_tokens: None,
        }
    }

    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_execution(
        mut self,
        max_iterations: usize,
        temperature: Option<f32>,
        max_tokens: Option<usize>,
    ) -> Self {
        self.max_iterations = max_iterations;
        self.temperature = temperature;
        self.max_tokens = max_tokens;
        self
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// 将静态 profile 与本次运行的动态上下文合成为 `AgentRunSpec`
    pub fn build_run_spec(
        &self,
        system_prompt: String,
        messages: Vec<ChatMessage>,
        tools: Vec<ToolSchema>,
    ) -> AgentRunSpec {
        AgentRunSpec {
            system_prompt,
            messages,
            tools,
            model: self.model.clone(),
            max_iterations: self.max_iterations,
            temperature: self.temperature,
            max_tokens: self.max_tokens,
            parallel_tools: true,
            fail_on_tool_error: matches!(self.kind, AgentKind::BackgroundAgent),
        }
    }
}

/// Agent profile 注册表
#[derive(Default)]
pub struct AgentRegistry {
    profiles: HashMap<String, Arc<AgentProfile>>,
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn bootstrap(config: &AppConfig, default_model: impl Into<String>) -> Self {
        let default_model = default_model.into();
        let mut registry = Self::new();

        registry.register(Arc::new(
            AgentProfile::new(MAIN_AGENT_ID, AgentKind::Main, default_model.clone())
                .named("main")
                .with_execution(
                    config.agent_max_iterations,
                    config.agent_temperature,
                    config.agent_max_tokens,
                ),
        ));

        registry.register(Arc::new(
            AgentProfile::new(
                SUBAGENT_AGENT_ID,
                AgentKind::SubAgent,
                default_model.clone(),
            )
            .named("inline-subagent")
            .with_execution(15, Some(0.0), Some(2000)),
        ));

        registry.register(Arc::new(
            AgentProfile::new(
                BACKGROUND_AGENT_ID,
                AgentKind::BackgroundAgent,
                default_model,
            )
            .named("background-agent")
            .with_execution(15, Some(0.0), Some(2000)),
        ));

        registry
    }

    pub fn register(&mut self, profile: Arc<AgentProfile>) {
        self.profiles.insert(profile.id.clone(), profile);
    }

    pub fn get(&self, id: &str) -> Option<Arc<AgentProfile>> {
        self.profiles.get(id).cloned()
    }
}

/// 最小 ModelRouter
#[derive(Default, Clone)]
pub struct ModelRouter;

impl ModelRouter {
    pub fn new() -> Self {
        Self
    }

    pub fn resolve<'a>(&self, profile: &'a AgentProfile) -> &'a str {
        &profile.model
    }
}
