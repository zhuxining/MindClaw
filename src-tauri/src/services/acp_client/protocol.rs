use serde::{Deserialize, Serialize};

/// ACP 协议请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpRequest {
    /// JSON-RPC 版本
    pub jsonrpc: String,
    /// 请求方法
    pub method: String,
    /// 请求参数
    pub params: AcpParams,
    /// 请求 ID
    pub id: u64,
}

/// ACP 请求参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpParams {
    /// 用户消息内容
    pub message: AcpMessage,
    /// Agent 配置
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<AcpAgentConfig>,
}

/// ACP 消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpMessage {
    /// 消息角色
    pub role: String,
    /// 消息内容
    pub content: String,
}

/// ACP Agent 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpAgentConfig {
    /// Agent 名称
    pub name: Option<String>,
    /// 模型名称
    pub model: Option<String>,
    /// 最大 token 数
    pub max_tokens: Option<u32>,
}

/// ACP 协议响应
#[derive(Debug, Clone, Deserialize)]
pub struct AcpResponse {
    /// JSON-RPC 版本
    #[allow(dead_code)]
    pub jsonrpc: Option<String>,
    /// 结果
    pub result: Option<AcpResult>,
    /// 错误
    pub error: Option<AcpError>,
    /// 请求 ID
    #[allow(dead_code)]
    pub id: Option<u64>,
}

/// ACP 成功结果
#[derive(Debug, Clone, Deserialize)]
pub struct AcpResult {
    /// 回复消息
    pub message: Option<AcpMessage>,
    /// 处理状态
    pub status: Option<String>,
    /// 使用的 token 数
    #[allow(dead_code)]
    pub usage: Option<AcpUsage>,
}

/// ACP Token 用量
#[derive(Debug, Clone, Deserialize)]
pub struct AcpUsage {
    #[allow(dead_code)]
    pub prompt_tokens: Option<u32>,
    #[allow(dead_code)]
    pub completion_tokens: Option<u32>,
}

/// ACP 错误
#[derive(Debug, Clone, Deserialize)]
pub struct AcpError {
    pub code: i32,
    pub message: String,
}

impl AcpRequest {
    /// 创建一个新的 ACP 请求
    pub fn new(id: u64, message_content: &str, agent_name: Option<&str>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method: "agent/run".to_string(),
            params: AcpParams {
                message: AcpMessage {
                    role: "user".to_string(),
                    content: message_content.to_string(),
                },
                config: agent_name.map(|name| AcpAgentConfig {
                    name: Some(name.to_string()),
                    model: None,
                    max_tokens: None,
                }),
            },
            id,
        }
    }
}
