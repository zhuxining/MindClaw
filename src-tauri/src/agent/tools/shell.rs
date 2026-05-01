//! Shell 工具 — rig ToolDyn 实现
//!
//! 危险命令阻断、环境隔离、超时保护、输出截断。
//! 直接实现 `rig::tool::ToolDyn`，无自定义 trait 依赖。

use regex::Regex;
use rig::tool::ToolDyn;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::OnceLock;
use std::time::Duration;
use tokio::process::Command;

const MAX_TIMEOUT_SECS: u64 = 300;
const DEFAULT_TIMEOUT_SECS: u64 = 30;
const HALF_MAX_OUTPUT: usize = 5_000;

static DENY_PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();

fn deny_patterns() -> &'static [Regex] {
    DENY_PATTERNS.get_or_init(|| {
        [
            "rm\\s+(-\\w*[rR]\\w*|-\\w*[fF]\\w*\\s+-\\w*[rR]\\w*)",
            "del\\s+/[fFqQ]",
            "rmdir\\s+/[sS]",
            "format\\b",
            "mkfs|diskpart",
            "dd\\s+if=",
            ">/dev/sd[a-z]",
            "shutdown|reboot|poweroff|halt|init\\s+[06]",
            ":\\(\\)\\s*\\{.*\\};\\s*:",
            ">/etc/(passwd|shadow|sudoers|hosts)",
        ]
        .iter()
        .filter_map(|p| Regex::new(&format!("(?i){p}")).ok())
        .collect()
    })
}

#[cfg(not(target_os = "windows"))]
const SAFE_ENV_VARS: &[&str] = &[
    "PATH", "HOME", "TERM", "LANG", "LC_ALL", "USER", "SHELL", "TMPDIR",
];

#[cfg(target_os = "windows")]
const SAFE_ENV_VARS: &[&str] = &[
    "PATH",
    "PATHEXT",
    "HOME",
    "USERPROFILE",
    "HOMEDRIVE",
    "HOMEPATH",
    "SYSTEMROOT",
    "SYSTEMDRIVE",
    "WINDIR",
    "COMSPEC",
    "TEMP",
    "TMP",
    "TERM",
    "LANG",
    "USERNAME",
];

pub struct ShellTool {
    workspace_dir: PathBuf,
}

impl ShellTool {
    pub fn new(workspace_dir: impl Into<PathBuf>) -> Self {
        Self {
            workspace_dir: workspace_dir.into(),
        }
    }

    async fn execute_inner(&self, params: Value) -> Result<String, String> {
        let command = params
            .get("command")
            .and_then(Value::as_str)
            .map(|s| s.trim().to_string())
            .filter(|c| !c.is_empty())
            .ok_or("missing 'command' parameter")?;

        for pattern in deny_patterns() {
            if pattern.is_match(&command) {
                return Err(format!(
                    "Command blocked: matches deny pattern `{}`",
                    pattern.as_str()
                ));
            }
        }
        if command.contains("../") || command.contains("..\\") {
            return Err("Command blocked: path traversal detected".into());
        }

        let working_dir = {
            let rel = params
                .get("working_dir")
                .and_then(Value::as_str)
                .unwrap_or("");
            if rel.is_empty() {
                self.workspace_dir.clone()
            } else {
                let candidate = self.workspace_dir.join(rel);
                let normalized = normalize_path(&candidate);
                if !normalized.starts_with(&self.workspace_dir) {
                    return Err(format!("working_dir '{rel}' is outside workspace"));
                }
                normalized
            }
        };

        let timeout_secs = params
            .get("timeout")
            .and_then(Value::as_u64)
            .unwrap_or(DEFAULT_TIMEOUT_SECS)
            .min(MAX_TIMEOUT_SECS);

        #[cfg(not(target_os = "windows"))]
        let mut cmd = {
            let mut c = Command::new("sh");
            c.args(["-c", &command]);
            c
        };

        #[cfg(target_os = "windows")]
        let mut cmd = {
            let mut c = Command::new("cmd");
            c.args(["/C", &command]);
            c
        };

        cmd.current_dir(&working_dir);
        cmd.env_clear();
        for var in SAFE_ENV_VARS {
            if let Ok(val) = std::env::var(var) {
                cmd.env(var, val);
            }
        }

        let result = tokio::time::timeout(Duration::from_secs(timeout_secs), cmd.output()).await;
        match result {
            Err(_) => Err(format!("Command timed out after {timeout_secs}s")),
            Ok(Err(e)) => Err(format!("Failed to execute command: {e}")),
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let exit_code = output.status.code().unwrap_or(-1);
                let mut content = String::new();
                if !stdout.is_empty() {
                    content.push_str(&head_tail_truncate(&stdout, HALF_MAX_OUTPUT));
                }
                if !stderr.is_empty() {
                    if !content.is_empty() {
                        content.push('\n');
                    }
                    content.push_str("STDERR:\n");
                    content.push_str(&head_tail_truncate(&stderr, HALF_MAX_OUTPUT));
                }
                if content.is_empty() {
                    content.push_str("(no output)");
                }
                content.push_str(&format!("\nExit code: {exit_code}"));
                if output.status.success() {
                    Ok(content)
                } else {
                    Err(content)
                }
            }
        }
    }
}

impl ToolDyn for ShellTool {
    fn name(&self) -> String {
        "shell".into()
    }

