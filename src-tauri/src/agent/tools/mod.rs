//! Tools — 基于 rig ToolDyn 的工具注册
//!
//! 所有工具直接实现 rig::tool::ToolDyn。

pub mod file_ops;
pub mod find_files;
pub mod mcp;
pub mod path_guard;
pub mod search_content;
pub mod shell;
pub mod spawn;

use crate::error::AppError;
use rig::tool::ToolDyn;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// 工具构建范围
pub enum ToolScope {
    /// 主 Agent 完整工具集（含 MCP、spawn）
    Main {
        config: Arc<crate::runtime::config::AppConfig>,
        spawn_tools: Vec<Box<dyn ToolDyn>>,
        allowed_tools: Vec<String>,
    },
    /// SubAgent 受限工具集（shell + 文件操作，vault only，排除 private）
    Subagent { workspace: PathBuf },
}

/// 统一工具构建入口
pub async fn build_tools(scope: ToolScope) -> Result<Vec<Box<dyn ToolDyn>>, AppError> {
    match scope {
        ToolScope::Main {
            config,
            spawn_tools,
            allowed_tools,
        } => {
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

            let mcp_mgr = mcp::MCPManager::from_file(config.data_dir());
            if mcp_mgr.server_count() > 0 {
                mcp_mgr.ensure_connected().await?;
                for tool in mcp_mgr.get_tools().await {
                    tools.push(tool);
                }
            }

            for tool in spawn_tools {
                tools.push(tool);
            }

            if !allowed_tools.is_empty() {
                tools.retain(|tool| allowed_tools.iter().any(|a| a == &tool.name()));
            }

            tracing::info!(tool_count = %tools.len(), "tools_initialized");
            Ok(tools)
        }
        ToolScope::Subagent { workspace } => Ok(build_subagent_tools_inner(&workspace)),
    }
}

fn build_subagent_tools_inner(workspace: &Path) -> Vec<Box<dyn ToolDyn>> {
    let guard =
        Arc::new(path_guard::PathGuard::vault_only(workspace.to_path_buf()).with_denied("private"));
    vec![
        Box::new(shell::ShellTool::new(workspace.to_path_buf())),
        Box::new(file_ops::FileReadTool::with_guard(Arc::clone(&guard))),
        Box::new(file_ops::FileWriteTool::with_guard(Arc::clone(&guard))),
        Box::new(file_ops::FileEditTool::with_guard(Arc::clone(&guard))),
        Box::new(find_files::FindFilesTool::new(Arc::clone(&guard))),
        Box::new(search_content::SearchContentTool::new(Arc::clone(&guard))),
    ]
}
