//! Provider 集成测试
//!
//! 运行需要设置对应的环境变量：
//! - DEEPSEEK_API_KEY
//! - OPENAI_API_KEY
//! - ANTHROPIC_API_KEY

use std::path::PathBuf;
use std::sync::Once;

mod deepseek;
mod openai;
mod registry;

static LOAD_ENV: Once = Once::new();

fn ensure_test_env_loaded() {
    LOAD_ENV.call_once(|| {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let candidate_paths = [
            cwd.join(".env.test.local"),
            cwd.join(".env.local"),
            cwd.join(".env.test"),
            cwd.join(".env"),
            cwd.parent()
                .map(|parent| parent.join(".env.test.local"))
                .unwrap_or_else(|| PathBuf::from("../.env.test.local")),
            cwd.parent()
                .map(|parent| parent.join(".env.local"))
                .unwrap_or_else(|| PathBuf::from("../.env.local")),
            cwd.parent()
                .map(|parent| parent.join(".env.test"))
                .unwrap_or_else(|| PathBuf::from("../.env.test")),
            cwd.parent()
                .map(|parent| parent.join(".env"))
                .unwrap_or_else(|| PathBuf::from("../.env")),
        ];

        for path in candidate_paths {
            if path.is_file() {
                let _ = dotenvy::from_path(&path);
            }
        }
    });
}

fn require_env(var_name: &str) -> Option<String> {
    ensure_test_env_loaded();

    match std::env::var(var_name) {
        Ok(value) if !value.trim().is_empty() => Some(value),
        _ => {
            let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            eprintln!(
                "跳过测试: {var_name} 未设置。可通过 shell export，或在以下位置创建 .env/.env.local: {}, {}",
                cwd.display(),
                cwd.parent()
                    .map(|parent| parent.display().to_string())
                    .unwrap_or_else(|| "..".to_string())
            );
            None
        }
    }
}
