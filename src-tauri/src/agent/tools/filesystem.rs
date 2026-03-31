use super::traits::{Tool, ToolInput, ToolOutput};
use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;

/// vault 内文件操作（安全边界约束：只允许访问 vault 目录，拒绝 private/ 路径）
pub struct FilesystemTool {
    vault_root: PathBuf,
}

/// 目录条目信息
#[derive(Debug, Serialize, Deserialize)]
pub struct DirEntry {
    pub name: String,
    pub file_type: String, // "file" | "directory"
    pub size: u64,
}

/// 编辑操作
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum EditOp {
    /// 在指定行后插入
    Insert { after_line: usize, content: String },
    /// 删除行范围
    Delete { from_line: usize, to_line: usize },
    /// 替换行范围
    Replace {
        from_line: usize,
        to_line: usize,
        content: String,
    },
    /// 在开头插入
    Prepend { content: String },
    /// 在末尾追加
    Append { content: String },
}

impl FilesystemTool {
    pub fn new(vault_root: impl Into<PathBuf>) -> Self {
        Self {
            vault_root: vault_root.into(),
        }
    }

    /// 解析并校验路径（核心安全方法）
    fn resolve_path(&self, path: &str) -> Result<PathBuf, AppError> {
        // 处理空路径（表示根目录）
        let full_path = if path.is_empty() {
            self.vault_root.clone()
        } else {
            self.vault_root.join(path)
        };

        // 规范化：清理 .. 组件，但保留根目录
        let mut components = Vec::new();
        let mut is_absolute = false;
        for comp in full_path.components() {
            match comp {
                std::path::Component::RootDir => {
                    is_absolute = true;
                }
                std::path::Component::Prefix(p) => {
                    components.push(std::path::Component::Prefix(p));
                }
                std::path::Component::ParentDir => {
                    components.pop();
                }
                std::path::Component::Normal(c) => {
                    components.push(std::path::Component::Normal(c));
                }
                _ => {}
            }
        }

        // 重新构建路径
        let mut normalized = PathBuf::new();
        if is_absolute {
            normalized.push("/");
        }
        for comp in &components {
            if let std::path::Component::Normal(c) = comp {
                normalized.push(c);
            }
        }

        // 检查是否在 vault 内：确保 normalized 以 vault_root 开头
        let vault_str = self.vault_root.to_string_lossy();
        let normalized_str = normalized.to_string_lossy();

        if !normalized_str.starts_with(vault_str.as_ref()) {
            return Err(AppError::PermissionDenied(format!(
                "Path '{}' is outside vault",
                path
            )));
        }

        // 检查是否包含 private/
        if !self.is_allowed(path) {
            return Err(AppError::PermissionDenied(format!(
                "Access to 'private/' is denied: {}",
                path
            )));
        }

        Ok(normalized)
    }

    /// 检查路径是否允许访问
    fn is_allowed(&self, path: &str) -> bool {
        !path.contains("private/") && !path.starts_with("private")
    }

    // ── 操作实现 ──────────────────────────────────────────────

    /// 读取文件内容
    async fn file_read(&self, path: &str) -> Result<String, AppError> {
        let full_path = self.resolve_path(path)?;

        if !full_path.exists() {
            return Err(AppError::NotFound(format!("File not found: {}", path)));
        }

        if full_path.is_dir() {
            return Err(AppError::Validation(format!(
                "'{}' is a directory, not a file",
                path
            )));
        }

        let content = fs::read_to_string(&full_path)?;
        Ok(truncate_content(&content, 1_048_576))
    }

