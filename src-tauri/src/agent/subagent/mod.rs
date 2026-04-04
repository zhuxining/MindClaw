//! SubAgent 系统
//!
//! 后台任务派生：将复杂、耗时任务委托给独立执行实例

mod manager;
mod types;

pub use manager::SubAgentManager;
pub use types::{SubAgentDef, SubAgentInfo, SubAgentMode, SubAgentResult};
