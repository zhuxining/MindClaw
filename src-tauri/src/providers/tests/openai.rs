//! OpenAI Provider 集成测试
//!
//! 运行需要设置环境变量: OPENAI_API_KEY

use crate::providers::config::builtin_configs;
use crate::providers::openai_compat::OpenAICompatProvider;
use crate::providers::tests::require_env;
use crate::providers::traits::{ChatMessage, ChatRequest, Provider, ToolChoice};
use futures_util::StreamExt;
use tokio_util::sync::CancellationToken;

#[tokio::test]
#[ignore = "requires external OpenAI API access and credentials"]
async fn test_openai_chat() {
    let api_key = match require_env("OPENAI_API_KEY") {
        Some(key) => key,
        None => return,
    };

    let configs = builtin_configs();
    let openai_config = configs
        .iter()
        .find(|c| c.name == "openai")
        .expect("OpenAI config not found");

    let provider = OpenAICompatProvider::new(openai_config, api_key, Some("gpt-4o-mini"))
        .expect("Failed to create provider");

    let messages = vec![ChatMessage::user("Say hello briefly.")];

    let response = provider
        .chat(ChatRequest {
            model: provider.model_id(),
            messages: &messages,
            system: None,
            tools: &[],
            tool_choice: ToolChoice::None,
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

    println!("✓ OpenAI chat test passed");
}

#[tokio::test]
#[ignore = "requires external OpenAI API access and credentials"]
async fn test_openai_stream() {
    use std::sync::{Arc, Mutex};

    let api_key = match require_env("OPENAI_API_KEY") {
        Some(key) => key,
        None => return,
    };

    let configs = builtin_configs();
    let openai_config = configs
        .iter()
        .find(|c| c.name == "openai")
        .expect("OpenAI config not found");

    let provider = OpenAICompatProvider::new(openai_config, api_key, Some("gpt-4o-mini"))
        .expect("Failed to create provider");

    let messages = vec![ChatMessage::user("Count from 1 to 3.")];

    let token_count = Arc::new(Mutex::new(0usize));
    let token_count_clone = Arc::clone(&token_count);

    let stream = provider
        .chat_stream(ChatRequest {
            model: provider.model_id(),
            messages: &messages,
            system: None,
            tools: &[],
            tool_choice: ToolChoice::None,
            max_tokens: Some(50),
            cancel: CancellationToken::new(),
        })
        .await;

    assert!(stream.is_ok(), "Stream chat failed: {:?}", stream.err());

    let mut text = String::new();
    let mut stream = stream.unwrap();
    while let Some(event) = stream.next().await {
        match event.expect("stream event should be ok") {
            crate::agent::events::ProviderEvent::TextDelta { text: chunk } => {
                let mut count = token_count_clone.lock().unwrap();
                *count += 1;
                text.push_str(&chunk);
            }
            crate::agent::events::ProviderEvent::Finished { .. } => break,
            crate::agent::events::ProviderEvent::ToolCall { .. } => {}
        }
    }

    let tokens_received = *token_count.lock().unwrap();
    assert!(!text.is_empty(), "Response content is empty");
    assert!(tokens_received > 0, "Should have received tokens");

    println!("✓ OpenAI stream test passed");
    println!("  Tokens received: {}", tokens_received);
}

#[test]
fn test_openai_config() {
    let configs = builtin_configs();
    let openai = configs
        .iter()
        .find(|c| c.name == "openai")
        .expect("OpenAI config exists");

    assert_eq!(openai.api_base, "https://api.openai.com/v1");
    assert_eq!(openai.api_key_env, Some("OPENAI_API_KEY".to_string()));
    assert_eq!(openai.models.len(), 2);

    println!("✓ OpenAI config test passed");
}
