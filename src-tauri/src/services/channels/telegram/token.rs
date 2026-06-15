use crate::error::AppError;
use crate::services::channels::CredentialsManager;
use crate::services::gateway::GatewayError;
use tokio::sync::RwLock;

/// Telegram Token 管理器
///
/// Telegram 使用 Bot Token 进行身份认证，不需要 OAuth 刷新流程。
/// Token 通过 `set_credentials` 存入，通过 `get_token` 获取。
pub struct TelegramTokenManager {
    token: RwLock<Option<String>>,
}

impl TelegramTokenManager {
    pub fn new() -> Self {
        Self {
            token: RwLock::new(None),
        }
    }

    /// 设置 Bot Token
    pub async fn set_token(&self, token: String) {
        let mut t = self.token.write().await;
        *t = Some(token);
    }

    /// 清除 Token
    #[allow(dead_code)]
    pub async fn clear_token(&self) {
        let mut t = self.token.write().await;
        *t = None;
    }

    /// 检查是否已配置 Token
    pub async fn has_credentials(&self) -> bool {
        self.token.read().await.is_some()
    }

    /// 获取 Bot Token
    pub async fn get_token(&self) -> Result<String, AppError> {
        self.token
            .read()
            .await
            .clone()
            .ok_or_else(|| AppError::Unauthorized("未配置 Telegram Bot Token".into()))
    }

    /// 测试连接：调用 getMe API 验证 Token 有效性
    pub async fn test_connection(&self) -> Result<(), AppError> {
        let token = self.get_token().await?;
        let url = format!("https://api.telegram.org/bot{}/getMe", token);

        let resp = reqwest::get(&url)
            .await
            .map_err(|_| AppError::Gateway("测试连接网络错误: 连接失败".into()))?;

        #[derive(serde::Deserialize)]
        struct TelegramResp {
            ok: bool,
        }

        let body: TelegramResp = resp
            .json()
            .await
            .map_err(|e| AppError::Gateway(format!("解析响应失败: {}", e)))?;

        if !body.ok {
            return Err(AppError::Gateway("Token 无效".into()));
        }

        Ok(())
    }
}

// ── CredentialsManager trait impl ────────────────────────────

impl CredentialsManager for TelegramTokenManager {
    fn set_credentials(&self, credentials: serde_json::Value) -> Result<(), GatewayError> {
        let token = credentials["bot_token"]
            .as_str()
            .ok_or_else(|| GatewayError::InvalidCredentials("缺少 bot_token".into()))?
            .to_string();

        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                self.set_token(token).await;
            })
        });
        Ok(())
    }

    fn clear_credentials(&self) -> Result<(), GatewayError> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                self.clear_token().await;
            })
        });
        Ok(())
    }

    fn has_credentials(&self) -> bool {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async { self.has_credentials().await })
        })
    }

    fn test_connection(&self) -> Result<(), GatewayError> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                self.test_connection().await.map_err(|e| match e {
                    AppError::Unauthorized(_) => GatewayError::Unauthorized,
                    AppError::Gateway(msg) => GatewayError::Network(msg),
                    other => GatewayError::Network(other.to_string()),
                })
            })
        })
    }
}

impl Default for TelegramTokenManager {
    fn default() -> Self {
        Self::new()
    }
}
