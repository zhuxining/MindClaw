//! SkillRegistry：技能系统
//!
//! 技能是 Agent 能力的组合包，可以注册：
//! - Tools（工具）
//! - ContextSources（上下文源）
//! - Hooks（钩子）

use crate::agent::context::{ContextBuildContext, ContextFragment, ContextSource, MessageRole};
use crate::agent::hooks::Hook;
use crate::agent::tools::{traits::Tool, ToolRegistry};
use crate::error::AppError;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

/// 技能文档（从 markdown 文件解析而来）
#[derive(Debug, Clone)]
pub struct SkillDoc {
    pub name: String,
    pub description: String,
    /// frontmatter 之后的完整 markdown 内容
    pub content: String,
}

impl SkillDoc {
    /// 解析带有 YAML frontmatter 的 markdown 字符串
    ///
    /// 期望格式：
    /// ```text
    /// ---
    /// name: foo
    /// description: "..."
    /// ---
    ///
    /// # Content...
    /// ```
    pub fn parse(markdown: &str) -> Self {
        let parts: Vec<&str> = markdown.splitn(3, "---").collect();

        let (frontmatter, content) = if parts.len() >= 3 {
            (parts[1], parts[2].trim_start_matches('\n'))
        } else {
            ("", markdown)
        };

        let mut name = String::new();
        let mut description = String::new();

        for line in frontmatter.lines() {
            if let Some(rest) = line.strip_prefix("name:") {
                name = rest.trim().trim_matches('"').to_string();
            } else if let Some(rest) = line.strip_prefix("description:") {
                let val = rest.trim().trim_matches('"');
                if !val.is_empty() {
                    description = val.to_string();
                }
            }
        }

        Self {
            name,
            description,
            content: content.to_string(),
        }
    }
}

/// 技能 trait：组合注册多个组件
///
/// 一个 Skill 可以包含多个组件，统一注册到各自的 Registry 中
pub trait Skill: Send + Sync {
    /// 技能名称
    fn name(&self) -> &str;
    /// 版本号
    fn version(&self) -> &str;
    /// 获取技能提供的工具（默认空，向后兼容）
    fn tools(&self) -> Vec<Arc<dyn Tool + Send + Sync>> {
        Vec::new()
    }
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

    /// 注册所有内置标准技能
    pub fn register_standard_skills(&mut self) {
        self.register(Box::new(StandardSkillsBundle::load()));
    }

    /// 将所有技能提供的工具注入 ToolRegistry
    pub fn inject_tools_to(&self, tool_registry: &mut ToolRegistry) {
        for skill in &self.skills {
            for tool in skill.tools() {
                tracing::info!(
                    skill_name = %skill.name(),
                    tool_name = %tool.name(),
                    "skill_tool_injected"
                );
                tool_registry.register(tool);
            }
        }
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

// ============================================================================
// 标准技能包：从内嵌 markdown 文档加载
// ============================================================================

const NOTES_SKILL_MD: &str = include_str!("../../../skills/notes.md");
const TASKS_SKILL_MD: &str = include_str!("../../../skills/tasks.md");
const DAILY_SKILL_MD: &str = include_str!("../../../skills/daily.md");

/// 标准技能包：加载内嵌的 markdown 技能文档
///
/// 提供：
/// - `SkillsOverviewSource`：向 system prompt 注入可用技能概要与内置技能说明
pub struct StandardSkillsBundle {
    skills: Arc<HashMap<String, SkillDoc>>,
}

impl StandardSkillsBundle {
    /// 解析内嵌的技能文档并构建技能包
    pub fn load() -> Self {
        let docs = [NOTES_SKILL_MD, TASKS_SKILL_MD, DAILY_SKILL_MD]
            .iter()
            .map(|md| SkillDoc::parse(md))
            .filter(|doc| !doc.name.is_empty())
            .map(|doc| (doc.name.clone(), doc))
            .collect::<HashMap<_, _>>();

        tracing::info!(count = docs.len(), "standard_skills_loaded");
        Self {
            skills: Arc::new(docs),
        }
    }
}

impl Skill for StandardSkillsBundle {
    fn name(&self) -> &str {
        "standard-skills"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn context_sources(&self) -> Vec<Arc<dyn ContextSource>> {
        vec![Arc::new(SkillsOverviewSource {
            skills: Arc::clone(&self.skills),
        })]
    }

    fn hooks(&self) -> Vec<Arc<dyn Hook>> {
        Vec::new()
    }
}

/// 上下文源：向 system prompt 注入可用技能概要列表
struct SkillsOverviewSource {
    skills: Arc<HashMap<String, SkillDoc>>,
}

#[async_trait]
impl ContextSource for SkillsOverviewSource {
    fn name(&self) -> &str {
        "skills_overview"
    }

    fn priority(&self) -> i32 {
        10
    }

    async fn inject(
        &self,
        _ctx: &ContextBuildContext,
        budget: usize,
    ) -> Result<Vec<ContextFragment>, AppError> {
        if self.skills.is_empty() {
            return Ok(Vec::new());
        }

        let mut lines = vec![
            "Available built-in skills:".to_string(),
            "Skills are prompt-side guidance, not callable tools.".to_string(),
            "Use the documented `operations` actions directly when a skill applies.".to_string(),
            String::new(),
        ];

        let mut names: Vec<&str> = self.skills.keys().map(String::as_str).collect();
        names.sort_unstable();
        for name in names.iter().copied() {
            if let Some(doc) = self.skills.get(name) {
                lines.push(format!("- {}: {}", doc.name, doc.description));
            }
        }

        lines.push(String::new());
        lines.push("Built-in skill instructions:".to_string());

        for name in names.iter().copied() {
            if let Some(doc) = self.skills.get(name) {
                lines.push(String::new());
                lines.push(format!("<skill name=\"{}\">", doc.name));
                lines.push(doc.content.trim().to_string());
                lines.push("</skill>".to_string());
            }
        }

        let content = lines.join("\n");
        let estimated_tokens = content.len() / 4;
        let final_content = if estimated_tokens > budget {
            let mut compact_lines = vec![
                "Available built-in skills:".to_string(),
                "Skills are prompt-side guidance, not callable tools.".to_string(),
            ];

            for name in names.iter().copied() {
                if let Some(doc) = self.skills.get(name) {
                    compact_lines.push(format!("- {}: {}", doc.name, doc.description));
                }
            }

            compact_lines.join("\n")
        } else {
            content
        };

        let token_estimate = final_content.len() / 4;

        Ok(vec![ContextFragment::new(
            MessageRole::System,
            final_content,
            token_estimate,
            "skills_overview",
        )])
    }
}
