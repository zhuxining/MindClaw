//! DeepSeek Provider 集成测试
//!
//! 运行需要设置环境变量: DEEPSEEK_API_KEY

use crate::providers::config::builtin_configs;
use crate::providers::openai_compat::OpenAICompatProvider;
use crate::providers::tests::require_env;
use crate::providers::traits::{ChatMessage, Provider};

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

    let messages = vec![ChatMessage {
        role: "user".to_string(),
        content: "Hello! Please respond with a short greeting.".to_string(),
        tool_call_id: None,
    }];

    let response = provider.chat(messages, None, 50).await;
    assert!(response.is_ok(), "Chat failed: {:?}", response.err());

    let resp = response.unwrap();
    assert!(!resp.content.is_empty(), "Response content is empty");
    assert!(resp.input_tokens > 0, "Input tokens should be > 0");
    assert!(resp.output_tokens > 0, "Output tokens should be > 0");

    println!("✓ DeepSeek chat test passed");
    println!("  Input tokens: {}", resp.input_tokens);
    println!("  Output tokens: {}", resp.output_tokens);
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

    let messages = vec![ChatMessage {
        role: "user".to_string(),
        content: "Count from 1 to 3.".to_string(),
        tool_call_id: None,
    }];

    let token_count = Arc::new(Mutex::new(0usize));
    let token_count_clone = Arc::clone(&token_count);

    let response = provider
        .chat_stream(
            messages,
            None,
            50,
            Box::new(move |_token: String| {
                let mut count = token_count_clone.lock().unwrap();
                *count += 1;
            }),
        )
        .await;

    assert!(response.is_ok(), "Stream chat failed: {:?}", response.err());

    let resp = response.unwrap();
    let tokens_received = *token_count.lock().unwrap();

    assert!(!resp.content.is_empty(), "Response content is empty");
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

    let messages = vec![ChatMessage {
        role: "user".to_string(),
        content: "What is 2+2?".to_string(),
        tool_call_id: None,
    }];

    let response = provider.chat(messages, None, 100).await;
    assert!(
        response.is_ok(),
        "Reasoner chat failed: {:?}",
        response.err()
    );

    let resp = response.unwrap();
    // Note: DeepSeek R1 may return reasoning in separate field,
    // just verify we got a response (content may be empty)
    assert!(
        !resp.content.is_empty() || resp.output_tokens > 0,
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
