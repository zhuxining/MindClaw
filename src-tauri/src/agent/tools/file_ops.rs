use super::path_guard::PathGuard;
use super::traits::{Tool, ToolInput, ToolOutput};
use crate::error::{AppError, AppResult};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;

const MAX_FILE_SIZE_BYTES: usize = 10 * 1024 * 1024; // 10MB
const MAX_OUTPUT_BYTES: usize = 1_048_576; // 1MB

fn default_guard(vault_root: impl Into<PathBuf>) -> Arc<PathGuard> {
    Arc::new(PathGuard::vault_only(vault_root).with_denied("private"))
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

/// 读取文件内容（支持偏移和限制）
pub struct FileReadTool {
    guard: Arc<PathGuard>,
}

impl FileReadTool {
    pub fn new(vault_root: impl Into<PathBuf>) -> Self {
        Self {
            guard: default_guard(vault_root),
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

        let metadata = std::fs::metadata(&full_path)?;
        if metadata.len() > MAX_FILE_SIZE_BYTES as u64 {
            return Err(AppError::Validation(format!(
                "File too large: {} bytes (max: {})",
                metadata.len(),
                MAX_FILE_SIZE_BYTES
            )));
        }

        let content = tokio::fs::read_to_string(&full_path).await?;
        let lines: Vec<&str> = content.lines().collect();
        let start = offset.saturating_sub(1).min(lines.len());
        let end = match limit {
            Some(l) => (start + l).min(lines.len()),
            None => lines.len(),
        };

        Ok(truncate_output(
            &lines[start..end].join("\n"),
            MAX_OUTPUT_BYTES,
        ))
    }
}

#[async_trait::async_trait]
impl Tool for FileReadTool {
    fn name(&self) -> &str {
        "read_file"
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

/// 写入文件内容（覆盖模式）
pub struct FileWriteTool {
    guard: Arc<PathGuard>,
}

impl FileWriteTool {
    pub fn new(vault_root: impl Into<PathBuf>) -> Self {
        Self {
            guard: default_guard(vault_root),
        }
    }

    pub fn with_guard(guard: Arc<PathGuard>) -> Self {
        Self { guard }
    }

    async fn write_file(&self, path: &str, content: &str) -> Result<(), AppError> {
        let full_path = self.guard.resolve(path)?;

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
        "write_file"
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

/// 编辑文件内容（内容匹配替换）
pub struct FileEditTool {
    guard: Arc<PathGuard>,
}

impl FileEditTool {
    pub fn new(vault_root: impl Into<PathBuf>) -> Self {
        Self {
            guard: default_guard(vault_root),
        }
    }

    pub fn with_guard(guard: Arc<PathGuard>) -> Self {
        Self { guard }
    }

    async fn edit_file(
        &self,
        path: &str,
        old_string: &str,
        new_string: &str,
    ) -> Result<String, AppError> {
        if old_string.is_empty() {
            return Err(AppError::Validation("old_string must not be empty".into()));
        }

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

        let content = tokio::fs::read_to_string(&full_path).await?;
        let matches: Vec<_> = content.match_indices(old_string).collect();

        if matches.is_empty() {
            return Err(AppError::NotFound(format!(
                "old_string not found in file: '{}...'",
                old_string.chars().take(50).collect::<String>()
            )));
        }

        if matches.len() > 1 {
            let locations: Vec<String> = matches
                .iter()
                .map(|(pos, _)| format!("offset {}", pos))
                .collect();
            return Err(AppError::Validation(format!(
                "old_string appears {} times in file (ambiguous). Locations: {}",
                matches.len(),
                locations.join(", ")
            )));
        }

        let new_content = content.replacen(old_string, new_string, 1);
        tokio::fs::write(&full_path, new_content).await?;

        Ok(format!(
            "Successfully edited '{}': replaced {} characters with {} characters",
            path,
            old_string.len(),
            new_string.len()
        ))
    }
}

#[async_trait::async_trait]
impl Tool for FileEditTool {
    fn name(&self) -> &str {
        "edit_file"
    }

    fn description(&self) -> &str {
        "Edit a file by replacing an exact string match with new content. \" \
         The old_string must appear exactly once in the file (0 matches = not found, \" \
         multiple matches = ambiguous). Cannot access private/ paths."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file (relative to vault root)"
                },
                "old_string": {
                    "type": "string",
                    "description": "The exact text to find and replace (must appear exactly once)"
                },
                "new_string": {
                    "type": "string",
                    "description": "The replacement text (empty string to delete the matched text)"
                }
            },
            "required": ["path", "old_string", "new_string"]
        })
    }

    async fn execute(&self, input: ToolInput) -> AppResult<ToolOutput> {
        let path = input
            .parameters
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::Validation("missing 'path' parameter".into()))?;

        let old_string = input
            .parameters
            .get("old_string")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::Validation("missing 'old_string' parameter".into()))?;

        let new_string = input
            .parameters
            .get("new_string")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::Validation("missing 'new_string' parameter".into()))?;

        let result = self.edit_file(path, old_string, new_string).await?;
        Ok(ToolOutput::ok(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_test_vault() -> (Arc<PathGuard>, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let vault = temp_dir.path().join("vault");
        std::fs::create_dir_all(&vault).unwrap();
        (default_guard(&vault), temp_dir)
    }

    #[tokio::test]
    async fn test_read_simple() {
        let (guard, temp) = setup_test_vault();
        let tool = FileReadTool::with_guard(guard);
        let test_file = temp.path().join("vault/test.txt");
        tokio::fs::write(&test_file, "line1\nline2\nline3\n")
            .await
            .unwrap();
        let content = tool.read_file("test.txt", 1, None).await.unwrap();
        assert_eq!(content, "line1\nline2\nline3");
    }

    #[tokio::test]
    async fn test_read_with_offset_and_limit() {
        let (guard, temp) = setup_test_vault();
        let tool = FileReadTool::with_guard(guard);
        let test_file = temp.path().join("vault/test.txt");
        tokio::fs::write(&test_file, "line1\nline2\nline3\nline4\nline5\n")
            .await
            .unwrap();
        let content = tool.read_file("test.txt", 2, Some(2)).await.unwrap();
        assert_eq!(content, "line2\nline3");
    }

    #[tokio::test]
    async fn test_write_new_file() {
        let (guard, temp) = setup_test_vault();
        let tool = FileWriteTool::with_guard(guard);
        tool.write_file("test.txt", "hello world").await.unwrap();
        let content = tokio::fs::read_to_string(temp.path().join("vault/test.txt"))
            .await
            .unwrap();
        assert_eq!(content, "hello world");
    }

    #[tokio::test]
    async fn test_write_creates_parent_dirs() {
        let (guard, temp) = setup_test_vault();
        let tool = FileWriteTool::with_guard(guard);
        tool.write_file("deep/nested/path/file.txt", "content")
            .await
            .unwrap();
        let content =
            tokio::fs::read_to_string(temp.path().join("vault/deep/nested/path/file.txt"))
                .await
                .unwrap();
        assert_eq!(content, "content");
    }

    #[tokio::test]
    async fn test_edit_simple_replace() {
        let (guard, temp) = setup_test_vault();
        let tool = FileEditTool::with_guard(guard);
        let test_file = temp.path().join("vault/test.txt");
        tokio::fs::write(&test_file, "hello world\nfoo bar\n")
            .await
            .unwrap();
        tool.edit_file("test.txt", "hello", "hi").await.unwrap();
        let content = tokio::fs::read_to_string(&test_file).await.unwrap();
        assert_eq!(content, "hi world\nfoo bar\n");
    }

    #[tokio::test]
    async fn test_edit_ambiguous_match() {
        let (guard, temp) = setup_test_vault();
        let tool = FileEditTool::with_guard(guard);
        let test_file = temp.path().join("vault/test.txt");
        tokio::fs::write(&test_file, "hello hello world\n")
            .await
            .unwrap();
        let result = tool.edit_file("test.txt", "hello", "hi").await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("ambiguous"));
        assert!(err.contains("2 times"));
    }

    #[tokio::test]
    async fn test_private_path_blocked_for_all_file_ops() {
        let (guard, temp) = setup_test_vault();
        let read_tool = FileReadTool::with_guard(Arc::clone(&guard));
        let write_tool = FileWriteTool::with_guard(Arc::clone(&guard));
        let edit_tool = FileEditTool::with_guard(guard);

        let private_dir = temp.path().join("vault/private");
        std::fs::create_dir_all(&private_dir).unwrap();
        tokio::fs::write(private_dir.join("secret.txt"), "secret\n")
            .await
            .unwrap();

        assert!(read_tool
            .read_file("private/secret.txt", 1, None)
            .await
            .is_err());
        assert!(write_tool
            .write_file("private/secret.txt", "content")
            .await
            .is_err());
        assert!(edit_tool
            .edit_file("private/secret.txt", "secret", "public")
            .await
            .is_err());
    }
}
