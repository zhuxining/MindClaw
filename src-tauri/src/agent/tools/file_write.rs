use super::path_guard::PathGuard;
use super::traits::{Tool, ToolInput, ToolOutput};
use crate::error::{AppError, AppResult};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;

/// 写入文件内容（覆盖模式）
pub struct FileWriteTool {
    guard: Arc<PathGuard>,
}

impl FileWriteTool {
    pub fn new(vault_root: impl Into<PathBuf>) -> Self {
        Self {
            guard: Arc::new(PathGuard::vault_only(vault_root).with_denied("private")),
        }
    }

    pub fn with_guard(guard: Arc<PathGuard>) -> Self {
        Self { guard }
    }

    async fn write_file(&self, path: &str, content: &str) -> Result<(), AppError> {
        let full_path = self.guard.resolve(path)?;

        // 确保父目录存在
        if let Some(parent) = full_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        tokio::fs::write(&full_path, content).await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl Tool for FileWriteTool {
    fn name(&self) -> &str {
        "file_write"
    }

    fn description(&self) -> &str {
        "Write contents to a file in the vault. Creates parent directories if needed. \" \
         Cannot access private/ paths."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file (relative to vault root)"
                },
                "content": {
                    "type": "string",
                    "description": "Content to write to the file"
                }
            },
            "required": ["path", "content"]
        })
    }

    async fn execute(&self, input: ToolInput) -> AppResult<ToolOutput> {
        let path = input
            .parameters
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::Validation("missing 'path' parameter".into()))?;

        let content = input
            .parameters
            .get("content")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::Validation("missing 'content' parameter".into()))?;

        self.write_file(path, content).await?;
        Ok(ToolOutput::ok(format!("Successfully wrote to '{}'", path)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_test_vault() -> (FileWriteTool, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let vault = temp_dir.path().join("vault");
        std::fs::create_dir_all(&vault).unwrap();
        let tool = FileWriteTool::new(&vault);
        (tool, temp_dir)
    }

    #[tokio::test]
    async fn test_write_new_file() {
        let (tool, _temp) = setup_test_vault();
        
        tool.write_file("test.txt", "hello world").await.unwrap();
        
        let content = tokio::fs::read_to_string(_temp.path().join("vault/test.txt")).await.unwrap();
        assert_eq!(content, "hello world");
    }

    #[tokio::test]
    async fn test_write_overwrite() {
        let (tool, _temp) = setup_test_vault();
        
        tool.write_file("test.txt", "original").await.unwrap();
        tool.write_file("test.txt", "overwritten").await.unwrap();
        
        let content = tokio::fs::read_to_string(_temp.path().join("vault/test.txt")).await.unwrap();
        assert_eq!(content, "overwritten");
    }

    #[tokio::test]
    async fn test_write_creates_parent_dirs() {
        let (tool, _temp) = setup_test_vault();
        
        tool.write_file("deep/nested/path/file.txt", "content").await.unwrap();
        
        let content = tokio::fs::read_to_string(
            _temp.path().join("vault/deep/nested/path/file.txt")
        ).await.unwrap();
        assert_eq!(content, "content");
    }

    #[tokio::test]
    async fn test_path_escape_blocked() {
        let (tool, _temp) = setup_test_vault();
        let result = tool.write_file("../escape.txt", "content").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_private_path_blocked() {
        let (tool, _temp) = setup_test_vault();
        let result = tool.write_file("private/secret.txt", "content").await;
        assert!(result.is_err());
    }
}