pub mod memory;

use crate::services::agent::ExecutionContext;

pub struct AgentContextBuilder;

impl AgentContextBuilder {
    pub fn build_prompt(context: &ExecutionContext, message_content: &str) -> String {
        let mut sections: Vec<String> = Vec::new();

        // ── System prompt (identity) ──────────────────────────
        let identity = &context.agent.identity;
        if !identity.system_prompt.is_empty() {
            sections.push(format!("[身份]\n{}", identity.system_prompt));
        }
        if let Some(style) = &identity.style {
            sections.push(format!("[风格]\n{style}"));
        }
        if let Some(safety_policy) = &identity.safety_policy {
            sections.push(format!("[安全策略]\n{safety_policy}"));
        }

        // ── Skill instruction ─────────────────────────────────
        if let Some(skill) = &context.skill {
            sections.push(format!(
                "[当前任务能力]\n能力名称：{}\n{}",
                skill.name, skill.instruction
            ));
        }

        // ── User message ──────────────────────────────────────
        sections.push(format!("[用户消息]\n{message_content}"));

        sections.join("\n\n")
    }
}
