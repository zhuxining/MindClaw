use super::path_guard::PathGuard;
use super::traits::{Tool, ToolInput, ToolOutput};
use crate::error::{AppError, AppResult};
use grep_regex::RegexMatcherBuilder;
use grep_searcher::{BinaryDetection, SearcherBuilder, Sink, SinkContext, SinkMatch};
use ignore::WalkBuilder;
use serde_json::{json, Value};
use std::path::Path;
use std::sync::Arc;

const MAX_OUTPUT_BYTES: usize = 16 * 1024; // 16KB 硬截断

/// 在授权目录内用正则表达式搜索文件内容
pub struct SearchContentTool {
    guard: Arc<PathGuard>,
}

impl SearchContentTool {
    pub fn new(guard: Arc<PathGuard>) -> Self {
        Self { guard }
    }
}

#[async_trait::async_trait]
impl Tool for SearchContentTool {
    fn name(&self) -> &str {
        "search_content"
    }

    fn description(&self) -> &str {
        "Search file contents using a regex pattern within the vault. \
         Returns grep-style output with file paths, line numbers, and context."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "正则表达式搜索模式"
                },
                "path": {
                    "type": "string",
                    "description": "搜索目录（相对于 vault 根目录，空字符串表示根目录）",
                    "default": ""
                },
                "glob": {
                    "type": "string",
                    "description": "限制搜索的文件 glob，如 *.md、*.txt"
                },
                "case_sensitive": {
                    "type": "boolean",
                    "default": false,
                    "description": "是否区分大小写"
                },
                "context_lines": {
                    "type": "integer",
                    "minimum": 0,
                    "maximum": 10,
                    "default": 2,
                    "description": "每个匹配项前后显示的上下文行数"
                },
                "max_results": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 200,
                    "default": 50,
                    "description": "最多返回的匹配数量"
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

        let glob_filter = input
            .parameters
            .get("glob")
            .and_then(Value::as_str)
            .map(|s| s.to_string());

        let case_sensitive = input
            .parameters
            .get("case_sensitive")
            .and_then(Value::as_bool)
            .unwrap_or(false);

        let context_lines = input
            .parameters
            .get("context_lines")
            .and_then(Value::as_u64)
            .unwrap_or(2) as usize;

        let max_results = input
            .parameters
            .get("max_results")
            .and_then(Value::as_u64)
            .unwrap_or(50) as usize;

        let search_root = self.guard.resolve(&path)?;
        let primary = self.guard.primary().to_path_buf();

        let output = tokio::task::spawn_blocking(move || {
            search_files(
                &search_root,
                &primary,
                &pattern,
                glob_filter.as_deref(),
                case_sensitive,
                context_lines,
                max_results,
            )
        })
        .await
        .map_err(|e| AppError::Internal(format!("search_content task failed: {}", e)))?
        .map_err(|e| AppError::Validation(format!("search error: {}", e)))?;

        Ok(ToolOutput::ok(output))
    }
}

fn search_files(
    search_root: &Path,
    primary: &Path,
    pattern: &str,
    glob_filter: Option<&str>,
    case_sensitive: bool,
    context_lines: usize,
    max_results: usize,
) -> Result<String, String> {
    let matcher = RegexMatcherBuilder::new()
        .case_insensitive(!case_sensitive)
        .build(pattern)
        .map_err(|e| format!("invalid regex '{}': {}", pattern, e))?;

    let mut walker = WalkBuilder::new(search_root);
    walker.hidden(false).git_ignore(true).git_global(false);

    if let Some(glob) = glob_filter {
        let mut types_builder = ignore::types::TypesBuilder::new();
        types_builder
            .add("custom", glob)
            .map_err(|e| e.to_string())?;
        types_builder.select("custom");
        let types = types_builder.build().map_err(|e| e.to_string())?;
        walker.types(types);
    }

    let mut searcher = SearcherBuilder::new()
        .binary_detection(BinaryDetection::quit(0))
        .before_context(context_lines)
        .after_context(context_lines)
        .build();

    let mut output = String::new();
    let mut total_matches = 0usize;
    let mut total_files = 0usize;
    let mut truncated = false;

    'walk: for entry in walker.build().flatten() {
        if entry.file_type().map(|t| t.is_dir()).unwrap_or(true) {
            continue;
        }

        let file_path = entry.path();
        let rel_path = file_path
            .strip_prefix(primary)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| file_path.to_string_lossy().to_string());

        let mut sink = CollectingSink {
            matches: Vec::new(),
            limit: max_results - total_matches,
            overflow: false,
        };

        let _ = searcher.search_path(&matcher, file_path, &mut sink);

        if sink.matches.is_empty() {
            continue;
        }

        // 追加文件标题
        if !output.is_empty() {
            output.push('\n');
        }
        output.push_str(&rel_path);
        output.push('\n');

        for line in &sink.matches {
            output.push_str(line);
            output.push('\n');
            total_matches += 1;
        }

        if sink.overflow {
            truncated = true;
        }

        total_files += 1;

        if output.len() >= MAX_OUTPUT_BYTES || truncated || total_matches >= max_results {
            truncated = true;
            break 'walk;
        }
    }

    if total_matches == 0 {
        return Ok(format!("[no matches for '{}']", pattern));
    }

    let summary = if truncated {
        format!(
            "[showing first {} matches in {} files, truncated]",
            total_matches, total_files
        )
    } else {
        format!("[{} matches in {} files]", total_matches, total_files)
    };

    output.push_str("--\n");
    output.push_str(&summary);

    // 最终硬截断保护
    if output.len() > MAX_OUTPUT_BYTES {
        output.truncate(MAX_OUTPUT_BYTES);
        output.push_str("\n... [output truncated]");
    }

    Ok(output)
}

