use super::server::AcpServer;
use crate::services::core::{AgentResponse, ResponseStatus};
use agent_client_protocol::role::acp::Agent as AcpAgentRole;
use agent_client_protocol::schema::{InitializeRequest, ProtocolVersion};
use agent_client_protocol::{AcpAgent, Client, ConnectionTo};
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;

struct DispatchRequest {
    prompt: String,
    request_id: String,
    response_tx: oneshot::Sender<AgentResponse>,
}

/// ACP 客户端，管理通往单个 ACP Server 的持久连接。
pub struct AcpClient {
    dispatch_tx: mpsc::Sender<DispatchRequest>,
    timeout_secs: u64,
    #[allow(dead_code)]
    cancel_token: CancellationToken,
}

impl AcpClient {
    pub async fn connect(server: &AcpServer) -> Result<Self, String> {
        let agent = AcpAgent::new(server.to_mcp_server());
        let (dispatch_tx, mut dispatch_rx) = mpsc::channel::<DispatchRequest>(32);
        let cancel_token = CancellationToken::new();
        let cancel = cancel_token.clone();

        tokio::spawn(async move {
            let result = Client
                .builder()
                .name("mindclaw")
                .connect_with(agent, async |cx| {
                    cx.send_request(InitializeRequest::new(ProtocolVersion::V1))
                        .block_task()
                        .await?;

                    loop {
                        tokio::select! {
                            _ = cancel.cancelled() => break,
                            request = dispatch_rx.recv() => {
                                let Some(req) = request else { break };
                                let response = dispatch_once(&cx, &req).await;
                                let _ = req.response_tx.send(response);
                            }
                        }
                    }
                    Ok(())
                })
                .await;

            if let Err(error) = result {
                eprintln!("ACP connection ended with error: {error}");
            }
        });

        Ok(Self {
            dispatch_tx,
            timeout_secs: server.timeout_secs.max(1),
            cancel_token,
        })
    }

    pub async fn dispatch(&self, prompt: String, request_id: String) -> AgentResponse {
        let (tx, rx) = oneshot::channel();
        if self
            .dispatch_tx
            .send(DispatchRequest {
                prompt,
                request_id: request_id.clone(),
                response_tx: tx,
            })
            .await
            .is_err()
        {
            return AgentResponse {
                request_id,
                status: ResponseStatus::Error,
                output: String::new(),
                error_message: Some("ACP 连接已断开".to_string()),
            };
        }
        match tokio::time::timeout(std::time::Duration::from_secs(self.timeout_secs), rx).await {
            Ok(response) => response.unwrap_or_else(|_| AgentResponse {
                request_id,
                status: ResponseStatus::Error,
                output: String::new(),
                error_message: Some("ACP 响应通道已关闭".to_string()),
            }),
            Err(_) => AgentResponse {
                request_id,
                status: ResponseStatus::Timeout,
                output: String::new(),
                error_message: Some(format!("ACP 响应超时（{} 秒）", self.timeout_secs)),
            },
        }
    }

    #[allow(dead_code)]
    pub fn shutdown(&self) {
        self.cancel_token.cancel();
    }
}

async fn dispatch_once(cx: &ConnectionTo<AcpAgentRole>, req: &DispatchRequest) -> AgentResponse {
    let builder = match cx.build_session_cwd() {
        Ok(builder) => builder,
        Err(error) => {
            return AgentResponse {
                request_id: req.request_id.clone(),
                status: ResponseStatus::Error,
                output: String::new(),
                error_message: Some(format!("创建 session 失败: {error}")),
            };
        }
    };

    match builder
        .block_task()
        .run_until(async |mut session| {
            session.send_prompt(&req.prompt)?;
            session.read_to_string().await
        })
        .await
    {
        Ok(output) => AgentResponse {
            request_id: req.request_id.clone(),
            status: ResponseStatus::Success,
            output,
            error_message: None,
        },
        Err(error) => AgentResponse {
            request_id: req.request_id.clone(),
            status: ResponseStatus::Error,
            output: String::new(),
            error_message: Some(format!("ACP 调用失败: {error}")),
        },
    }
}
