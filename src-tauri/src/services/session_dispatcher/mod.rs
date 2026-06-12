use crate::error::AppError;
use crate::services::acp_client::AcpClient;
use crate::services::agent::{
    AgentResolver, ConversationKey, SlashCommandAction, SlashCommandParser,
};
use crate::services::agent_context::AgentContextBuilder;
use crate::services::core::{AgentResponse, ChannelMessage, ResponseStatus};
use crate::services::event_bus::{EventBus, RuntimeEvent};
use dashmap::DashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};

pub struct RetryPolicy {
    pub max_retries: u32,
    pub base_delay_ms: u64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 2,
            base_delay_ms: 500,
        }
    }
}

impl RetryPolicy {
    fn backoff_duration(&self, attempt: u32) -> Duration {
        Duration::from_millis(self.base_delay_ms * 2u64.saturating_pow(attempt.saturating_sub(1)))
    }
}

const SESSION_IDLE_TIMEOUT: Duration = Duration::from_secs(300);

pub struct SessionDispatcher {
    resolver: Arc<AgentResolver>,
    acp_client: Arc<AcpClient>,
    event_bus: Arc<EventBus>,
    sessions: Arc<DashMap<ConversationKey, mpsc::UnboundedSender<DispatchCommand>>>,
}

struct DispatchCommand {
    message: ChannelMessage,
    responder: oneshot::Sender<Result<AgentResponse, AppError>>,
}

impl SessionDispatcher {
    pub fn new(
        resolver: Arc<AgentResolver>,
        acp_client: Arc<AcpClient>,
        event_bus: Arc<EventBus>,
    ) -> Self {
        Self {
            resolver,
            acp_client,
            event_bus,
            sessions: Arc::new(DashMap::new()),
        }
    }

    /// 按 session 保序分发消息：同一 `channel + conversation_id` 的消息 FIFO 处理，
    /// 不同 session 的消息并发执行。
    pub async fn dispatch(&self, message: ChannelMessage) -> Result<AgentResponse, AppError> {
        let key = ConversationKey {
            channel: message.channel.clone(),
            conversation_id: message.conversation_id.clone(),
        };

        let sessions = self.sessions.clone();
        let sender = self
            .sessions
            .entry(key.clone())
            .or_insert_with(|| {
                let (tx, rx) = mpsc::unbounded_channel();
                let worker_key = key.clone();
                tokio::spawn(session_worker(
                    rx,
                    self.resolver.clone(),
                    self.acp_client.clone(),
                    self.event_bus.clone(),
                    sessions,
                    worker_key,
                ));
                tx
            })
            .clone();

        let (responder, rx) = oneshot::channel();
        sender
            .send(DispatchCommand { message, responder })
            .map_err(|_| AppError::Internal("会话 worker 已关闭".to_string()))?;

        rx.await
            .map_err(|_| AppError::Internal("会话 worker 无响应".to_string()))?
    }
}

async fn session_worker(
    mut rx: mpsc::UnboundedReceiver<DispatchCommand>,
    resolver: Arc<AgentResolver>,
    acp_client: Arc<AcpClient>,
    event_bus: Arc<EventBus>,
    sessions: Arc<DashMap<ConversationKey, mpsc::UnboundedSender<DispatchCommand>>>,
    key: ConversationKey,
) {
    loop {
        let Some(cmd) = tokio::time::timeout(SESSION_IDLE_TIMEOUT, rx.recv())
            .await
            .ok()
            .flatten()
        else {
            sessions.remove(&key);
            break;
        };

        event_bus.publish(RuntimeEvent::MessageReceived {
            message_id: cmd.message.message_id.clone(),
            channel: cmd.message.channel.clone(),
            conversation_id: cmd.message.conversation_id.clone(),
        });

        let result = process_message(&resolver, &acp_client, &cmd.message).await;

        match &result {
            Ok(_) => {
                event_bus.publish(RuntimeEvent::DispatchSucceeded {
                    message_id: cmd.message.message_id.clone(),
                });
            }
            Err(error) => {
                event_bus.publish(RuntimeEvent::DispatchFailed {
                    message_id: cmd.message.message_id.clone(),
                    error: error.to_string(),
                });
            }
        }

        let _ = cmd.responder.send(result);
    }
}

async fn process_message(
    resolver: &AgentResolver,
    acp_client: &AcpClient,
    message: &ChannelMessage,
) -> Result<AgentResponse, AppError> {
    let key = ConversationKey {
        channel: message.channel.clone(),
        conversation_id: message.conversation_id.clone(),
    };

    match SlashCommandParser::parse(&message.content) {
        SlashCommandAction::PlainText(content) => {
            execute_default(resolver, acp_client, &key, message, &content).await
        }
        SlashCommandAction::Execute {
            agent_id,
            skill_id,
            content,
        } => {
            let context = resolver.context_for_agent(&agent_id, skill_id.as_deref())?;
            Ok(execute_with_retry(acp_client, message, &content, context).await)
        }
        SlashCommandAction::SwitchAgent { agent_id } => {
            let state = resolver.switch_conversation(key, agent_id)?;
            Ok(system_response(format!(
                "已切换当前会话 Agent：{}",
                state.agent_id
            )))
        }
        SlashCommandAction::SelectSkill { skill_id } => {
            let state = resolver.select_skill(key, skill_id)?;
            Ok(system_response(format!(
                "已切换当前会话 Skill：{}",
                state.skill_id.unwrap_or_else(|| "默认".to_string())
            )))
        }
        SlashCommandAction::ResetConversation => {
            resolver.reset_conversation(&key);
            Ok(system_response("已恢复默认 Agent".to_string()))
        }
        SlashCommandAction::Help => Ok(system_response(
            "可用命令：/agent、/skill、/use、/default、/help".to_string(),
        )),
    }
}

async fn execute_default(
    resolver: &AgentResolver,
    acp_client: &AcpClient,
    key: &ConversationKey,
    message: &ChannelMessage,
    content: &str,
) -> Result<AgentResponse, AppError> {
    let context = resolver.context_for_conversation(key)?;
    Ok(execute_with_retry(acp_client, message, content, context).await)
}

async fn execute_with_retry(
    acp_client: &AcpClient,
    message: &ChannelMessage,
    content: &str,
    context: crate::services::agent::ExecutionContext,
) -> AgentResponse {
    let prompt = AgentContextBuilder::build_prompt(&context, content);
    let mut attempt = 0u32;
    let retry = RetryPolicy::default();

    loop {
        let response = acp_client
            .dispatch(prompt.clone(), message.message_id.clone())
            .await;

        if response.status == ResponseStatus::Success || attempt >= retry.max_retries {
            return response;
        }

        attempt += 1;
        tokio::time::sleep(retry.backoff_duration(attempt)).await;
    }
}

fn system_response(output: String) -> AgentResponse {
    AgentResponse {
        request_id: uuid::Uuid::new_v4().to_string(),
        status: ResponseStatus::Success,
        output,
        error_message: None,
    }
}
