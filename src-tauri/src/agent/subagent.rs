//! Subagent manager for background task execution.
//!
//! AgentSpawnDispatcher — 子 Agent 委派调度器。
//! 复用 AgentRunner，不复用 AgentLoop（无需 Session 管理）。

use crate::agent::agent::{AgentProfile, SUBAGENT_AGENT_ID};
use crate::agent::context::SystemPromptBuilder;
use crate::agent::hooks::{
    IterationFinishContext, IterationStartContext, ModelResponseContext, RunAbortReason, RunHooks,
    ToolCallPlaceholder,
};
use crate::agent::messages::ChatMessage;
use crate::agent::runner::AgentRunner;
use crate::agent::spec::{AgentRunResult, InvocationMode, TokenUsage, ToolEvent, ToolStatus};
use crate::agent::tools::{build_tools, ToolScope};
use crate::bus::events::InboundMessage;
use crate::bus::MessageBus;
use crate::error::{AppError, AppResult};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore};
use tokio_util::sync::CancellationToken;

// ============================================================================
// SubagentStatus
// ============================================================================

/// SubAgent 运行阶段
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubagentPhase {
    Initializing,
    AwaitingTools,
    ToolsCompleted,
    FinalResponse,
    Done,
    Error,
}

/// SubAgent 实时状态
#[derive(Debug, Clone)]
pub struct SubagentStatus {
    pub task_id: String,
    pub label: String,
    pub phase: SubagentPhase,
    pub iteration: usize,
    pub tool_events: Vec<ToolEvent>,
    pub usage: TokenUsage,
    pub stop_reason: Option<String>,
    pub error: Option<String>,
}

impl SubagentStatus {
    fn new(task_id: String, label: String) -> Self {
        Self {
            task_id,
            label,
            phase: SubagentPhase::Initializing,
            iteration: 0,
            tool_events: Vec::new(),
            usage: TokenUsage::default(),
            stop_reason: None,
            error: None,
        }
    }
}

// ============================================================================
// AgentSpawnDispatcher
// ============================================================================

/// SubAgent 委派调度器
///
/// 持有 Runner 引用（复用），管理后台任务生命周期和状态跟踪。
pub struct AgentSpawnDispatcher {
    runner: Arc<AgentRunner>,
    bus: Arc<MessageBus>,
    workspace: PathBuf,
    main_agent: Arc<AgentProfile>,
    light_model: String,
    context_window: usize,
    tasks: Arc<RwLock<HashMap<String, CancellationToken>>>,
    statuses: Arc<RwLock<HashMap<String, SubagentStatus>>>,
    /// 当前活跃的父 session，由 AgentLoop 设置，用于 inject announce 消息
    parent_session: Arc<RwLock<Option<String>>>,
    gate: Arc<Semaphore>,
}

impl AgentSpawnDispatcher {
    pub fn new(
        runner: Arc<AgentRunner>,
        bus: Arc<MessageBus>,
        workspace: PathBuf,
        gate: Arc<Semaphore>,
        main_agent: Arc<AgentProfile>,
        light_model: String,
        context_window: usize,
    ) -> Self {
        Self {
            runner,
            bus,
            workspace,
            main_agent,
            light_model,
            context_window,
            tasks: Arc::new(RwLock::new(HashMap::new())),
            statuses: Arc::new(RwLock::new(HashMap::new())),
            parent_session: Arc::new(RwLock::new(None)),
            gate,
        }
    }

    /// 由 AgentLoop 在每次 run 前设置父 session
    pub async fn set_parent_session(&self, session_id: String) {
        *self.parent_session.write().await = Some(session_id);
    }

