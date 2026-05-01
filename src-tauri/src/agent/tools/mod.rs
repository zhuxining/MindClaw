//! Tools — 基于 rig ToolDyn 的工具注册
//!
//! 所有工具直接实现 rig::tool::ToolDyn。

pub mod agent_spawn;
pub mod file_ops;
pub mod find_files;
pub mod mcp;
pub mod path_guard;
pub mod search_content;
pub mod shell;

use rig::tool::ToolDyn;
use std::sync::Arc;

/// 构建工具列表（用于 AgentRunner）
///
/// 返回 `Vec<Box<dyn ToolDyn>>`，交给 rig Agent 构建本次 run 的 ToolServer。
/// 包含内置工具、MCP 工具和 spawn 工具，并按 profile tool policy 过滤。
pub async fn build_tools(
    config: &crate::runtime::config::AppConfig,
    spawn_tools: Vec<Box<dyn ToolDyn>>,
    allowed_tools: &[String],
) -> Result<Vec<Box<dyn ToolDyn>>, crate::error::AppError> {
    let guard = Arc::new(
        path_guard::PathGuard::vault_only(config.vault_path.clone()).with_denied("private"),
    );

    let mut tools: Vec<Box<dyn ToolDyn>> = vec![
        Box::new(shell::ShellTool::new(config.data_dir().clone())),
        Box::new(file_ops::FileReadTool::with_guard(Arc::clone(&guard))),
        Box::new(file_ops::FileWriteTool::with_guard(Arc::clone(&guard))),
        Box::new(file_ops::FileEditTool::with_guard(Arc::clone(&guard))),
        Box::new(find_files::FindFilesTool::new(Arc::clone(&guard))),
        Box::new(search_content::SearchContentTool::new(Arc::clone(&guard))),
    ];

    // MCP 工具
    let mcp_mgr = mcp::MCPManager::from_file(config.data_dir());
    if mcp_mgr.server_count() > 0 {
        mcp_mgr.ensure_connected().await?;
        for tool in mcp_mgr.get_tools().await {
            tools.push(tool);
        }
    }

    // Spawn 工具
    for tool in spawn_tools {
        tools.push(tool);
    }

    if !allowed_tools.is_empty() {
        tools.retain(|tool| allowed_tools.iter().any(|allowed| allowed == &tool.name()));
    }

    tracing::info!(tool_count = %tools.len(), "tools_initialized");
    Ok(tools)
}
