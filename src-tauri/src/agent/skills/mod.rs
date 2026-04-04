//! Skills 系统
//!
//! 基于 [Agent Skills](https://agentskills.io) 规范的能力扩展机制
//! 渐进式披露：元数据启动加载 → 指令激活加载 → 资源按需加载

mod registry;

pub use registry::{SkillManifest, SkillMetadata, SkillsRegistry};