// ── Sink 实现 ──────────────────────────────────────────────

struct CollectingSink {
    matches: Vec<String>,
    limit: usize,
    overflow: bool,
}

impl Sink for CollectingSink {
    type Error = std::io::Error;

    fn matched(
        &mut self,
        _searcher: &grep_searcher::Searcher,
        mat: &SinkMatch<'_>,
    ) -> Result<bool, Self::Error> {
        if self.matches.len() >= self.limit {
            self.overflow = true;
            return Ok(false);
        }
        let line_number = mat.line_number().unwrap_or(0);
        let text = String::from_utf8_lossy(mat.bytes()).trim_end().to_string();
        self.matches.push(format!("  {}: {}", line_number, text));
        Ok(true)
    }

    fn context(
        &mut self,
        _searcher: &grep_searcher::Searcher,
        ctx: &SinkContext<'_>,
    ) -> Result<bool, Self::Error> {
        let line_number = ctx.line_number().unwrap_or(0);
        let text = String::from_utf8_lossy(ctx.bytes()).trim_end().to_string();
        self.matches.push(format!("  {}: {}", line_number, text));
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup() -> (SearchContentTool, TempDir) {
        let tmp = TempDir::new().unwrap();
        let vault = tmp.path().join("vault");
        fs::create_dir_all(&vault).unwrap();
        fs::write(
            vault.join("a.md"),
            "Hello world\nThis is a test\nAnother line",
        )
        .unwrap();
        fs::write(vault.join("b.md"), "No match here\nJust some text").unwrap();
        fs::write(vault.join("c.md"), "hello again\nHELLO uppercase").unwrap();

        let guard = Arc::new(PathGuard::vault_only(vault).with_denied("private"));
        (SearchContentTool::new(guard), tmp)
    }

    #[tokio::test]
    async fn test_basic_search() {
        let (tool, _tmp) = setup();
        let input = ToolInput {
            parameters: json!({ "pattern": "hello", "context_lines": 0 }),
        };
        let output = tool.execute(input).await.unwrap();
        assert!(!output.is_error);
        assert!(output.content.contains("a.md") || output.content.contains("c.md"));
    }

    #[tokio::test]
    async fn test_case_insensitive_default() {
        let (tool, _tmp) = setup();
        let input = ToolInput {
            parameters: json!({ "pattern": "hello", "context_lines": 0 }),
        };
        let output = tool.execute(input).await.unwrap();
        // should match both "Hello" and "hello" and "HELLO"
        assert!(output.content.contains("a.md"));
        assert!(output.content.contains("c.md"));
    }

    #[tokio::test]
    async fn test_no_matches() {
        let (tool, _tmp) = setup();
        let input = ToolInput {
            parameters: json!({ "pattern": "xyznotfound" }),
        };
        let output = tool.execute(input).await.unwrap();
        assert!(!output.is_error);
        assert!(output.content.contains("no matches"));
    }

    #[tokio::test]
    async fn test_invalid_regex() {
        let (tool, _tmp) = setup();
        let input = ToolInput {
            parameters: json!({ "pattern": "[invalid" }),
        };
        let output = tool.execute(input).await;
        assert!(output.is_err() || output.unwrap().is_error);
    }
}
