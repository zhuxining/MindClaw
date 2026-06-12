/// ACP Server 发起的本地工具调用请求。
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ToolCall {
    /// 工具调用唯一标识
    pub call_id: String,
    /// 工具名称
    pub name: String,
    /// 工具参数（JSON）
    pub arguments: serde_json::Value,
}

/// 本地工具执行结果。
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ToolResult {
    /// 对应的工具调用 ID
    pub call_id: String,
    /// 执行结果内容
    pub content: String,
    /// 是否执行失败
    pub is_error: bool,
}

/// 本地工具执行器 trait。
///
/// 实现此 trait 以注册和处理 ACP Server 发起的本地工具调用。
#[allow(dead_code)]
#[async_trait::async_trait]
pub trait ToolExecutor: Send + Sync {
    /// 执行本地工具调用。
    async fn execute(&self, call: ToolCall) -> ToolResult;

    /// 检查是否允许执行指定工具。
    fn is_allowed(&self, tool_name: &str) -> bool;
}

/// 空工具执行器 — 拒绝所有工具调用。
pub struct NoopToolExecutor;

#[async_trait::async_trait]
impl ToolExecutor for NoopToolExecutor {
    async fn execute(&self, call: ToolCall) -> ToolResult {
        ToolResult {
            call_id: call.call_id,
            content: format!("工具 '{}' 未注册", call.name),
            is_error: true,
        }
    }

    fn is_allowed(&self, _tool_name: &str) -> bool {
        false
    }
}
