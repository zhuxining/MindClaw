//! SkillRegistry：技能系统
//!
//! 技能是 Agent 能力的组合包，可以注册：
//! - Tools（工具）
//! - ContextSources（上下文源）
//! - Hooks（钩子）
//! - SubAgentTasks（子代理任务）
//! - Operations（业务操作）

use crate::agent::context_pipeline::ContextSource;
use crate::agent::hooks::{Hook, HookRegistry};
use std::sync::Arc;

/// 技能 trait：组合注册多个组件
///
/// 一个 Skill 可以包含多个组件，统一注册到各自的 Registry 中
pub trait Skill: Send + Sync {
    /// 技能名称
    fn name(&self) -> &str;
    /// 版本号
    fn version(&self) -> &str;
    /// 获取技能提供的上下文源
    fn context_sources(&self) -> Vec<Arc<dyn ContextSource>>;
    /// 获取技能提供的钩子
    fn hooks(&self) -> Vec<Arc<dyn Hook>>;
}

/// 技能注册表
pub struct SkillRegistry {
    skills: Vec<Box<dyn Skill>>,
}

impl SkillRegistry {
    /// 创建新的技能注册表
    pub fn new() -> Self {
        Self { skills: Vec::new() }
    }

    /// 注册技能
    pub fn register(&mut self, skill: Box<dyn Skill>) {
        tracing::info!(
            skill_name = %skill.name(),
            skill_version = %skill.version(),
            "skill_registered"
        );
        self.skills.push(skill);
    }

    /// 获取所有技能提供的上下文源
    pub fn all_context_sources(&self) -> Vec<Arc<dyn ContextSource>> {
        let mut sources = Vec::new();
        for skill in &self.skills {
            sources.extend(skill.context_sources());
        }
        sources
    }

    /// 获取所有技能提供的钩子
    pub fn all_hooks(&self) -> Vec<Arc<dyn Hook>> {
        let mut hooks = Vec::new();
        for skill in &self.skills {
            hooks.extend(skill.hooks());
        }
        hooks
    }

    /// 将技能的钩子注册到 HookRegistry
    pub fn register_hooks_to(&self, _registry: &mut HookRegistry) {
        for hook in self.all_hooks() {
            // 需要将 Arc<dyn Hook> 转换为 Box<dyn Hook>
            // 这里我们直接使用 Arc，HookRegistry 需要支持 Arc
            // 为简化，这里先记录日志
            tracing::debug!(hook_name = %hook.name(), "hook_would_register");
        }
    }

    /// 获取已注册的技能数量
    pub fn len(&self) -> usize {
        self.skills.len()
    }

    /// 是否没有注册任何技能
    pub fn is_empty(&self) -> bool {
        self.skills.is_empty()
    }

    /// 获取所有技能的名称列表
    pub fn skill_names(&self) -> Vec<String> {
        self.skills.iter().map(|s| s.name().to_string()).collect()
    }
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 内置技能示例
// ============================================================================

/// 基础技能（示例）
///
/// 提供日志钩子
pub struct BaseSkill;

impl Skill for BaseSkill {
    fn name(&self) -> &str {
        "base"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn context_sources(&self) -> Vec<Arc<dyn ContextSource>> {
        Vec::new()
    }

    fn hooks(&self) -> Vec<Arc<dyn Hook>> {
        Vec::new()
    }
}
