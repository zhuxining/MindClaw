//! Agent 定义层
//!
//! 本文件负责定义 Agent 的静态配置（AgentProfile），包含 8 个 Policy Group：
//! - PromptPolicy: 提示词模板与版本管理
//! - ModelPolicy: 模型选择与生成参数
//! - ToolPolicy: 工具白名单与执行策略
//! - ContextPolicy: 上下文窗口与记忆策略
//! - ExecutionPolicy: 执行迭代与输出格式
//! - SecurityPolicy: 工作区与权限边界
//! - DelegationPolicy: 子代理委托策略
//! - InvocationPolicy: 调用模式与可见性
//!
//! Profile 不是 Instance：Profile 描述"应该如何运行"，不是"正在运行什么"。

use crate::agent::spec::{AgentRunSpec, InvocationMode};
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

// ============================================================================
// Policy Groups - 8个策略组
// ============================================================================

/// 提示词策略
#[derive(Debug, Clone)]
pub struct PromptPolicy {
    /// 系统提示词模板
    pub system_prompt_template: String,
    /// 提示词版本（用于缓存失效）
    pub prompt_version: String,
    /// 额外指令（追加到系统提示词）
    pub extra_instructions: Option<String>,
}

impl Default for PromptPolicy {
    fn default() -> Self {
        Self {
            system_prompt_template: String::new(),
            prompt_version: "1.0".to_string(),
            extra_instructions: None,
        }
    }
}

/// 模型策略
#[derive(Debug, Clone)]
pub struct ModelPolicy {
    /// 首选模型标识
    pub preferred_model: String,
    /// 回退模型列表（按优先级）
    pub fallback_models: Vec<String>,
    /// 允许的提供商白名单
    pub provider_allowlist: Vec<String>,
    /// 必需的模型能力
    pub required_capabilities: Vec<String>,
    /// 采样温度
    pub temperature: Option<f32>,
    /// 最大输出token数
    pub max_tokens: Option<usize>,
    /// 推理努力程度（如 Claude 的 reasoning_effort）
    pub reasoning_effort: Option<String>,
}

impl ModelPolicy {
    pub fn new(preferred_model: impl Into<String>) -> Self {
        Self {
            preferred_model: preferred_model.into(),
            fallback_models: Vec::new(),
            provider_allowlist: Vec::new(),
            required_capabilities: Vec::new(),
            temperature: None,
            max_tokens: None,
            reasoning_effort: None,
        }
    }
}

