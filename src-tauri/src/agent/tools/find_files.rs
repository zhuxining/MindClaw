//! 文件查找工具 — rig ToolDyn 实现

use super::path_guard::PathGuard;
use globset::{Glob, GlobSetBuilder};
use ignore::WalkBuilder;
use rig::tool::ToolDyn;
use serde_json::{json, Value};
use std::pin::Pin;
use std::sync::Arc;

pub struct FindFilesTool {
    guard: Arc<PathGuard>,
}

impl FindFilesTool {
    pub fn new(guard: Arc<PathGuard>) -> Self {
        Self { guard }
    }
}

impl ToolDyn for FindFilesTool {
    fn name(&self) -> String {
        "find_files".into()
    }
    fn definition(
        &self,
        _: String,
    ) -> Pin<Box<dyn std::future::Future<Output = rig::completion::ToolDefinition> + Send + '_>>
    {
        let p = json!({"type":"object","properties":{"pattern":{"type":"string","description":"Glob pattern (e.g. '*.md', 'src/**/*.rs'). Use '*' to list all."},"path":{"type":"string","description":"Subdirectory within vault to search (default: root)"}},"required":["pattern"]});
        Box::pin(async move {
            rig::completion::ToolDefinition {
                name: "find_files".into(),
                description: "Find files/directories matching a glob pattern within the vault"
                    .into(),
                parameters: p,
            }
        })
    }
    fn call(
        &self,
        args: String,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<String, rig::tool::ToolError>> + Send + '_>>
    {
        Box::pin(async move {
            let p: Value = serde_json::from_str(&args).map_err(rig::tool::ToolError::JsonError)?;
            let pattern = p["pattern"].as_str().unwrap_or("*");
            let subdir = p["path"].as_str().unwrap_or("");
            let root = if subdir.is_empty() {
                self.guard.primary().to_path_buf()
            } else {
                self.guard.primary().join(subdir)
            };
            let mut builder = GlobSetBuilder::new();
            builder.add(
                Glob::new(pattern).map_err(|e| rig::tool::ToolError::ToolCallError(Box::new(e)))?,
            );
            let set = builder
                .build()
                .map_err(|e| rig::tool::ToolError::ToolCallError(Box::new(e)))?;
            let mut results = Vec::new();
            for entry in WalkBuilder::new(&root)
                .max_depth(Some(20))
                .hidden(false)
                .build()
                .flatten()
            {
                let path = entry.path();
                let rel = path.strip_prefix(self.guard.primary()).unwrap_or(path);
                if set.is_match(rel) {
                    results.push(rel.display().to_string());
                }
                if results.len() >= 200 {
                    results.push("... (truncated at 200)".into());
                    break;
                }
            }
            if results.is_empty() {
                Ok("No files found".into())
            } else {
                Ok(results.join("\n"))
            }
        })
    }
}
