use serde::Serialize;

/// 应用统一错误类型，实现 Serialize 以支持 Tauri Command 返回
#[derive(Debug, thiserror::Error, Serialize)]
pub enum AppError {
    /// 渠道通用错误（新，替代 FeishuGateway）
    #[error("渠道错误: {0}")]
    Gateway(String),

    /// 飞书网关错误（deprecated，保留向后兼容）
    #[error("飞书网关错误: {0}")]
    #[allow(dead_code)]
    FeishuGateway(String),

    /// 消息路由错误（v2 路由时使用）
    #[error("消息路由错误: {0}")]
    #[allow(dead_code)]
    MessageBus(String),

    #[error("ACP 客户端错误: {0}")]
    AcpClient(String),

    #[error("Agent 错误: {0}")]
    Agent(String),

    /// 存储错误（v2 SQLite 持久化时使用）
    #[error("存储错误: {0}")]
    #[allow(dead_code)]
    Storage(String),

    /// 配置错误（v2 文件配置加载时使用）
    #[error("配置错误: {0}")]
    #[allow(dead_code)]
    Config(String),

    #[error("未授权: {0}")]
    #[allow(dead_code)]
    Unauthorized(String),

    /// 内部错误（v2 集成错误处理时使用）
    #[error("内部错误: {0}")]
    #[allow(dead_code)]
    Internal(String),
}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        AppError::Gateway(format!("HTTP 请求失败: {}", e))
    }
}

impl From<agent_client_protocol::Error> for AppError {
    fn from(e: agent_client_protocol::Error) -> Self {
        AppError::AcpClient(e.to_string())
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        AppError::Storage(e.to_string())
    }
}
