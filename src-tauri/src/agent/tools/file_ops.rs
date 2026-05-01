//! 文件操作工具 — rig ToolDyn 实现
//!
//! read_file / write_file / edit_file

use super::path_guard::PathGuard;
use rig::tool::ToolDyn;
use serde_json::{json, Value};
use std::pin::Pin;
use std::sync::Arc;

const MAX_FILE_SIZE: usize = 10 * 1024 * 1024;
const MAX_OUTPUT: usize = 1_048_576;

fn truncate_output(content: &str) -> String {
    if content.len() <= MAX_OUTPUT {
        return content.to_string();
    }
    let mut s = String::new();
    let mut len = 0;
    for ch in content.chars() {
        if len + ch.len_utf8() > MAX_OUTPUT {
            s.push_str("\n... [truncated]");
            break;
        }
        s.push(ch);
        len += ch.len_utf8();
    }
    s
}

// ── read_file ────────────────────────────────────────────

pub struct FileReadTool {
    guard: Arc<PathGuard>,
}

impl FileReadTool {
    pub fn with_guard(guard: Arc<PathGuard>) -> Self {
        Self { guard }
    }
}

impl ToolDyn for FileReadTool {
    fn name(&self) -> String {
        "read_file".into()
    }
    fn definition(
        &self,
        _: String,
    ) -> Pin<Box<dyn std::future::Future<Output = rig::completion::ToolDefinition> + Send + '_>>
    {
        let p = json!({"type":"object","properties":{"path":{"type":"string","description":"File path relative to vault"},"offset":{"type":"integer","description":"Line offset (1-indexed, default 1)"},"limit":{"type":"integer","description":"Max lines to read"}},"required":["path"]});
        Box::pin(async move {
            rig::completion::ToolDefinition {
                name: "read_file".into(),
                description: "Read a file from the vault".into(),
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
            let path = p["path"].as_str().unwrap_or("");
            let offset = p["offset"].as_u64().unwrap_or(1) as usize;
            let limit = p["limit"].as_u64().map(|l| l as usize);
            let full = self.guard.resolve(path).map_err(|e| {
                rig::tool::ToolError::ToolCallError(Box::new(std::io::Error::other(e.to_string())))
            })?;
            if !full.exists() {
                return Ok(format!("File not found: {path}"));
            }
            if full.is_dir() {
                return Ok(format!("'{path}' is a directory"));
            }
            let meta = std::fs::metadata(&full)
                .map_err(|e| rig::tool::ToolError::ToolCallError(Box::new(e)))?;
            if meta.len() > MAX_FILE_SIZE as u64 {
                return Ok(format!("File too large: {} bytes", meta.len()));
            }
            let content = tokio::fs::read_to_string(&full)
                .await
                .map_err(|e| rig::tool::ToolError::ToolCallError(Box::new(e)))?;
            let lines: Vec<&str> = content.lines().collect();
            let start = offset.saturating_sub(1).min(lines.len());
            let end = limit
                .map(|l| (start + l).min(lines.len()))
                .unwrap_or(lines.len());
            Ok(truncate_output(&lines[start..end].join("\n")))
        })
    }
}

// ── write_file ───────────────────────────────────────────

pub struct FileWriteTool {
    guard: Arc<PathGuard>,
}

impl FileWriteTool {
    pub fn with_guard(guard: Arc<PathGuard>) -> Self {
        Self { guard }
    }
}

impl ToolDyn for FileWriteTool {
    fn name(&self) -> String {
        "write_file".into()
    }
    fn definition(
        &self,
        _: String,
    ) -> Pin<Box<dyn std::future::Future<Output = rig::completion::ToolDefinition> + Send + '_>>
    {
        let p = json!({"type":"object","properties":{"path":{"type":"string","description":"File path relative to vault"},"content":{"type":"string","description":"Content to write"}},"required":["path","content"]});
        Box::pin(async move {
            rig::completion::ToolDefinition {
                name: "write_file".into(),
                description: "Write content to a vault file (creates or overwrites)".into(),
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
            let path = p["path"].as_str().unwrap_or("");
            let content = p["content"].as_str().unwrap_or("");
            let full = self.guard.resolve(path).map_err(|e| {
                rig::tool::ToolError::ToolCallError(Box::new(std::io::Error::other(e.to_string())))
            })?;
            if let Some(parent) = full.parent() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(|e| rig::tool::ToolError::ToolCallError(Box::new(e)))?;
            }
            tokio::fs::write(&full, content)
                .await
                .map_err(|e| rig::tool::ToolError::ToolCallError(Box::new(e)))?;
            Ok(format!("Wrote {} bytes to {path}", content.len()))
        })
    }
}

// ── edit_file ────────────────────────────────────────────

pub struct FileEditTool {
    guard: Arc<PathGuard>,
}

impl FileEditTool {
    pub fn with_guard(guard: Arc<PathGuard>) -> Self {
        Self { guard }
    }
}

impl ToolDyn for FileEditTool {
    fn name(&self) -> String {
        "edit_file".into()
    }
    fn definition(
        &self,
        _: String,
    ) -> Pin<Box<dyn std::future::Future<Output = rig::completion::ToolDefinition> + Send + '_>>
    {
        let p = json!({"type":"object","properties":{"path":{"type":"string","description":"File path relative to vault"},"oldText":{"type":"string","description":"Exact text to find and replace"},"newText":{"type":"string","description":"Replacement text"}},"required":["path","oldText","newText"]});
        Box::pin(async move {
            rig::completion::ToolDefinition {
                name: "edit_file".into(),
                description: "Edit a vault file by exact text replacement".into(),
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
            let path = p["path"].as_str().unwrap_or("");
            let old_text = p["oldText"].as_str().unwrap_or("");
            let new_text = p["newText"].as_str().unwrap_or("");
            if old_text.is_empty() {
                return Ok("Error: oldText is empty".to_string());
            }
            let full = self.guard.resolve(path).map_err(|e| {
                rig::tool::ToolError::ToolCallError(Box::new(std::io::Error::other(e.to_string())))
            })?;
            let content = tokio::fs::read_to_string(&full)
                .await
                .map_err(|e| rig::tool::ToolError::ToolCallError(Box::new(e)))?;
            match content.find(old_text) {
                None => Ok(format!("Error: oldText not found in {path}")),
                Some(_) => {
                    let new_content = content.replacen(old_text, new_text, 1);
                    tokio::fs::write(&full, &new_content)
                        .await
                        .map_err(|e| rig::tool::ToolError::ToolCallError(Box::new(e)))?;
                    Ok(format!("Applied edit to {path}"))
                }
            }
        })
    }
}