    /// 后台派生执行
    pub async fn spawn_background(&self, task: &str, label: Option<&str>) -> AppResult<String> {
        let task_id = uuid::Uuid::new_v4().to_string()[..8].to_string();
        let label = label.unwrap_or_else(|| truncate(task, 30)).to_string();

        if !self
            .main_agent
            .can_delegate_to(SUBAGENT_AGENT_ID, InvocationMode::Detached)
        {
            return Err(AppError::PermissionDenied(
                "background delegation disabled".into(),
            ));
        }

        let tools = build_tools(ToolScope::Subagent {
            workspace: self.workspace.clone(),
        })
        .await?;
        let prompt_builder = SystemPromptBuilder::new(self.workspace.clone());
        let system_prompt = prompt_builder.build_subagent_prompt(task);
        let profile = AgentProfile::subagent(self.light_model.clone());
        let mut spec = profile.build_run_spec(
            task_id.clone(),
            "background".to_string(),
            InvocationMode::Detached,
            system_prompt,
            vec![ChatMessage::user(task)],
            vec![],
        );
        spec.context_window_tokens = Some(self.context_window);

        let cancel = CancellationToken::new();
        self.tasks
            .write()
            .await
            .insert(task_id.clone(), cancel.clone());

        let status = SubagentStatus::new(task_id.clone(), label.clone());
        self.statuses.write().await.insert(task_id.clone(), status);

        let parent_session_id = self.parent_session.read().await.clone();
        let runner = Arc::clone(&self.runner);
        let bus = Arc::clone(&self.bus);
        let gate = Arc::clone(&self.gate);
        let task_id_clone = task_id.clone();
        let tasks_map = Arc::clone(&self.tasks);
        let statuses_map = Arc::clone(&self.statuses);

        tokio::spawn(async move {
            let _permit = gate.acquire_owned().await;
            let status_hook = SubagentStatusHook::new(statuses_map.clone(), task_id_clone.clone());

            let result = runner
                .run_spec(&spec, tools, Box::new(status_hook), cancel)
                .await;

            // 更新最终状态
            {
                let mut statuses = statuses_map.write().await;
                if let Some(s) = statuses.get_mut(&task_id_clone) {
                    match &result {
                        Ok(r) => {
                            s.phase = SubagentPhase::Done;
                            s.stop_reason = Some(format!("{:?}", r.stop_reason));
                            s.usage = r.usage.clone();
                            s.tool_events = r.tool_events.clone();
                        }
                        Err(e) => {
                            s.phase = SubagentPhase::Error;
                            s.error = Some(e.to_string());
                        }
                    }
                }
            }

            match result {
                Ok(r) => announce(&bus, &task_id_clone, Ok(&r), parent_session_id.as_deref()).await,
                Err(e) => {
                    tracing::error!(task_id = %task_id_clone, error = %e, "subagent_failed");
                    announce(&bus, &task_id_clone, Err(&e), parent_session_id.as_deref()).await
                }
            }

            tasks_map.write().await.remove(&task_id_clone);
        });

        tracing::info!(task_id = %task_id, label = %label, "subagent_spawned");
        Ok(format!(
            "Subagent [{}] started (id: {}). I'll notify you when complete.",
            label, task_id
        ))
    }

    /// 内联委托执行
    pub async fn delegate_inline(&self, task: &str) -> AppResult<AgentRunResult> {
        if !self
            .main_agent
            .can_delegate_to(SUBAGENT_AGENT_ID, InvocationMode::InlineChild)
        {
            return Err(AppError::PermissionDenied(
                "inline delegation disabled".into(),
            ));
        }

        let tools = build_tools(ToolScope::Subagent {
            workspace: self.workspace.clone(),
        })
        .await?;
        let prompt_builder = SystemPromptBuilder::new(self.workspace.clone());
        let system_prompt = prompt_builder.build_subagent_prompt(task);
        let run_id = uuid::Uuid::new_v4().to_string();

        let status = SubagentStatus::new(run_id.clone(), "inline".to_string());
        self.statuses.write().await.insert(run_id.clone(), status);
        let status_hook = SubagentStatusHook::new(self.statuses.clone(), run_id.clone());

        let mut spec = {
            let profile = AgentProfile::subagent(self.light_model.clone());
            profile.build_run_spec(
                run_id.clone(),
                format!("subagent-{}", &run_id[..8]),
                InvocationMode::InlineChild,
                system_prompt,
                vec![ChatMessage::user(task)],
                vec![],
            )
        };
        spec.context_window_tokens = Some(self.context_window);

        let result = self
            .runner
            .run_spec(
                &spec,
                tools,
                Box::new(status_hook),
                CancellationToken::new(),
            )
            .await;

        // 更新最终状态
        {
            let mut statuses = self.statuses.write().await;
            if let Some(s) = statuses.get_mut(&run_id) {
                match &result {
                    Ok(r) => {
                        s.phase = SubagentPhase::Done;
                        s.stop_reason = Some(format!("{:?}", r.stop_reason));
                        s.usage = r.usage.clone();
                        s.tool_events = r.tool_events.clone();
                    }
                    Err(e) => {
                        s.phase = SubagentPhase::Error;
                        s.error = Some(e.to_string());
                    }
                }
            }
        }

        result
    }

    /// 获取指定任务状态
    pub async fn get_status(&self, task_id: &str) -> Option<SubagentStatus> {
        self.statuses.read().await.get(task_id).cloned()
    }

