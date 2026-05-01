//! 内容搜索工具 — rig ToolDyn 实现

use super::path_guard::PathGuard;
use grep_regex::RegexMatcherBuilder;
use grep_searcher::{SearcherBuilder, Sink, SinkMatch};
use ignore::WalkBuilder;
use rig::tool::ToolDyn;
use serde_json::{json, Value};
use std::pin::Pin;
use std::sync::Arc;

pub struct SearchContentTool {
    guard: Arc<PathGuard>,
}

impl SearchContentTool {
    pub fn new(guard: Arc<PathGuard>) -> Self {
        Self { guard }
    }
}

impl ToolDyn for SearchContentTool {
    fn name(&self) -> String {
        "search_content".into()
    }
    fn definition(
        &self,
        _: String,
    ) -> Pin<Box<dyn std::future::Future<Output = rig::completion::ToolDefinition> + Send + '_>>
    {
        let p = json!({"type":"object","properties":{"pattern":{"type":"string","description":"Regex or literal text to search for (case-insensitive by default)"},"path":{"type":"string","description":"Subdirectory to search within (default: vault root)"},"max_results":{"type":"integer","description":"Max matches to return (default 50)"}},"required":["pattern"]});
        Box::pin(async move {
            rig::completion::ToolDefinition {
                name: "search_content".into(),
                description: "Search file contents with regex within the vault".into(),
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
            let pattern = p["pattern"].as_str().unwrap_or("");
            let subdir = p["path"].as_str().unwrap_or("");
            let max = p["max_results"].as_u64().unwrap_or(50) as usize;
            if pattern.is_empty() {
                return Ok("Error: pattern is empty".into());
            }
            let root = if subdir.is_empty() {
                self.guard.primary().to_path_buf()
            } else {
                self.guard.primary().join(subdir)
            };
            let matcher = RegexMatcherBuilder::new()
                .case_insensitive(true)
                .build(pattern)
                .map_err(|e| rig::tool::ToolError::ToolCallError(Box::new(e)))?;
            let mut searcher = SearcherBuilder::new().build();
            let mut results = Vec::new();
            let mut count: usize = 0;
            for entry in WalkBuilder::new(&root)
                .max_depth(Some(20))
                .hidden(false)
                .build()
                .flatten()
            {
                if entry.file_type().map(|t| !t.is_file()).unwrap_or(true) || count >= max {
                    continue;
                }
                let rel = entry
                    .path()
                    .strip_prefix(self.guard.primary())
                    .unwrap_or(entry.path());
                struct Collector<'a> {
                    path: &'a std::path::Path,
                    results: &'a mut Vec<String>,
                    count: &'a mut usize,
                    max: usize,
                }
                impl Sink for Collector<'_> {
                    type Error = std::io::Error;
                    fn matched(
                        &mut self,
                        _searcher: &grep_searcher::Searcher,
                        mat: &SinkMatch<'_>,
                    ) -> Result<bool, Self::Error> {
                        let line = std::str::from_utf8(mat.buffer()).unwrap_or("");
                        self.results.push(format!(
                            "{}:{}:{}",
                            self.path.display(),
                            mat.line_number().unwrap_or(0),
                            line.trim()
                        ));
                        *self.count += 1;
                        Ok(*self.count < self.max)
                    }
                }
                let mut c = Collector {
                    path: rel,
                    results: &mut results,
                    count: &mut count,
                    max,
                };
                let _ = searcher.search_path(&matcher, entry.path(), &mut c);
            }
            if results.is_empty() {
                Ok("No matches found".into())
            } else {
                Ok(results.join("\n"))
            }
        })
    }
}
