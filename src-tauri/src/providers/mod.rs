//! Provider layer - rig client factory
//!
//! 完全使用 rig 内置 providers，通过宏定义供应商。
//!
//! Providers 只负责配置、密钥、client 和 completion model 创建。
//! Agent 多轮循环、工具执行、streaming 和消息转换由 AgentRunner 通过 rig Agent 负责。

pub mod config;
pub mod macros;
pub mod registry;

pub use config::ProviderConfig;
pub use registry::{AgentModelSet, LLMClient, LLMCompletionModel, ProviderRegistry};