    /// 获取所有运行中/已完成的任务状态
    pub async fn all_statuses(&self) -> Vec<SubagentStatus> {
        self.statuses.read().await.values().cloned().collect()
    }

    /// 取消所有任务
    pub async fn cancel_all(&self) -> usize {
        let mut cancelled = 0;
        let mut tasks = self.tasks.write().await;
        for (_id, cancel) in tasks.iter() {
            cancel.cancel();
            cancelled += 1;
        }
        tasks.clear();
        cancelled
    }

    /// 获取运行数量
    pub async fn running_count(&self) -> usize {
        self.tasks.read().await.len()
    }
}

// ============================================================================
// SubagentStatusHook — 记录子代理运行状态
// ============================================================================

struct SubagentStatusHook {
    statuses: Arc<RwLock<HashMap<String, SubagentStatus>>>,
    task_id: String,
}

impl SubagentStatusHook {
    fn new(statuses: Arc<RwLock<HashMap<String, SubagentStatus>>>, task_id: String) -> Self {
        Self { statuses, task_id }
    }
}

impl RunHooks for SubagentStatusHook {
    fn on_iteration_start(&mut self, ctx: &IterationStartContext) {
        if let Ok(mut statuses) = self.statuses.try_write() {
            if let Some(s) = statuses.get_mut(&self.task_id) {
                s.iteration = ctx.iteration;
            }
        }
    }

    fn on_model_response_ready(&mut self, ctx: &ModelResponseContext) {
        if let Ok(mut statuses) = self.statuses.try_write() {
            if let Some(s) = statuses.get_mut(&self.task_id) {
                if ctx.has_tool_calls {
                    s.phase = SubagentPhase::AwaitingTools;
                } else {
                    s.phase = SubagentPhase::FinalResponse;
                }
                s.usage = ctx.usage.clone();
            }
        }
    }

    fn on_tool_call_finish(
        &mut self,
        _call: &ToolCallPlaceholder,
        _success: bool,
        _output_summary: &str,
    ) {
        if let Ok(mut statuses) = self.statuses.try_write() {
            if let Some(s) = statuses.get_mut(&self.task_id) {
                s.phase = SubagentPhase::ToolsCompleted;
            }
        }
    }

    fn on_iteration_finish(&mut self, ctx: &IterationFinishContext) {
        if let Ok(mut statuses) = self.statuses.try_write() {
            if let Some(s) = statuses.get_mut(&self.task_id) {
                s.usage = ctx.usage.clone();
            }
        }
    }

    fn on_abort(&mut self, reason: &RunAbortReason) {
        if let Ok(mut statuses) = self.statuses.try_write() {
            if let Some(s) = statuses.get_mut(&self.task_id) {
                s.phase = SubagentPhase::Error;
                s.error = Some(format!("{:?}", reason));
            }
        }
    }
}

// ── 辅助函数 ─────────────────────────────────────────────────────

async fn announce(
    bus: &Arc<MessageBus>,
    task_id: &str,
    result: Result<&AgentRunResult, &AppError>,
    parent_session_id: Option<&str>,
) {
    let content = match result {
        Ok(r) => format_success(r),
        Err(e) => format!("**Error**: {}", e),
    };

    let msg = InboundMessage {
        id: uuid::Uuid::new_v4().to_string(),
        request_id: uuid::Uuid::new_v4().to_string(),
        session_id: parent_session_id.map(str::to_string),
        sender: "subagent".into(),
        channel: "system".into(),
        mode: crate::models::conversation::ConversationMode::Companion,
        content: format!(
            "## Subagent [{}] completed\n\n{}\n\nSummarize for user naturally.",
            task_id, content
        ),
        timestamp: chrono::Utc::now().timestamp(),
        is_injection: true,
    };

    let _ = bus.publish_inbound(msg).await;
}

fn format_success(r: &AgentRunResult) -> String {
    let tools = r
        .tool_events
        .iter()
        .map(|e| {
            format!(
                "- {}: {}",
                e.name,
                if matches!(e.status, ToolStatus::Succeeded) {
                    "✅"
                } else {
                    "❌"
                }
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "**Status**: ✅ Done\n\n**Steps**:\n{}\n\n**Result**:\n{}\n\n**Tokens**: {}",
        if tools.is_empty() { "None" } else { &tools },
        r.final_text,
        r.usage.total_tokens
    )
}

fn truncate(s: &str, max: usize) -> &str {
    if s.chars().count() <= max {
        s
    } else {
        &s[..s
            .char_indices()
            .take(max)
            .last()
            .map(|(i, _)| i)
            .unwrap_or(0)]
    }
}
