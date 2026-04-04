use super::path_guard::PathGuard;
use super::traits::{Tool, ToolInput, ToolOutput};
use crate::error::{AppError, AppResult};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;

const MAX_FILE_SIZE_BYTES: usize = 10 * 1024 * 1024; // 10MB
const MAX_OUTPUT_BYTES: usize = 1_048_576; // 1MB

/// 读取文件内容（支持偏移和限制）
pub struct FileReadTool {
    guard: Arc<PathGuard>,
}

impl FileReadTool {
    pub fn new(vault_root: impl Into<PathBuf>) -> Self {
        Self {
            guard: Arc::new(PathGuard::vault_only(vault_root).with_denied("private")),
        }
    }

    pub fn with_guard(guard: Arc<PathGuard>) -> Self {
        Self { guard }
    }

    async fn read_file(
        &self,
        path: &str,
        offset: usize,
        limit: Option<usize>,
    ) -> Result<String, AppError> {
        let full_path = self.guard.resolve(path)?;

        if !full_path.exists() {
            return Err(AppError::NotFound(format!("File not found: {}", path)));
        }

        if full_path.is_dir() {
            return Err(AppError::Validation(format!(
                "'{}' is a directory, not a file",
                path
            )));
        }

        // 检查文件大小
        let metadata = std::fs::metadata(&full_path)?;
        if metadata.len() > MAX_FILE_SIZE_BYTES as u64 {
            return Err(AppError::Validation(format!(
                "File too large: {} bytes (max: {})",
                metadata.len(),
                MAX_FILE_SIZE_BYTES
            )));
        }

        let content = tokio::fs::read_to_string(&full_path).await?;

        // 处理 offset/limit
        let lines: Vec<&str> = content.lines().collect();
        let start = offset.saturating_sub(1); // 1-based to 0-based
        let start = start.min(lines.len());

        let end = match limit {
            Some(l) => (start + l).min(lines.len()),
            None => lines.len(),
        };

        let result = lines[start..end].join("\n");
        Ok(truncate_output(&result, MAX_OUTPUT_BYTES))
    }
}

#[async_trait::async_trait]
impl Tool for FileReadTool {
    fn name(&self) -> &str {
        "file_read"
    }

    fn description(&self) -> &str {
        "Read file contents with line numbers. Supports partial reading via offset and limit. \" \
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
                "offset": {
                    "type": "integer",
                    "description": "Starting line number (1-based, default: 1)",
                    "minimum": 1
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of lines to return (default: all)",
                    "minimum": 1
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, input: ToolInput) -> AppResult<ToolOutput> {
        let path = input
            .parameters
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::Validation("missing 'path' parameter".into()))?;

        let offset = input
            .parameters
            .get("offset")
            .and_then(Value::as_u64)
            .map(|v| v as usize)
            .unwrap_or(1);

        let limit = input
            .parameters
            .get("limit")
            .and_then(Value::as_u64)
            .map(|v| v as usize);

        let content = self.read_file(path, offset, limit).await?;
        Ok(ToolOutput::ok(content))
    }
}

fn truncate_output(content: &str, max_bytes: usize) -> String {
    if content.len() <= max_bytes {
        return content.to_string();
    }

    let mut truncated = String::new();
    let mut current_len = 0;
    for ch in content.chars() {
        let ch_len = ch.len_utf8();
        if current_len + ch_len > max_bytes {
            truncated.push_str("\n... [truncated]");
            break;
        }
        truncated.push(ch);
        current_len += ch_len;
    }
    truncated
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_test_vault() -> (FileReadTool, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let vault = temp_dir.path().join("vault");
        std::fs::create_dir_all(&vault).unwrap();
        let tool = FileReadTool::new(&vault);
        (tool, temp_dir)
    }

    #[tokio::test]
    async fn test_read_simple() {
        let (tool, _temp) = setup_test_vault();

        // 创建测试文件
        let test_file = _temp.path().join("vault/test.txt");
        tokio::fs::write(&test_file, "line1\nline2\nline3\n")
            .await
            .unwrap();

        let content = tool.read_file("test.txt", 1, None).await.unwrap();
        assert_eq!(content, "line1\nline2\nline3");
    }

    #[tokio::test]
    async fn test_read_with_offset() {
        let (tool, _temp) = setup_test_vault();

        let test_file = _temp.path().join("vault/test.txt");
        tokio::fs::write(&test_file, "line1\nline2\nline3\nline4\n")
            .await
            .unwrap();

        let content = tool.read_file("test.txt", 2, None).await.unwrap();
        assert_eq!(content, "line2\nline3\nline4");
    }

    #[tokio::test]
    async fn test_read_with_limit() {
        let (tool, _temp) = setup_test_vault();

        let test_file = _temp.path().join("vault/test.txt");
        tokio::fs::write(&test_file, "line1\nline2\nline3\nline4\n")
            .await
            .unwrap();

        let content = tool.read_file("test.txt", 1, Some(2)).await.unwrap();
        assert_eq!(content, "line1\nline2");
    }

    #[tokio::test]
    async fn test_read_offset_and_limit() {
        let (tool, _temp) = setup_test_vault();

        let test_file = _temp.path().join("vault/test.txt");
        tokio::fs::write(&test_file, "line1\nline2\nline3\nline4\nline5\n")
            .await
            .unwrap();

        let content = tool.read_file("test.txt", 2, Some(2)).await.unwrap();
        assert_eq!(content, "line2\nline3");
    }

    #[tokio::test]
    async fn test_path_escape_blocked() {
        let (tool, _temp) = setup_test_vault();
        let result = tool.read_file("../escape.txt", 1, None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_private_path_blocked() {
        let (tool, _temp) = setup_test_vault();

        // 创建 private 目录和文件
        let private_dir = _temp.path().join("vault/private");
        std::fs::create_dir_all(&private_dir).unwrap();
        tokio::fs::write(private_dir.join("secret.txt"), "secret")
            .await
            .unwrap();

        let result = tool.read_file("private/secret.txt", 1, None).await;
        assert!(result.is_err());
    }
}
