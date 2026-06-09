use serde::Serialize;

/// Gateway 层统一错误类型
///
/// 替代原来的 `AppError::FeishuGateway`，所有渠道 Gateway 共用。
#[derive(Debug, thiserror::Error, Serialize)]
pub enum GatewayError {
    /// 网络/HTTP 错误（可重试）
    #[error("网络错误: {0}")]
    Network(String),

    /// API 业务错误（不可重试）
    #[error("API 错误: code={code}, msg={msg}")]
    #[allow(dead_code)]
    Api { code: i32, msg: String },

    /// 凭证未配置
    #[error("凭证未配置")]
    Unauthorized,

    /// 凭证无效或已过期且无法刷新
    #[error("凭证无效: {0}")]
    InvalidCredentials(String),

    /// 消息格式转换失败
    #[error("消息格式转换失败: {0}")]
    #[allow(dead_code)]
    Conversion(String),

    /// 渠道不支持某操作
    #[error("不支持的操作: {0}")]
    Unsupported(String),
}

impl GatewayError {
    /// 是否可重试
    #[allow(dead_code)]
    pub fn is_retryable(&self) -> bool {
        matches!(self, GatewayError::Network(_))
    }
}

impl From<reqwest::Error> for GatewayError {
    fn from(e: reqwest::Error) -> Self {
        GatewayError::Network(format!("HTTP 请求失败: {}", e))
    }
}
