//! OpenAI Provider 集成测试
//!
//! 运行需要设置环境变量: OPENAI_API_KEY

use crate::providers::config::builtin_configs;
use crate::providers::openai_compat::OpenAICompatProvider;
use crate::providers::tests::require_env;
use crate::providers::traits::{ChatMessage, Provider};

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

    let messages = vec![ChatMessage {
        role: "user".to_string(),
        content: "Say hello briefly.".to_string(),
        tool_call_id: None,
    }];

    let response = provider.chat(messages, None, 50).await;
    assert!(response.is_ok(), "Chat failed: {:?}", response.err());

    let resp = response.unwrap();
    assert!(!resp.content.is_empty(), "Response content is empty");

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
