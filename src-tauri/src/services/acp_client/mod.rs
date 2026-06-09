pub mod client;
pub mod protocol;

pub use client::AcpClient;
// Re-exports for future use by MessageBus/Commands
#[allow(unused_imports)]
pub use protocol::{AcpAgentConfig, AcpMessage, AcpParams, AcpRequest, AcpResponse, AcpResult};
