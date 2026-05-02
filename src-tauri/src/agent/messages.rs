//! Agent Runtime message and tool schema contracts.
//!
//! These types are owned by the Agent Runtime. Provider adapters convert them
//! to and from provider-specific or third-party SDK types.

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MessageContent {
    Text {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
    ToolResult {
        tool_use_id: String,
        content: String,
        is_error: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: Vec<MessageContent>,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: vec![MessageContent::Text {
                text: content.into(),
            }],
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: vec![MessageContent::Text {
                text: content.into(),
            }],
        }
    }

    pub fn assistant_text(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: vec![MessageContent::Text {
                text: content.into(),
            }],
        }
    }

    pub fn assistant_parts(content: Vec<MessageContent>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content,
        }
    }

    pub fn tool_result(
        content: impl Into<String>,
        tool_call_id: impl Into<String>,
        is_error: bool,
    ) -> Self {
        Self {
            role: MessageRole::User,
            content: vec![MessageContent::ToolResult {
                tool_use_id: tool_call_id.into(),
                content: content.into(),
                is_error,
            }],
        }
    }

    pub fn text_content(&self) -> String {
        self.content
            .iter()
            .filter_map(|part| match part {
                MessageContent::Text { text } => Some(text.as_str()),
                MessageContent::ToolUse { .. } | MessageContent::ToolResult { .. } => None,
            })
            .collect::<Vec<_>>()
            .join("")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolChoice {
    Auto,
    None,
    Required,
    Specific(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolSchema {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}