    fn definition(
        &self,
        _prompt: String,
    ) -> Pin<Box<dyn std::future::Future<Output = rig::completion::ToolDefinition> + Send + '_>>
    {
        let desc = "Execute a shell command in the workspace directory. Destructive commands are blocked. Defaults to 30s timeout, max 300s.".to_string();
        let params = json!({
            "type": "object",
            "properties": {
                "command": {"type": "string", "description": "The shell command to execute"},
                "working_dir": {"type": "string", "description": "Working directory, relative to workspace"},
                "timeout": {"type": "integer", "minimum": 1, "maximum": 300, "description": "Timeout in seconds (default 30, max 300)"}
            },
            "required": ["command"]
        });
        Box::pin(async move {
            rig::completion::ToolDefinition {
                name: "shell".into(),
                description: desc,
                parameters: params,
            }
        })
    }

    fn call(
        &self,
        args: String,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<String, rig::tool::ToolError>> + Send + '_>>
    {
        Box::pin(async move {
            let params: Value =
                serde_json::from_str(&args).map_err(rig::tool::ToolError::JsonError)?;
            self.execute_inner(params).await.map_err(|e| {
                rig::tool::ToolError::ToolCallError(Box::new(std::io::Error::other(e)))
            })
        })
    }
}

fn head_tail_truncate(s: &str, half_max: usize) -> String {
    if s.len() <= half_max * 2 {
        return s.to_string();
    }
    let head_end = find_char_boundary(s, half_max);
    let tail_start = s.len() - find_char_boundary(&s[s.len() - half_max..], half_max);
    format!(
        "{}\n\n... ({} bytes omitted) ...\n\n{}",
        &s[..head_end],
        s.len() - head_end - (s.len() - tail_start),
        &s[tail_start..]
    )
}

fn find_char_boundary(s: &str, max_bytes: usize) -> usize {
    let mut b = max_bytes.min(s.len());
    while b > 0 && !s.is_char_boundary(b) {
        b -= 1;
    }
    b
}

fn normalize_path(path: &std::path::Path) -> PathBuf {
    let mut result = PathBuf::new();
    for c in path.components() {
        match c {
            std::path::Component::Prefix(_) | std::path::Component::RootDir => result.push(c),
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                result.pop();
            }
            std::path::Component::Normal(c) => result.push(c),
        }
    }
    result
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_basic_command_execution() {
        let temp = TempDir::new().expect("failed to create temp dir");
        let tool = ShellTool::new(temp.path());

        let result = tool.execute_inner(json!({"command": "echo hello"})).await;
        assert!(result.is_ok(), "basic echo should succeed");
        let output = result.unwrap();
        assert!(output.contains("hello"), "output should contain 'hello'");
    }

    #[tokio::test]
    async fn test_deny_rm_rf() {
        let temp = TempDir::new().expect("failed to create temp dir");
        let tool = ShellTool::new(temp.path());

        let result = tool
            .execute_inner(json!({"command": "rm -rf /tmp/test"}))
            .await;
        assert!(result.is_err(), "rm -rf should be blocked");
        let error = result.unwrap_err();
        assert!(
            error.contains("blocked"),
            "error message should mention blocking"
        );
    }

    #[tokio::test]
    async fn test_deny_format_command() {
        let temp = TempDir::new().expect("failed to create temp dir");
        let tool = ShellTool::new(temp.path());

        let result = tool.execute_inner(json!({"command": "format C:"})).await;
        assert!(result.is_err(), "format command should be blocked");
    }

    #[tokio::test]
    async fn test_path_traversal_blocked() {
        let temp = TempDir::new().expect("failed to create temp dir");
        let tool = ShellTool::new(temp.path());

        let result = tool
            .execute_inner(json!({
                "command": "cat file.txt",
                "working_dir": "../outside"
            }))
            .await;
        assert!(result.is_err(), "path traversal should be blocked");
        let error = result.unwrap_err();
        assert!(
            error.contains("outside workspace"),
            "error should mention workspace boundary"
        );
    }

    #[tokio::test]
    async fn test_timeout_enforcement() {
        let temp = TempDir::new().expect("failed to create temp dir");
        let tool = ShellTool::new(temp.path());

        // 短超时测试
        let result = tool
            .execute_inner(json!({
                "command": "sleep 10",
                "timeout": 1
            }))
            .await;
        assert!(result.is_err(), "long command should timeout");
        let error = result.unwrap_err();
        assert!(error.contains("timed out"), "error should mention timeout");
    }

    #[tokio::test]
    async fn test_working_dir_validation() {
        let temp = TempDir::new().expect("failed to create temp dir");
        let tool = ShellTool::new(temp.path());

        // 有效的工作目录
        let result = tool
            .execute_inner(json!({
                "command": "pwd",
                "working_dir": ""
            }))
            .await;
        assert!(result.is_ok(), "empty working_dir should use workspace");

        // 子目录（需要先创建）
        std::fs::create_dir_all(temp.path().join("subdir")).unwrap();
        let result = tool
            .execute_inner(json!({
                "command": "pwd",
                "working_dir": "subdir"
            }))
            .await;
        assert!(result.is_ok(), "valid subdir should succeed");
    }

    #[test]
    fn test_normalize_path() {
        assert_eq!(
            normalize_path(std::path::Path::new("/a/b/../c")),
            PathBuf::from("/a/c")
        );
        assert_eq!(
            normalize_path(std::path::Path::new("/a/./b")),
            PathBuf::from("/a/b")
        );
    }

    #[test]
    fn test_deny_patterns_compiled() {
        let patterns = deny_patterns();
        assert!(!patterns.is_empty(), "deny patterns should be compiled");

        // 验证关键模式存在
        let rm_rf_pattern = patterns
            .iter()
            .find(|p| p.as_str().contains("rm"))
            .expect("rm pattern should exist");
        assert!(rm_rf_pattern.is_match("rm -rf /"), "should match rm -rf");
    }
}
