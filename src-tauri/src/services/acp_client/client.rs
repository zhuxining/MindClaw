use super::protocol::{AcpRequest, AcpResponse};
use crate::error::AppError;
use crate::services::message_bus::{AgentRequest, AgentResponse, ResponseStatus};
use std::time::Duration;

/// ACP Agent 客户端 — 通过 stdio 与本地 Agent 进程通信
pub struct AcpClient {
    /// Agent 可执行文件路径
    agent_path: String,
    /// 请求超时时间（秒）
    timeout_secs: u64,
}

impl AcpClient {
    /// 创建新的 ACP 客户端
    pub fn new(agent_path: String, timeout_secs: u64) -> Self {
        Self {
            agent_path,
            timeout_secs,
        }
    }

    /// 发送请求到 Agent 并等待响应
    pub async fn send(&self, request: AgentRequest) -> Result<AgentResponse, AppError> {
        let acp_request = AcpRequest::new(
            self.generate_request_id(),
            &request.message.content,
            Some(&request.agent_id),
        );

        let request_json = serde_json::to_string(&acp_request)
            .map_err(|e| AppError::AcpClient(format!("序列化 ACP 请求失败: {}", e)))?;

        // 启动子进程并通信
        let result = tokio::time::timeout(
            Duration::from_secs(self.timeout_secs),
            self.invoke_agent(request_json),
        )
        .await;

        match result {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(e)) => Err(e),
            Err(_) => Ok(AgentResponse {
                request_id: request.request_id,
                status: ResponseStatus::Timeout,
                output: String::new(),
                error_message: Some(format!("Agent 调用超时 ({} 秒)", self.timeout_secs)),
            }),
        }
    }

    /// 通过 stdio 调用 Agent 进程
    async fn invoke_agent(&self, request_json: String) -> Result<AgentResponse, AppError> {
        use tokio::io::AsyncWriteExt;
        use tokio::process::Command;

        let mut child = Command::new(&self.agent_path)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| AppError::AcpClient(format!("启动 Agent 进程失败: {}", e)))?;

        // 写入请求
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(request_json.as_bytes())
                .await
                .map_err(|e| AppError::AcpClient(format!("写入 Agent stdin 失败: {}", e)))?;
            stdin
                .write_all(b"\n")
                .await
                .map_err(|e| AppError::AcpClient(format!("写入 Agent stdin 换行失败: {}", e)))?;
        }

        // 读取响应
        let output = child
            .wait_with_output()
            .await
            .map_err(|e| AppError::AcpClient(format!("等待 Agent 响应失败: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::AcpClient(format!(
                "Agent 进程异常退出: {}",
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let response: AcpResponse = serde_json::from_str(&stdout)
            .map_err(|e| AppError::AcpClient(format!("解析 ACP 响应失败: {}", e)))?;

        self.parse_response(response)
    }

    /// 解析 ACP 响应为 AgentResponse
    fn parse_response(&self, response: AcpResponse) -> Result<AgentResponse, AppError> {
        if let Some(error) = response.error {
            return Ok(AgentResponse {
                request_id: String::new(),
                status: ResponseStatus::Error,
                output: String::new(),
                error_message: Some(format!("ACP 错误 [{}]: {}", error.code, error.message)),
            });
        }

        if let Some(result) = response.result {
            let output = result.message.map(|m| m.content).unwrap_or_default();

            let status = match result.status.as_deref() {
                Some("error") => ResponseStatus::Error,
                _ => ResponseStatus::Success,
            };

            return Ok(AgentResponse {
                request_id: String::new(),
                status,
                output,
                error_message: None,
            });
        }

        Err(AppError::AcpClient("ACP 响应格式无效".into()))
    }

    fn generate_request_id(&self) -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }
}
