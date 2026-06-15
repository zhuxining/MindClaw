use crate::error::AppError;
use crate::services::acp_client::AcpClient;
use crate::services::agent::{
    AgentResolver, ConversationKey, SlashCommandAction, SlashCommandParser,
};
use crate::services::agent_context::AgentContextBuilder;
use crate::services::core::{AgentResponse, ChannelMessage, ResponseStatus};
use crate::services::event_bus::{EventBus, RuntimeEvent};
use crate::services::session_dispatcher::retry::RetryPolicy;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::mpsc;

use super::types::DispatchCommand;

const SESSION_IDLE_TIMEOUT: Duration = Duration::from_secs(300);

use std::time::Duration;

pub(crate) async fn session_worker(
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
