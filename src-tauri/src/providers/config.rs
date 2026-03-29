//! 模型配置、API endpoint、token 限制

pub const CLAUDE_API_BASE: &str = "https://api.anthropic.com";
pub const CLAUDE_API_VERSION: &str = "2023-06-01";

pub const MODEL_HAIKU: &str = "claude-haiku-4-5-20251001";
pub const MODEL_SONNET: &str = "claude-sonnet-4-6";

pub const MAX_TOKENS_HAIKU: u32 = 8_192;
pub const MAX_TOKENS_SONNET: u32 = 64_000;

/// 上下文窗口裁剪阈值（tokens）
pub const CONTEXT_TRIM_THRESHOLD: u32 = 180_000;
