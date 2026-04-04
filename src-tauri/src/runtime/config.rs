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
    /// Agent 采样温度（默认 None，使用 Provider 默认值）
    pub agent_temperature: Option<f32>,
    /// Agent 最大 token 数（默认 None，使用 Provider 默认值）
    pub agent_max_tokens: Option<usize>,
    /// Agent 最大迭代次数（默认 8）
    pub agent_max_iterations: usize,
    /// 工具并行执行限制（默认 4）
    pub tool_concurrency: usize,
    /// LLM 请求并发限制（默认 3）
    pub llm_concurrency: usize,
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
            agent_temperature: None,
            agent_max_tokens: None,
            agent_max_iterations: 8,
            tool_concurrency: 4,
            llm_concurrency: 3,
        }
    }
}
