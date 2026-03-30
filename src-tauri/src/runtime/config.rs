//! 应用配置：集中管理数据目录、Provider、上下文等配置项

use crate::error::{AppError, AppResult};
use std::path::PathBuf;

/// 应用全局配置
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// 应用数据根目录（~/.config/mindclaw）
    pub data_dir: PathBuf,
    /// SQLite 数据库路径
    pub db_path: PathBuf,
    /// Vault 存储目录（知识库 Markdown 文件）
    pub vault_path: PathBuf,
    /// LLM Provider 标识（如 "deepseek"、"openai"、"claude"）
    pub provider_id: String,
    /// 可选的 Model ID 覆盖
    pub model_id: Option<String>,
    /// MessageBus 通道容量
    pub bus_capacity: usize,
    /// Context 窗口 token 上限
    pub context_token_limit: usize,
    /// Agent 系统提示词
    pub system_prompt: String,
}

impl AppConfig {
    /// 获取平台标准应用数据目录
    pub fn default_data_dir() -> AppResult<PathBuf> {
        use directories::BaseDirs;
        let base = BaseDirs::new()
            .ok_or_else(|| AppError::Internal("cannot determine base directories".into()))?;
        Ok(base.config_dir().join("mindclaw"))
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        let data_dir = Self::default_data_dir().unwrap_or_else(|_| PathBuf::from(".mindclaw"));
        Self {
            db_path: data_dir.join("mindclaw.db"),
            vault_path: data_dir.join("vault"),
            data_dir,
            provider_id: "deepseek".to_string(),
            model_id: None,
            bus_capacity: 100,
            context_token_limit: 128_000,
            system_prompt: "你是一个智能助手，可以帮助用户完成各种任务。".to_string(),
        }
    }
}
