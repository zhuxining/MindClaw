//! DeepSeek Provider 集成测试
//!
//! 运行需要设置环境变量: DEEPSEEK_API_KEY

use crate::providers::config::builtin_configs;
use crate::providers::openai_compat::OpenAICompatProvider;
use crate::providers::tests::require_env;
use crate::providers::traits::{ChatMessage, ChatRequest, Provider, ToolChoice};
use futures_util::StreamExt;
use tokio_util::sync::CancellationToken;

#[tokio::test]
#[ignore = "requires external DeepSeek API access and credentials"]
async fn test_deepseek_chat() {
    let api_key = match require_env("DEEPSEEK_API_KEY") {
        Some(key) => key,
        None => return,
    };

    let configs = builtin_configs();
    let deepseek_config = configs
        .iter()
        .find(|c| c.name == "deepseek")
        .expect("DeepSeek config not found");

    let provider = OpenAICompatProvider::new(deepseek_config, api_key, Some("deepseek-chat"))
        .expect("Failed to create provider");

    let messages = vec![ChatMessage::user(
        "Hello! Please respond with a short greeting.",
    )];

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
    assert!(resp.usage.input_tokens > 0, "Input tokens should be > 0");
    assert!(resp.usage.output_tokens > 0, "Output tokens should be > 0");

    println!("✓ DeepSeek chat test passed");
    println!("  Input tokens: {}", resp.usage.input_tokens);
    println!("  Output tokens: {}", resp.usage.output_tokens);
}

#[tokio::test]
#[ignore = "requires external DeepSeek API access and credentials"]
async fn test_deepseek_stream() {
    use std::sync::{Arc, Mutex};

    let api_key = match require_env("DEEPSEEK_API_KEY") {
        Some(key) => key,
        None => return,
    };

    let configs = builtin_configs();
    let deepseek_config = configs
        .iter()
        .find(|c| c.name == "deepseek")
        .expect("DeepSeek config not found");

    let provider = OpenAICompatProvider::new(deepseek_config, api_key, Some("deepseek-chat"))
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

    println!("✓ DeepSeek stream test passed");
    println!("  Tokens received: {}", tokens_received);
}

#[tokio::test]
#[ignore = "requires external DeepSeek API access and credentials"]
async fn test_deepseek_reasoner() {
    let api_key = match require_env("DEEPSEEK_API_KEY") {
        Some(key) => key,
        None => return,
    };

    let configs = builtin_configs();
    let deepseek_config = configs
        .iter()
        .find(|c| c.name == "deepseek")
        .expect("DeepSeek config not found");

    let provider = OpenAICompatProvider::new(deepseek_config, api_key, Some("deepseek-reasoner"))
        .expect("Failed to create provider");

    let messages = vec![ChatMessage::user("What is 2+2?")];

    let response = provider
        .chat(ChatRequest {
            model: provider.model_id(),
            messages: &messages,
            system: None,
            tools: &[],
            tool_choice: ToolChoice::None,
            max_tokens: Some(100),
            cancel: CancellationToken::new(),
        })
        .await;
    assert!(
        response.is_ok(),
        "Reasoner chat failed: {:?}",
        response.err()
    );

    let resp = response.unwrap();
    // Note: DeepSeek R1 may return reasoning in separate field,
    // just verify we got a response (content may be empty)
    assert!(
        !resp.message.text_content().is_empty() || resp.usage.output_tokens > 0,
        "Response should have content or tokens"
    );

    println!("✓ DeepSeek reasoner test passed");
}

#[test]
fn test_deepseek_config() {
    let configs = builtin_configs();
    let deepseek = configs
        .iter()
        .find(|c| c.name == "deepseek")
        .expect("DeepSeek config exists");

    assert_eq!(deepseek.api_base, "https://api.deepseek.com");
    assert_eq!(deepseek.api_key_env, Some("DEEPSEEK_API_KEY".to_string()));
    assert_eq!(deepseek.models.len(), 2);

    let chat_model = deepseek
        .models
        .iter()
        .find(|m| m.id == "deepseek-chat")
        .unwrap();
    assert_eq!(chat_model.display_name, "DeepSeek V3");

    let reasoner_model = deepseek
        .models
        .iter()
        .find(|m| m.id == "deepseek-reasoner")
        .unwrap();
    assert_eq!(reasoner_model.display_name, "DeepSeek R1");

    println!("✓ DeepSeek config test passed");
}
