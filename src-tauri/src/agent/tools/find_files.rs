use super::path_guard::PathGuard;
use super::traits::{Tool, ToolInput, ToolOutput};
use crate::error::{AppError, AppResult};
use globset::{Glob, GlobSet, GlobSetBuilder};
use ignore::WalkBuilder;
use serde_json::{json, Value};
use std::sync::Arc;

/// 按 glob 模式在授权目录内递归查找文件/目录
pub struct FindFilesTool {
    guard: Arc<PathGuard>,
}

impl FindFilesTool {
    pub fn new(guard: Arc<PathGuard>) -> Self {
        Self { guard }
    }
}

#[async_trait::async_trait]
impl Tool for FindFilesTool {
    fn name(&self) -> &str {
        "find_files"
    }

    fn description(&self) -> &str {
        "Find files or directories matching a glob pattern within the vault. \
         Returns relative paths. Use pattern='*' to list files in a directory. \
         Examples: '*.md' for markdown files, 'src/**/*.rs' for Rust files in src/."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "文件名 glob 模式，如 *.md、**/*.rs、notes/**"
                },
                "path": {
                    "type": "string",
                    "description": "起始目录（相对于 vault 根目录，空字符串表示根目录）",
                    "default": ""
                },
                "file_type": {
                    "type": "string",
                    "enum": ["any", "file", "directory"],
                    "default": "file",
                    "description": "限制结果类型"
                },
                "max_depth": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 20,
                    "description": "最大递归深度（不设则无限制）"
                },
                "max_results": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 500,
                    "default": 100,
                    "description": "最多返回的结果数量"
                }
            },
            "required": ["pattern"]
        })
    }

    async fn execute(&self, input: ToolInput) -> AppResult<ToolOutput> {
        let pattern = input
            .parameters
            .get("pattern")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::Validation("missing 'pattern' parameter".into()))?
            .to_string();

        let path = input
            .parameters
            .get("path")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();

        let file_type = input
            .parameters
            .get("file_type")
            .and_then(Value::as_str)
            .unwrap_or("file")
            .to_string();

        let max_depth = input
            .parameters
            .get("max_depth")
            .and_then(Value::as_u64)
            .map(|v| v as usize);

        let max_results = input
            .parameters
            .get("max_results")
            .and_then(Value::as_u64)
            .unwrap_or(100) as usize;

        let search_root = self.guard.resolve(&path)?;
        let primary = self.guard.primary().to_path_buf();

        let glob_set = build_glob_set(&pattern).map_err(|e| {
            AppError::Validation(format!("invalid glob pattern '{}': {}", pattern, e))
        })?;

        let results = tokio::task::spawn_blocking(move || {
            let mut walker = WalkBuilder::new(&search_root);
            walker.hidden(false).git_ignore(true).git_global(false);

            if let Some(depth) = max_depth {
                walker.max_depth(Some(depth));
            }

            let mut found: Vec<String> = Vec::new();
            let mut truncated = false;

            for entry in walker.build().flatten() {
                let entry_path = entry.path();

                // 跳过搜索起点本身
                if entry_path == search_root {
                    continue;
                }

                let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);

                // 按 file_type 过滤
                let matches_type = match file_type.as_str() {
                    "file" => !is_dir,
                    "directory" => is_dir,
                    _ => true,
                };

                if !matches_type {
                    continue;
                }

                // 取文件名做 glob 匹配
                let name = entry_path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();

                // 同时尝试匹配相对于 primary 的完整路径
                let rel_path = entry_path
                    .strip_prefix(&primary)
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| name.clone());

                if glob_set.is_match(&name) || glob_set.is_match(&rel_path) {
                    if found.len() >= max_results {
                        truncated = true;
                        break;
                    }
                    found.push(rel_path);
                }
            }

            (found, truncated)
        })
        .await
        .map_err(|e| AppError::Internal(format!("find_files task failed: {}", e)))?;

        let (results, truncated) = results;
        let total = results.len();
        let output = json!({
            "total": total,
            "truncated": truncated,
            "results": results
        });

        Ok(ToolOutput::ok(
            serde_json::to_string_pretty(&output).unwrap_or_default(),
        ))
    }
}

fn build_glob_set(pattern: &str) -> Result<GlobSet, globset::Error> {
    let mut builder = GlobSetBuilder::new();
    builder.add(Glob::new(pattern)?);
    // 也支持匹配完整相对路径（如 notes/**/*.md）
    builder.add(Glob::new(&format!("**/{}", pattern))?);
    builder.build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup() -> (FindFilesTool, TempDir) {
        let tmp = TempDir::new().unwrap();
        let vault = tmp.path().join("vault");
        fs::create_dir_all(vault.join("notes/2024")).unwrap();
        fs::write(vault.join("notes/foo.md"), "hello").unwrap();
        fs::write(vault.join("notes/bar.md"), "world").unwrap();
        fs::write(vault.join("notes/2024/entry.md"), "entry").unwrap();
        fs::write(vault.join("notes/code.rs"), "fn main() {}").unwrap();

        let guard = Arc::new(PathGuard::vault_only(vault).with_denied("private"));
        (FindFilesTool::new(guard), tmp)
    }

    #[tokio::test]
    async fn test_find_md_files() {
        let (tool, _tmp) = setup();
        let input = ToolInput {
            parameters: json!({ "pattern": "*.md" }),
        };
        let output = tool.execute(input).await.unwrap();
        assert!(!output.is_error);
        let v: Value = serde_json::from_str(&output.content).unwrap();
        let results = v["results"].as_array().unwrap();
        assert!(results.len() >= 3);
        assert!(results.iter().all(|r| r.as_str().unwrap().ends_with(".md")));
    }

    #[tokio::test]
    async fn test_path_outside_vault_blocked() {
        let (tool, _tmp) = setup();
        let input = ToolInput {
            parameters: json!({ "pattern": "*.md", "path": "../etc" }),
        };
        let result = tool.execute(input).await;
        assert!(result.is_err());
    }
}
