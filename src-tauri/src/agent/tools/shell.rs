use super::traits::{Tool, ToolInput, ToolOutput};
use crate::error::AppResult;
use regex::Regex;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;
use tokio::process::Command;

/// 命令超时上限（秒）
const MAX_TIMEOUT_SECS: u64 = 300;
/// 默认超时（秒）
const DEFAULT_TIMEOUT_SECS: u64 = 30;
/// 单端输出截断阈值（head / tail 各最多保留这么多字节）
const HALF_MAX_OUTPUT: usize = 5_000;

/// 始终拒绝的危险命令正则（在 deny list 中的命令直接返回错误，不执行）
static DENY_PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();

fn deny_patterns() -> &'static [Regex] {
    DENY_PATTERNS.get_or_init(|| {
        let raw = [
            // 递归删除
            r"(?i)\brm\s+(-\w*[rR]\w*|-\w*[fF]\w*\s+-\w*[rR]\w*)\b",
            r"(?i)\bdel\s+/[fFqQ]\b",
            r"(?i)\brmdir\s+/[sS]\b",
            // 磁盘/分区破坏
            r"(?i)(?:^|[;&|]\s*)format\b",
            r"(?i)\b(?:mkfs|diskpart)\b",
            r"\bdd\s+if=",
            r">\s*/dev/sd[a-z]",
            // 系统电源
            r"(?i)\b(?:shutdown|reboot|poweroff|halt|init\s+[06])\b",
            // fork bomb
            r":\(\)\s*\{.*\};\s*:",
            // 覆盖系统文件
            r"(?i)>\s*/etc/(?:passwd|shadow|sudoers|hosts)",
        ];
        raw.iter().filter_map(|p| Regex::new(p).ok()).collect()
    })
}

/// 允许透传给子进程的环境变量（清空其他变量防止泄露 API keys）
#[cfg(not(target_os = "windows"))]
const SAFE_ENV_VARS: &[&str] = &[
    "PATH", "HOME", "TERM", "LANG", "LC_ALL", "LC_CTYPE", "USER", "SHELL", "TMPDIR",
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

/// Shell 执行工具
///
/// - 危险命令（deny list）直接拒绝
/// - 路径穿越检测
/// - 环境变量隔离（只透传 SAFE_ENV_VARS）
/// - 超时保护（默认 30s，最大 300s）
/// - 输出 head+tail 截断（各 5000 字节）
pub struct ShellTool {
    workspace_dir: PathBuf,
}

impl ShellTool {
    pub fn new(workspace_dir: impl Into<PathBuf>) -> Self {
        Self {
            workspace_dir: workspace_dir.into(),
        }
    }
}

#[async_trait::async_trait]
impl Tool for ShellTool {
    fn name(&self) -> &str {
        "shell"
    }

    fn description(&self) -> &str {
        "Execute a shell command in the workspace directory. \
         Destructive commands (rm -rf, shutdown, disk ops, etc.) are blocked. \
         Defaults to 30s timeout, max 300s."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "要执行的 shell 命令"
                },
                "working_dir": {
                    "type": "string",
                    "description": "工作目录（相对于 workspace，空=workspace 根目录）"
                },
                "timeout": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 300,
                    "description": "超时秒数（默认 30，最大 300）"
                }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, input: ToolInput) -> AppResult<ToolOutput> {
        let command = match input.parameters.get("command").and_then(Value::as_str) {
            Some(c) => c.trim().to_string(),
            None => return Ok(ToolOutput::err("missing 'command' parameter")),
        };

        if command.is_empty() {
            return Ok(ToolOutput::err("command is empty"));
        }

        // deny list 检查
        for pattern in deny_patterns() {
            if pattern.is_match(&command) {
                return Ok(ToolOutput::err(format!(
                    "Command blocked by safety policy: matches deny pattern `{}`",
                    pattern.as_str()
                )));
            }
        }

        // 路径穿越检测
        if command.contains("../") || command.contains("..\\") {
            return Ok(ToolOutput::err(
                "Command blocked: path traversal (`../`) detected",
            ));
        }

        // 解析工作目录（只允许 workspace 内的相对路径）
        let working_dir = {
            let rel = input
                .parameters
                .get("working_dir")
                .and_then(Value::as_str)
                .unwrap_or("");

            if rel.is_empty() {
                self.workspace_dir.clone()
            } else {
                let candidate = self.workspace_dir.join(rel);
                // 规范化后必须在 workspace 内
                let normalized = normalize_path(&candidate);
                if !normalized.starts_with(&self.workspace_dir) {
                    return Ok(ToolOutput::err(format!(
                        "working_dir '{}' is outside workspace",
                        rel
                    )));
                }
                normalized
            }
        };

        let timeout_secs = input
            .parameters
            .get("timeout")
            .and_then(Value::as_u64)
            .unwrap_or(DEFAULT_TIMEOUT_SECS)
            .min(MAX_TIMEOUT_SECS);

        // 构建命令
        #[cfg(target_os = "windows")]
        let mut cmd = {
            let mut c = Command::new("cmd");
            c.args(["/C", &command]);
            c
        };
        #[cfg(not(target_os = "windows"))]
        let mut cmd = {
            let mut c = Command::new("sh");
            c.args(["-c", &command]);
            c
        };

        cmd.current_dir(&working_dir);

        // 环境隔离：清空后只透传安全变量
        cmd.env_clear();
        for var in SAFE_ENV_VARS {
            if let Ok(val) = std::env::var(var) {
                cmd.env(var, val);
            }
        }

        // 执行 + 超时
        let result = tokio::time::timeout(Duration::from_secs(timeout_secs), cmd.output()).await;

        match result {
            Err(_) => Ok(ToolOutput::err(format!(
                "Command timed out after {}s",
                timeout_secs
            ))),

            Ok(Err(e)) => Ok(ToolOutput::err(format!("Failed to execute command: {}", e))),

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

                content.push_str(&format!("\nExit code: {}", exit_code));

                Ok(ToolOutput {
                    content,
                    is_error: !output.status.success(),
                })
            }
        }
    }
}