/// 工具策略
#[derive(Debug, Clone)]
pub struct ToolPolicy {
    /// 允许的工具名称列表（空表示全部）
    pub allowed_tools: Vec<String>,
    /// 工具选择模式
    pub tool_choice: ToolChoice,
    /// 是否并行执行工具
    pub parallel_tools: bool,
    /// 单次run中工具调用次数上限
    pub max_tool_calls: Option<usize>,
    /// 单个工具超时（毫秒）
    pub tool_timeout_ms: Option<u64>,
    /// 工具错误是否立即中止
    pub fail_on_tool_error: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToolChoice {
    #[default]
    Auto, // 模型决定是否调用工具
    Required, // 必须调用至少一个工具
    None,     // 禁止调用工具
}

impl Default for ToolPolicy {
    fn default() -> Self {
        Self {
            allowed_tools: Vec::new(),
            tool_choice: ToolChoice::Auto,
            parallel_tools: true,
            max_tool_calls: None,
            tool_timeout_ms: None,
            fail_on_tool_error: false,
        }
    }
}

/// 上下文策略
#[derive(Debug, Clone)]
pub struct ContextPolicy {
    /// 上下文预算（token数）
    pub context_budget: Option<usize>,
    /// 历史消息处理策略
    pub history_policy: HistoryPolicy,
    /// 记忆检索策略
    pub memory_policy: MemoryPolicy,
    /// 压缩策略
    pub compression_policy: CompressionPolicy,
    /// 是否包含知识库
    pub include_knowledge: bool,
    /// 是否包含技能定义
    pub include_skills: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HistoryPolicy {
    #[default]
    Full, // 保留完整历史
    Sliding,    // 滑动窗口
    Summarized, // 摘要压缩
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MemoryPolicy {
    #[default]
    Disabled,
    Session,      // 仅当前session
    CrossSession, // 跨session
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CompressionPolicy {
    #[default]
    None,
    Truncate,  // 截断
    Summarize, // 摘要
}

impl Default for ContextPolicy {
    fn default() -> Self {
        Self {
            context_budget: None,
            history_policy: HistoryPolicy::Full,
            memory_policy: MemoryPolicy::Disabled,
            compression_policy: CompressionPolicy::None,
            include_knowledge: false,
            include_skills: true,
        }
    }
}

/// 执行策略
#[derive(Debug, Clone)]
pub struct ExecutionPolicy {
    /// 最大迭代次数（模型调用次数上限）
    pub max_iterations: usize,
    /// 流式输出策略
    pub streaming_policy: StreamingPolicy,
    /// 输出格式
    pub output_format: OutputFormat,
    /// 响应Schema（用于结构化输出）
    pub response_schema: Option<serde_json::Value>,
    /// 停止序列
    pub stop_sequences: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StreamingPolicy {
    #[default]
    Disabled,
    TextOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    #[default]
    Text,
    Markdown,
    Json,       // 纯JSON
    Structured, // 基于schema的结构化输出
}

impl Default for ExecutionPolicy {
    fn default() -> Self {
        Self {
            max_iterations: 8,
            streaming_policy: StreamingPolicy::TextOnly,
            output_format: OutputFormat::Markdown,
            response_schema: None,
            stop_sequences: Vec::new(),
        }
    }
}

/// 安全策略
#[derive(Debug, Clone)]
pub struct SecurityPolicy {
    /// 允许访问的工作区根目录
    pub workspace_roots: Vec<std::path::PathBuf>,
    /// 路径访问策略
    pub path_policy: PathPolicy,
    /// 权限配置文件
    pub permission_profile: PermissionProfile,
    /// 网络访问策略
    pub network_policy: NetworkPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PathPolicy {
    #[default]
    WorkspaceOnly, // 仅工作区内
    Allowlisted,  // 白名单模式
    Unrestricted, // 无限制（危险）
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PermissionProfile {
    #[default]
    Restricted, // 最严格
    Standard,
    Elevated, // 需要用户确认
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NetworkPolicy {
    #[default]
    Blocked, // 禁止网络
    Allowlisted, // 仅允许特定域名
    Full,        // 完全访问
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self {
            workspace_roots: Vec::new(),
            path_policy: PathPolicy::WorkspaceOnly,
            permission_profile: PermissionProfile::Standard,
            network_policy: NetworkPolicy::Allowlisted,
        }
    }
}

/// 委托策略 - 控制子代理行为
#[derive(Debug, Clone)]
pub struct DelegationPolicy {
    /// 是否允许创建子代理
    pub allow_subagents: bool,
    /// 是否允许创建后台代理
    pub allow_background_agents: bool,
    /// 允许的子代理类型列表（空表示全部）
    pub allowed_child_agents: Vec<String>,
    /// 子代理嵌套深度上限（0表示禁止嵌套）
    pub max_child_depth: usize,
    /// 最大并行子代理数
    pub max_parallel_children: usize,
    /// 子代理超时（毫秒）
    pub child_timeout_ms: Option<u64>,
}

impl Default for DelegationPolicy {
    fn default() -> Self {
        Self {
            allow_subagents: true,
            allow_background_agents: true,
            allowed_child_agents: Vec::new(),
            max_child_depth: 3,
            max_parallel_children: 5,
            child_timeout_ms: Some(300_000), // 5分钟
        }
    }
}

impl DelegationPolicy {
    /// 创建严格模式（禁止所有委托）
    pub fn strict() -> Self {
        Self {
            allow_subagents: false,
            allow_background_agents: false,
            allowed_child_agents: Vec::new(),
            max_child_depth: 0,
            max_parallel_children: 0,
            child_timeout_ms: None,
        }
    }
}

/// 调用策略 - 控制如何被调用和结果可见性
#[derive(Debug, Clone)]
pub struct InvocationPolicy {
    /// 可被谁调用（agent id列表，空表示任意）
    pub callable_by: Vec<String>,
    /// 是否继承父代理的工作区范围
    pub inherits_workspace_scope: bool,
    /// 是否继承父代理的记忆
    pub inherits_memory: bool,
    /// 是否将结果发布给用户
    pub publish_result_to_user: bool,
}

impl Default for InvocationPolicy {
    fn default() -> Self {
        Self {
            callable_by: Vec::new(),
            inherits_workspace_scope: true,
            inherits_memory: false,
            publish_result_to_user: true,
        }
    }
}

// ============================================================================
// AgentProfile - 静态定义
// ============================================================================

/// Agent 静态定义
///
/// 负责定义某类 Agent 的身份、策略和硬边界，不负责任何运行时状态。
/// 策略与状态分离：Session、消息历史、取消状态等均属于 Runtime。
#[derive(Debug, Clone)]
pub struct AgentProfile {
    // 基础身份
    pub id: String,
    pub kind: AgentKind,
    pub name: Option<String>,
    pub description: Option<String>,

    // 8个策略组
    pub prompt_policy: PromptPolicy,
    pub model_policy: ModelPolicy,
    pub tool_policy: ToolPolicy,
    pub context_policy: ContextPolicy,
    pub execution_policy: ExecutionPolicy,
    pub security_policy: SecurityPolicy,
    pub delegation_policy: DelegationPolicy,
    pub invocation_policy: InvocationPolicy,
}

impl AgentProfile {
    /// 创建最小化 Profile（仅必需字段）
    pub fn new(id: impl Into<String>, kind: AgentKind, model: impl Into<String>) -> Self {
        let id = id.into();
        let model = model.into();
        Self {
            id: id.clone(),
            kind,
            name: None,
            description: None,
            prompt_policy: PromptPolicy::default(),
            model_policy: ModelPolicy::new(model),
            tool_policy: ToolPolicy::default(),
            context_policy: ContextPolicy::default(),
            execution_policy: ExecutionPolicy::default(),
            security_policy: SecurityPolicy::default(),
            delegation_policy: DelegationPolicy::default(),
            invocation_policy: InvocationPolicy::default(),
        }
    }

    /// 设置展示名
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// 设置描述
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// 设置提示词策略
    pub fn with_prompt_policy(mut self, policy: PromptPolicy) -> Self {
        self.prompt_policy = policy;
        self
    }

    /// 设置模型策略
    pub fn with_model_policy(mut self, policy: ModelPolicy) -> Self {
        self.model_policy = policy;
        self
    }

    /// 设置工具策略
    pub fn with_tool_policy(mut self, policy: ToolPolicy) -> Self {
        self.tool_policy = policy;
        self
    }

    /// 设置上下文策略
    pub fn with_context_policy(mut self, policy: ContextPolicy) -> Self {
        self.context_policy = policy;
        self
    }

    /// 设置执行策略
    pub fn with_execution_policy(mut self, policy: ExecutionPolicy) -> Self {
        self.execution_policy = policy;
        self
    }

    /// 设置安全策略
    pub fn with_security_policy(mut self, policy: SecurityPolicy) -> Self {
        self.security_policy = policy;
        self
    }

    /// 设置委托策略
    pub fn with_delegation_policy(mut self, policy: DelegationPolicy) -> Self {
        self.delegation_policy = policy;
        self
    }

    /// 设置调用策略
    pub fn with_invocation_policy(mut self, policy: InvocationPolicy) -> Self {
        self.invocation_policy = policy;
        self
    }

    /// 便捷方法：设置系统提示词模板
    pub fn with_system_prompt(mut self, template: impl Into<String>) -> Self {
        self.prompt_policy.system_prompt_template = template.into();
        self
    }

    /// 便捷方法：设置执行参数
    pub fn with_execution(
        mut self,
        max_iterations: usize,
        temperature: Option<f32>,
        max_tokens: Option<usize>,
    ) -> Self {
        self.execution_policy.max_iterations = max_iterations;
        self.model_policy.temperature = temperature;
        self.model_policy.max_tokens = max_tokens;
        self
    }

    /// 便捷方法：设置模型
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model_policy.preferred_model = model.into();
        self
    }

    /// 将静态 profile 与本次运行的动态上下文合成为 `AgentRunSpec`
    pub fn build_run_spec(
        &self,
        run_id: impl Into<String>,
        session_id: impl Into<String>,
        invocation: InvocationMode,
        system_prompt: String,
        messages: Vec<ChatMessage>,
        tools: Vec<ToolSchema>,
    ) -> AgentRunSpec {
        AgentRunSpec {
            run_id: run_id.into(),
            session_id: session_id.into(),
            agent_id: self.id.clone(),
            invocation,
            system_prompt,
            messages,
            tools,
            model: self.model_policy.preferred_model.clone(),
            max_iterations: self.execution_policy.max_iterations,
            temperature: self.model_policy.temperature,
            max_tokens: self.model_policy.max_tokens,
            parallel_tools: self.tool_policy.parallel_tools,
            fail_on_tool_error: self.tool_policy.fail_on_tool_error,
        }
    }

    /// 检查是否允许委托给指定 agent
    ///
    /// - `child_id`：目标 agent ID（用于区分 subagent vs background agent）
    /// - `current_depth`：当前委托嵌套深度（0 = 主 Agent 发起）
    pub fn can_delegate_to(&self, child_id: &str, current_depth: usize) -> bool {
        let is_background = child_id == BACKGROUND_AGENT_ID;

        // 检查对应的开关
        if is_background && !self.delegation_policy.allow_background_agents {
            return false;
        }
        if !is_background && !self.delegation_policy.allow_subagents {
            return false;
        }

        // 深度仅约束 inline_child；detached 不计入深度链
        if !is_background && current_depth >= self.delegation_policy.max_child_depth {
            return false;
        }

        // 白名单检查（空表示全部允许）
        if !self.delegation_policy.allowed_child_agents.is_empty()
            && !self
                .delegation_policy
                .allowed_child_agents
                .iter()
                .any(|s| s == child_id)
        {
            return false;
        }

        true
    }
}

// ============================================================================
// AgentRegistry - Profile 注册表
// ============================================================================

/// Agent profile 注册表
#[derive(Default)]
pub struct AgentRegistry {
    profiles: HashMap<String, Arc<AgentProfile>>,
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// 启动时初始化默认 profiles
    pub fn bootstrap(config: &AppConfig, default_model: impl Into<String>) -> Self {
        let default_model = default_model.into();
        let mut registry = Self::new();

        // Main Agent
        registry.register(Arc::new(
            AgentProfile::new(MAIN_AGENT_ID, AgentKind::Main, default_model.clone())
                .named("main")
                .with_execution(
                    config.agent_max_iterations,
                    config.agent_temperature,
                    config.agent_max_tokens,
                )
                .with_delegation_policy(DelegationPolicy {
                    allow_subagents: true,
                    allow_background_agents: true,
                    max_child_depth: 3,
                    ..Default::default()
                }),
        ));

        // SubAgent (inline)
        registry.register(Arc::new(
            AgentProfile::new(
                SUBAGENT_AGENT_ID,
                AgentKind::SubAgent,
                default_model.clone(),
            )
            .named("inline-subagent")
            .with_execution(15, Some(0.0), Some(2000))
            .with_delegation_policy(DelegationPolicy {
                allow_subagents: true,
                allow_background_agents: false,
                max_child_depth: 2,
                ..Default::default()
            })
            .with_invocation_policy(InvocationPolicy {
                publish_result_to_user: false,
                ..Default::default()
            }),
        ));

        // Background Agent
        registry.register(Arc::new(
            AgentProfile::new(
                BACKGROUND_AGENT_ID,
                AgentKind::BackgroundAgent,
                default_model,
            )
            .named("background-agent")
            .with_execution(15, Some(0.0), Some(2000))
            .with_tool_policy(ToolPolicy {
                fail_on_tool_error: true,
                ..Default::default()
            })
            .with_delegation_policy(DelegationPolicy {
                allow_subagents: false,
                allow_background_agents: false,
                max_child_depth: 0,
                ..Default::default()
            })
            .with_invocation_policy(InvocationPolicy {
                publish_result_to_user: true,
                ..Default::default()
            }),
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

// ============================================================================
// ModelRouter - 模型路由
// ============================================================================

/// 模型路由器
#[derive(Default, Clone)]
pub struct ModelRouter;

impl ModelRouter {
    pub fn new() -> Self {
        Self
    }

    /// 从 profile 解析实际使用的模型
    pub fn resolve<'a>(&self, profile: &'a AgentProfile) -> &'a str {
        &profile.model_policy.preferred_model
    }

    /// 获取带 fallback 的模型列表
    pub fn resolve_with_fallbacks<'a>(&self, profile: &'a AgentProfile) -> Vec<&'a str> {
        let mut models = vec![profile.model_policy.preferred_model.as_str()];
        for fallback in &profile.model_policy.fallback_models {
            models.push(fallback.as_str());
        }
        models
    }
}