    /// 写入文件（覆盖）
    async fn file_write(&self, path: &str, content: &str) -> Result<(), AppError> {
        let full_path = self.resolve_path(path)?;

        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&full_path, content)?;
        Ok(())
    }

    /// 列出目录内容
    async fn file_list(&self, path: &str) -> Result<Vec<DirEntry>, AppError> {
        let full_path = self.resolve_path(path)?;

        if !full_path.exists() {
            return Err(AppError::NotFound(format!("Directory not found: {}", path)));
        }

        if full_path.is_file() {
            return Err(AppError::Validation(format!(
                "'{}' is a file, not a directory",
                path
            )));
        }

        let mut entries = Vec::new();
        for entry in fs::read_dir(&full_path)? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            entries.push(DirEntry {
                name: entry.file_name().to_string_lossy().to_string(),
                file_type: if metadata.is_dir() {
                    "directory"
                } else {
                    "file"
                }
                .to_string(),
                size: metadata.len(),
            });
        }

        entries.truncate(1000);
        Ok(entries)
    }

    /// 编辑文件（局部修改）
    async fn file_edit(&self, path: &str, edits: Vec<EditOp>) -> Result<(), AppError> {
        let full_path = self.resolve_path(path)?;

        if !full_path.exists() {
            return Err(AppError::NotFound(format!("File not found: {}", path)));
        }

        if full_path.is_dir() {
            return Err(AppError::Validation(format!(
                "'{}' is a directory, not a file",
                path
            )));
        }

        // 读取文件内容
        let content = fs::read_to_string(&full_path)?;
        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

        // 按顺序应用编辑
        for edit in edits {
            match edit {
                EditOp::Prepend { content } => {
                    lines.insert(0, content);
                }
                EditOp::Append { content } => {
                    lines.push(content);
                }
                EditOp::Insert {
                    after_line,
                    content,
                } => {
                    let insert_pos = (after_line + 1).min(lines.len());
                    lines.insert(insert_pos, content);
                }
                EditOp::Delete { from_line, to_line } => {
                    if from_line <= to_line && to_line < lines.len() {
                        lines.drain(from_line..=to_line);
                    }
                }
                EditOp::Replace {
                    from_line,
                    to_line,
                    content,
                } => {
                    if from_line <= to_line && to_line < lines.len() {
                        lines.drain(from_line..=to_line);
                        // 将替换内容按行拆分后插入
                        for (i, line) in content.lines().enumerate() {
                            lines.insert(from_line + i, line.to_string());
                        }
                    }
                }
            }
        }

        // 写回文件
        let new_content = lines.join("\n");
        fs::write(&full_path, new_content)?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl Tool for FilesystemTool {
    fn name(&self) -> &str {
        "filesystem"
    }

    fn description(&self) -> &str {
        "Read and write files within the vault. Cannot access private/ paths."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["read", "write", "list", "edit"],
                    "description": "操作类型"
                },
                "path": {
                    "type": "string",
                    "description": "文件路径（相对于 vault 根目录）"
                },
                "content": {
                    "type": "string",
                    "description": "文件内容（write 操作需要）"
                },
                "edits": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "type": {
                                "type": "string",
                                "enum": ["insert", "delete", "replace", "prepend", "append"]
                            },
                            "after_line": { "type": "integer" },
                            "from_line": { "type": "integer" },
                            "to_line": { "type": "integer" },
                            "content": { "type": "string" }
                        },
                        "required": ["type"]
                    },
                    "description": "编辑操作列表（edit 操作需要）"
                }
            },
            "required": ["operation", "path"]
        })
    }

    async fn execute(&self, input: ToolInput) -> AppResult<ToolOutput> {
        let operation = input
            .parameters
            .get("operation")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::Validation("missing 'operation' parameter".into()))?;

        let path = input
            .parameters
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::Validation("missing 'path' parameter".into()))?;

        if !self.is_allowed(path) {
            return Ok(ToolOutput::err(format!(
                "Access denied: path '{}' contains 'private/'",
                path
            )));
        }

        match operation {
            "read" => self
                .file_read(path)
                .await
                .map(ToolOutput::ok)
                .map_err(|e| AppError::Storage(format!("read failed: {}", e))),

            "write" => {
                let content = input
                    .parameters
                    .get("content")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        AppError::Validation("missing 'content' parameter for write".into())
                    })?;

                self.file_write(path, content)
                    .await
                    .map(|_| ToolOutput::ok(format!("Successfully wrote to '{}'", path)))
                    .map_err(|e| AppError::Storage(format!("write failed: {}", e)))
            }

            "list" => self
                .file_list(path)
                .await
                .map(|entries| {
                    let json =
                        serde_json::to_string_pretty(&entries).unwrap_or_else(|_| "[]".to_string());
                    ToolOutput::ok(json)
                })
                .map_err(|e| AppError::Storage(format!("list failed: {}", e))),

            "edit" => {
                let edits_value = input.parameters.get("edits").ok_or_else(|| {
                    AppError::Validation("missing 'edits' parameter for edit".into())
                })?;

                let edits: Vec<EditOp> = serde_json::from_value(edits_value.clone())
                    .map_err(|e| AppError::Validation(format!("invalid edits format: {}", e)))?;

                if edits.is_empty() {
                    return Err(AppError::Validation("'edits' cannot be empty".into()));
                }

                self.file_edit(path, edits)
                    .await
                    .map(|_| ToolOutput::ok(format!("Successfully edited '{}'", path)))
                    .map_err(|e| AppError::Storage(format!("edit failed: {}", e)))
            }

            _ => Err(AppError::Validation(format!(
                "Unknown operation: {}",
                operation
            ))),
        }
    }
}