// ── 辅助函数 ──────────────────────────────────────────────

/// head + tail 截断：保留前 half_max 字节和后 half_max 字节，中间用省略号代替
fn head_tail_truncate(s: &str, half_max: usize) -> String {
    if s.len() <= half_max * 2 {
        return s.to_string();
    }

    // 找到安全的字符边界
    let head_end = find_char_boundary(s, half_max);
    let tail_start = s.len() - find_char_boundary(&s[s.len() - half_max..], half_max);

    let omitted = s.len() - head_end - (s.len() - tail_start);
    format!(
        "{}\n\n... ({} bytes omitted) ...\n\n{}",
        &s[..head_end],
        omitted,
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
    for component in path.components() {
        match component {
            std::path::Component::Prefix(_) | std::path::Component::RootDir => {
                result.push(component)
            }
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                result.pop();
            }
            std::path::Component::Normal(c) => result.push(c),
        }
    }
    result
}

// ── 测试 ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn tool() -> (ShellTool, TempDir) {
        let tmp = TempDir::new().unwrap();
        (ShellTool::new(tmp.path()), tmp)
    }

    fn input(params: Value) -> ToolInput {
        ToolInput { parameters: params }
    }

    #[tokio::test]
    async fn test_basic_command() {
        let (t, _tmp) = tool();
        let out = t
            .execute(input(json!({ "command": "echo hello" })))
            .await
            .unwrap();
        assert!(!out.is_error);
        assert!(out.content.contains("hello"));
    }

    #[tokio::test]
    async fn test_exit_code_nonzero() {
        let (t, _tmp) = tool();
        let out = t
            .execute(input(json!({ "command": "exit 1" })))
            .await
            .unwrap();
        assert!(out.is_error);
        assert!(out.content.contains("Exit code: 1"));
    }

    #[tokio::test]
    async fn test_deny_rm_rf() {
        let (t, _tmp) = tool();
        let out = t
            .execute(input(json!({ "command": "rm -rf /tmp/foo" })))
            .await
            .unwrap();
        assert!(out.is_error);
        assert!(out.content.contains("blocked"));
    }

    #[tokio::test]
    async fn test_deny_shutdown() {
        let (t, _tmp) = tool();
        let out = t
            .execute(input(json!({ "command": "sudo shutdown -h now" })))
            .await
            .unwrap();
        assert!(out.is_error);
        assert!(out.content.contains("blocked"));
    }

    #[tokio::test]
    async fn test_path_traversal_blocked() {
        let (t, _tmp) = tool();
        let out = t
            .execute(input(json!({ "command": "cat ../secret" })))
            .await
            .unwrap();
        assert!(out.is_error);
        assert!(out.content.contains("path traversal"));
    }

    #[tokio::test]
    async fn test_working_dir_outside_workspace_blocked() {
        let (t, _tmp) = tool();
        let out = t
            .execute(input(json!({ "command": "pwd", "working_dir": "/etc" })))
            .await
            .unwrap();
        assert!(out.is_error);
        assert!(out.content.contains("outside workspace"));
    }

    #[tokio::test]
    async fn test_timeout() {
        let (t, _tmp) = tool();
        let out = t
            .execute(input(json!({ "command": "sleep 5", "timeout": 1 })))
            .await
            .unwrap();
        assert!(out.is_error);
        assert!(out.content.contains("timed out"));
    }

    #[test]
    fn test_head_tail_truncate_short() {
        let s = "hello world";
        assert_eq!(head_tail_truncate(s, 100), s);
    }

    #[test]
    fn test_head_tail_truncate_long() {
        let s = "a".repeat(30_000);
        let result = head_tail_truncate(&s, 5_000);
        assert!(result.contains("omitted"));
        assert!(result.len() < s.len());
    }
}
