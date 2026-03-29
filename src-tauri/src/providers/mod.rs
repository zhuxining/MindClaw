pub mod claude;
pub mod config;
pub mod openai_compat;
pub mod registry;
pub mod traits;

#[cfg(test)]
mod tests;

pub use config::{ModelConfig, ProviderConfig};
pub use registry::ProviderRegistry;
pub use traits::{ChatMessage, ModelTier, Provider, ProviderResponse};
