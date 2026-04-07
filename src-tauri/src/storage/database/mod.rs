//! SQLite 数据库管理
//!
//! 双 DB 架构：
//! - 全局 DB（`~/.config/mindclaw/mindclaw.db`）：sessions/turns/messages/vaults
//! - Vault DB（`{vault}/.mindclaw/mindclaw.db`）：tasks/notes/memories 索引（可重建）

pub mod global;
pub mod vault;

pub use global::open as open_global;
pub use vault::open as open_vault;
