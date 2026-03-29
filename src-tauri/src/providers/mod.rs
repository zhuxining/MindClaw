pub mod claude;
pub mod config;
pub mod openai_compat;
pub mod registry;
pub mod traits;

#[cfg(test)]
mod tests;

pub use config::{ModelConfig, ProviderConfig};
pub use registry::ProviderRegistry;
pub use traits::{
    ChatMessage, ChatRequest, MessageContent, MessageRole, ModelTier, Provider, ProviderResponse,
    ToolChoice, ToolSchema,
};
