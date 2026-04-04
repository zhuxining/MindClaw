use super::path_guard::PathGuard;
use super::traits::{Tool, ToolInput, ToolOutput};
use crate::error::{AppError, AppResult};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;

/// 编辑文件内容（内容匹配替换）
///
/// 使用 old_string → new_string 精确替换。
/// old_string 必须在文件中唯一出现（0 次=未找到，多次=歧义）。
/// new_string 可为空（删除匹配内容）。
pub struct FileEditTool {
    guard: Arc<PathGuard>,
}

impl FileEditTool {
    pub fn new(vault_root: impl Into<PathBuf>) -> Self {
        Self {
            guard: Arc::new(PathGuard::vault_only(vault_root).with_denied("private")),
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

        // 检查 old_string 出现次数
        let matches: Vec<_> = content.match_indices(old_string).collect();

        if matches.is_empty() {
            return Err(AppError::NotFound(format!(
                "old_string not found in file: '{}...'",
                old_string.chars().take(50).collect::<String>()
            )));
        }

        if matches.len() > 1 {
            // 找出所有匹配位置用于错误信息
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

        // 执行替换
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
        "file_edit"
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

    fn setup_test_vault() -> (FileEditTool, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let vault = temp_dir.path().join("vault");
        std::fs::create_dir_all(&vault).unwrap();
        let tool = FileEditTool::new(&vault);
        (tool, temp_dir)
    }

    #[tokio::test]
    async fn test_edit_simple_replace() {
        let (tool, _temp) = setup_test_vault();

        // 创建测试文件
        let test_file = _temp.path().join("vault/test.txt");
        tokio::fs::write(&test_file, "hello world\nfoo bar\n")
            .await
            .unwrap();

        tool.edit_file("test.txt", "hello", "hi").await.unwrap();

        let content = tokio::fs::read_to_string(&test_file).await.unwrap();
        assert_eq!(content, "hi world\nfoo bar\n");
    }

    #[tokio::test]
    async fn test_edit_delete() {
        let (tool, _temp) = setup_test_vault();

        let test_file = _temp.path().join("vault/test.txt");
        tokio::fs::write(&test_file, "hello world\nfoo bar\n")
            .await
            .unwrap();

        tool.edit_file("test.txt", " world", "").await.unwrap();

        let content = tokio::fs::read_to_string(&test_file).await.unwrap();
        assert_eq!(content, "hello\nfoo bar\n");
    }

    #[tokio::test]
    async fn test_edit_multiline() {
        let (tool, _temp) = setup_test_vault();

        let test_file = _temp.path().join("vault/test.txt");
        tokio::fs::write(&test_file, "fn foo() {\n    println!(\"hello\");\n}\n")
            .await
            .unwrap();

        tool.edit_file("test.txt", "fn foo() {", "fn bar() {")
            .await
            .unwrap();

        let content = tokio::fs::read_to_string(&test_file).await.unwrap();
        assert_eq!(content, "fn bar() {\n    println!(\"hello\");\n}\n");
    }

    #[tokio::test]
    async fn test_edit_old_string_not_found() {
        let (tool, _temp) = setup_test_vault();

        let test_file = _temp.path().join("vault/test.txt");
        tokio::fs::write(&test_file, "hello world\n").await.unwrap();

        let result = tool.edit_file("test.txt", "notfound", "replacement").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_edit_ambiguous_match() {
        let (tool, _temp) = setup_test_vault();

        let test_file = _temp.path().join("vault/test.txt");
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
    async fn test_edit_empty_old_string() {
        let (tool, _temp) = setup_test_vault();

        let test_file = _temp.path().join("vault/test.txt");
        tokio::fs::write(&test_file, "content\n").await.unwrap();

        let result = tool.edit_file("test.txt", "", "replacement").await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("must not be empty"));
    }

    #[tokio::test]
    async fn test_path_escape_blocked() {
        let (tool, _temp) = setup_test_vault();
        let result = tool.edit_file("../escape.txt", "old", "new").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_private_path_blocked() {
        let (tool, _temp) = setup_test_vault();

        let private_dir = _temp.path().join("vault/private");
        std::fs::create_dir_all(&private_dir).unwrap();
        tokio::fs::write(private_dir.join("secret.txt"), "secret\n")
            .await
            .unwrap();

        let result = tool.edit_file("private/secret.txt", "old", "new").await;
        assert!(result.is_err());
    }
}