// ── 辅助函数 ──────────────────────────────────────────────

/// 截断超长内容
fn truncate_content(content: &str, max_bytes: usize) -> String {
    if content.len() <= max_bytes {
        return content.to_string();
    }

    let mut truncated = String::new();
    let mut current_len = 0;
    for ch in content.chars() {
        let ch_len = ch.len_utf8();
        if current_len + ch_len > max_bytes {
            truncated.push_str("... [truncated]");
            break;
        }
        truncated.push(ch);
        current_len += ch_len;
    }
    truncated
}

// ── 测试 ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_test_vault() -> (FilesystemTool, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let vault = temp_dir.path().join("vault");
        fs::create_dir_all(&vault).unwrap();
        let tool = FilesystemTool::new(vault.clone()); // 使用 clone 确保路径一致
        (tool, temp_dir)
    }

    #[tokio::test]
    async fn test_read_write() {
        let (tool, _temp) = setup_test_vault();
        tool.file_write("test.txt", "hello\nworld").await.unwrap();
        let content = tool.file_read("test.txt").await.unwrap();
        assert_eq!(content, "hello\nworld");
    }

    #[tokio::test]
    async fn test_edit_insert() {
        let (tool, _temp) = setup_test_vault();
        tool.file_write("test.txt", "line1\nline2\nline3")
            .await
            .unwrap();

        tool.file_edit(
            "test.txt",
            vec![EditOp::Insert {
                after_line: 0, // 在第 0 行（line1）后插入
                content: "inserted".to_string(),
            }],
        )
        .await
        .unwrap();

        let content = tool.file_read("test.txt").await.unwrap();
        assert_eq!(content, "line1\ninserted\nline2\nline3");
    }

    #[tokio::test]
    async fn test_edit_delete() {
        let (tool, _temp) = setup_test_vault();
        tool.file_write("test.txt", "line1\nline2\nline3")
            .await
            .unwrap();

        tool.file_edit(
            "test.txt",
            vec![EditOp::Delete {
                from_line: 1,
                to_line: 1,
            }],
        )
        .await
        .unwrap();

        let content = tool.file_read("test.txt").await.unwrap();
        assert_eq!(content, "line1\nline3");
    }

    #[tokio::test]
    async fn test_edit_replace() {
        let (tool, _temp) = setup_test_vault();
        tool.file_write("test.txt", "line1\nline2\nline3")
            .await
            .unwrap();

        tool.file_edit(
            "test.txt",
            vec![EditOp::Replace {
                from_line: 1,
                to_line: 1,
                content: "replaced".to_string(),
            }],
        )
        .await
        .unwrap();

        let content = tool.file_read("test.txt").await.unwrap();
        assert_eq!(content, "line1\nreplaced\nline3");
    }

    #[tokio::test]
    async fn test_edit_append() {
        let (tool, _temp) = setup_test_vault();
        tool.file_write("test.txt", "line1").await.unwrap();

        tool.file_edit(
            "test.txt",
            vec![EditOp::Append {
                content: "appended".to_string(),
            }],
        )
        .await
        .unwrap();

        let content = tool.file_read("test.txt").await.unwrap();
        assert_eq!(content, "line1\nappended");
    }

    #[tokio::test]
    async fn test_list() {
        let (tool, temp) = setup_test_vault();
        let vault_path = temp.path().join("vault");
        fs::write(vault_path.join("file1.txt"), "content1").unwrap();
        fs::write(vault_path.join("file2.txt"), "content2").unwrap();
        fs::create_dir_all(vault_path.join("subdir")).unwrap();

        let entries = tool.file_list("").await.unwrap();
        assert!(entries.len() >= 3);
    }

    #[tokio::test]
    async fn test_path_escape_blocked() {
        let (tool, _temp) = setup_test_vault();
        let result = tool.file_read("../escape.txt").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_private_path_blocked() {
        let (tool, _temp) = setup_test_vault();
        let result = tool.file_read("private/secret.txt").await;
        assert!(result.is_err());
    }
}
