use super::traits::{Tool, ToolInput, ToolOutput};
use crate::error::AppResult;
use serde_json::{json, Value};

/// vault 内文件操作（安全边界约束：只允许访问 vault 目录，拒绝 private/ 路径）
#[allow(dead_code)]
pub struct FilesystemTool {
    vault_root: std::path::PathBuf,
}

impl FilesystemTool {
    pub fn new(vault_root: impl Into<std::path::PathBuf>) -> Self {
        Self {
            vault_root: vault_root.into(),
        }
    }

    #[allow(dead_code)]
    fn is_allowed(&self, path: &str) -> bool {
        !path.contains("private/") && !path.starts_with("private")
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
                "operation": { "type": "string", "enum": ["read", "write", "list", "delete"] },
                "path": { "type": "string" },
                "content": { "type": "string" }
            },
            "required": ["operation", "path"]
        })
    }

    async fn execute(&self, _input: ToolInput) -> AppResult<ToolOutput> {
        todo!("实现文件系统工具")
    }
}
