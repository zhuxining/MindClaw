use crate::error::AppError;
use crate::providers::config::{ModelConfig, ProviderConfig};
use crate::providers::traits::ModelTier;
use crate::providers::ProviderRegistry;

fn custom_openai_compat_config() -> ProviderConfig {
    ProviderConfig {
        name: "test-openai".into(),
        api_base: "https://example.com/v1".into(),
        api_key_env: Some("HOME".into()),
        models: vec![
            ModelConfig {
                id: "fast-model".into(),
                display_name: "Fast Model".into(),
                tier: ModelTier::Fast,
                max_output_tokens: 1024,
                context_window: 8_192,
            },
            ModelConfig {
                id: "smart-model".into(),
                display_name: "Smart Model".into(),
                tier: ModelTier::Smart,
                max_output_tokens: 2048,
                context_window: 16_384,
            },
        ],
        default_model: "smart-model".into(),
    }
}

#[test]
fn provider_registry_should_load_builtin_configs() {
    let registry = ProviderRegistry::new();
    let mut providers = registry.available_providers();
    providers.sort_unstable();

    assert_eq!(providers, vec!["claude", "deepseek", "openai"]);
}

#[test]
fn provider_registry_should_register_custom_config_and_expose_models() {
    let mut registry = ProviderRegistry::new();
    let config = custom_openai_compat_config();
    let config_name = config.name.clone();
    registry.register(config);

    let models = registry
        .models_for(&config_name)
        .expect("custom config should expose models");

    assert_eq!(models.len(), 2);
    assert_eq!(models[0].id, "fast-model");
    assert_eq!(models[1].id, "smart-model");
}

#[test]
fn provider_registry_should_resolve_model_by_tier() {
    let mut registry = ProviderRegistry::new();
    registry.register(custom_openai_compat_config());

    let fast_model = registry
        .resolve_model_by_tier("test-openai", ModelTier::Fast)
        .expect("fast tier should resolve");
    let smart_model = registry
        .resolve_model_by_tier("test-openai", ModelTier::Smart)
        .expect("smart tier should resolve");

    assert_eq!(fast_model.id, "fast-model");
    assert_eq!(smart_model.id, "smart-model");
}

#[test]
fn provider_registry_should_create_openai_compat_provider_with_explicit_model() {
    let mut registry = ProviderRegistry::new();
    registry.register(custom_openai_compat_config());

    let provider = registry
        .create_provider("test-openai", "test-key".into(), Some("fast-model"))
        .expect("provider should be created");

    assert_eq!(provider.name(), "test-openai");
    assert_eq!(provider.model_id(), "fast-model");
    assert_eq!(provider.tier(), ModelTier::Fast);
}

#[test]
fn provider_registry_should_create_claude_provider_with_default_model() {
    let registry = ProviderRegistry::new();

    let provider = registry
        .create_provider("claude", "test-key".into(), None)
        .expect("claude provider should be created");

    assert_eq!(provider.name(), "claude");
    assert_eq!(provider.model_id(), "claude-sonnet-4-6");
    assert_eq!(provider.tier(), ModelTier::Smart);
}

#[test]
fn provider_registry_should_create_provider_from_env() {
    let mut registry = ProviderRegistry::new();
    registry.register(custom_openai_compat_config());

    let provider = registry
        .create_provider_from_env("test-openai", None)
        .expect("provider should be created from env");

    assert_eq!(provider.name(), "test-openai");
    assert_eq!(provider.model_id(), "smart-model");
    assert_eq!(provider.tier(), ModelTier::Smart);
}

#[test]
fn provider_registry_should_return_error_for_unknown_provider() {
    let registry = ProviderRegistry::new();

    let error = match registry.create_provider("missing", "test-key".into(), None) {
        Ok(_) => panic!("missing provider should error"),
        Err(error) => error,
    };

    match error {
        AppError::Provider(message) => assert_eq!(message, "unknown provider: missing"),
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn provider_registry_should_return_error_when_api_key_env_is_missing_in_config() {
    let mut registry = ProviderRegistry::new();
    let mut config = custom_openai_compat_config();
    config.name = "no-env-provider".into();
    config.api_key_env = None;
    registry.register(config);

    let error = match registry.create_provider_from_env("no-env-provider", None) {
        Ok(_) => panic!("missing env configuration should error"),
        Err(error) => error,
    };

    match error {
        AppError::Provider(message) => {
            assert_eq!(
                message,
                "provider 'no-env-provider' has no api_key_env configured"
            )
        }
        other => panic!("unexpected error: {other:?}"),
    }
}
