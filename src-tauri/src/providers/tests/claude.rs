//! Claude Provider 集成测试
//!
//! 运行需要设置环境变量: ANTHROPIC_API_KEY

use crate::providers::claude::ClaudeProvider;
use crate::providers::config::builtin_configs;
use crate::providers::tests::require_env;
use crate::providers::traits::{ChatMessage, ChatRequest, Provider, ToolChoice, ToolSchema};
use futures_util::StreamExt;
use tokio_util::sync::CancellationToken;

fn claude_config() -> crate::providers::config::ProviderConfig {
    builtin_configs()
        .into_iter()
        .find(|c| c.name == "claude")
        .expect("Claude config not found")
}

#[tokio::test]
#[ignore = "requires external Anthropic API access and credentials"]
async fn test_claude_chat() {
    let api_key = match require_env("ANTHROPIC_API_KEY") {
        Some(key) => key,
        None => return,
    };

    let config = claude_config();
    let provider = ClaudeProvider::new(&config, api_key, Some("claude-haiku-4-5-20251001"));

    let messages = vec![ChatMessage::user("Say hello briefly.")];

    let response = provider
        .chat(ChatRequest {
            model: provider.model_id(),
            messages: &messages,
            system: Some("You are a helpful assistant."),
            tools: &[],
            tool_choice: ToolChoice::Auto,
            max_tokens: Some(50),
            cancel: CancellationToken::new(),
        })
        .await;

    assert!(response.is_ok(), "Chat failed: {:?}", response.err());

    let resp = response.unwrap();
    assert!(
        !resp.message.text_content().is_empty(),
        "Response content is empty"
    );
    assert!(resp.usage.input_tokens > 0, "Should have input tokens");
    assert!(resp.usage.output_tokens > 0, "Should have output tokens");

    println!("✓ Claude chat: {}", resp.message.text_content());
}

#[tokio::test]
#[ignore = "requires external Anthropic API access and credentials"]
async fn test_claude_stream() {
    let api_key = match require_env("ANTHROPIC_API_KEY") {
        Some(key) => key,
        None => return,
    };

    let config = claude_config();
    let provider = ClaudeProvider::new(&config, api_key, Some("claude-haiku-4-5-20251001"));

    let messages = vec![ChatMessage::user("Count from 1 to 3.")];

    let stream = provider
        .chat_stream(ChatRequest {
            model: provider.model_id(),
            messages: &messages,
            system: None,
            tools: &[],
            tool_choice: ToolChoice::Auto,
            max_tokens: Some(100),
            cancel: CancellationToken::new(),
        })
        .await;

    assert!(stream.is_ok(), "Stream creation failed: {:?}", stream.err());

    let mut stream = stream.unwrap();
    let mut text = String::new();
    let mut chunk_count = 0;
    let mut got_finished = false;

    while let Some(event) = stream.next().await {
        match event.expect("stream event should be ok") {
            crate::agent::events::ProviderEvent::TextDelta { text: chunk } => {
                chunk_count += 1;
                text.push_str(&chunk);
            }
            crate::agent::events::ProviderEvent::StreamFinished { usage, .. } => {
                assert!(usage.input_tokens > 0);
                assert!(usage.output_tokens > 0);
                got_finished = true;
                break;
            }
            crate::agent::events::ProviderEvent::ToolCallReady { .. } => {}
        }
    }

    assert!(got_finished, "Should have received Finished event");
    assert!(!text.is_empty(), "Response content is empty");
    assert!(chunk_count > 0, "Should have received text chunks");

    println!("✓ Claude stream: chunks={chunk_count}, text={text}");
}

#[tokio::test]
#[ignore = "requires external Anthropic API access and credentials"]
async fn test_claude_tool_use() {
    let api_key = match require_env("ANTHROPIC_API_KEY") {
        Some(key) => key,
        None => return,
    };

    let config = claude_config();
    let provider = ClaudeProvider::new(&config, api_key, Some("claude-haiku-4-5-20251001"));

    let tools = vec![ToolSchema {
        name: "get_weather".into(),
        description: "Get the current weather in a given location".into(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "location": {
                    "type": "string",
                    "description": "The city and state, e.g. San Francisco, CA"
                }
            },
            "required": ["location"]
        }),
    }];

    let messages = vec![ChatMessage::user("What's the weather in Tokyo?")];

    let stream = provider
        .chat_stream(ChatRequest {
            model: provider.model_id(),
            messages: &messages,
            system: None,
            tools: &tools,
            tool_choice: ToolChoice::Auto,
            max_tokens: Some(200),
            cancel: CancellationToken::new(),
        })
        .await;

    assert!(stream.is_ok(), "Stream creation failed: {:?}", stream.err());

    let mut stream = stream.unwrap();
    let mut got_tool_call = false;
    let mut got_finished = false;

    while let Some(event) = stream.next().await {
        match event.expect("stream event should be ok") {
            crate::agent::events::ProviderEvent::TextDelta { .. } => {}
            crate::agent::events::ProviderEvent::ToolCallReady {
                name,
                arguments_json,
                ..
            } => {
                assert_eq!(name, "get_weather");
                assert!(arguments_json.get("location").is_some());
                got_tool_call = true;
            }
            crate::agent::events::ProviderEvent::StreamFinished { .. } => {
                got_finished = true;
                break;
            }
        }
    }

    assert!(got_tool_call, "Should have received a tool call");
    assert!(got_finished, "Should have received Finished event");

    println!("✓ Claude tool use test passed");
}
