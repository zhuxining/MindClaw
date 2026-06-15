use crate::error::AppError;
use crate::services::channels::CredentialsManager;
use crate::services::gateway::GatewayError;
use tokio::sync::RwLock;

/// Token 管理器：负责飞书 tenant_access_token 的获取和缓存
pub struct TokenManager {
    app_id: RwLock<Option<String>>,
    app_secret: RwLock<Option<String>>,
    token: RwLock<Option<CachedToken>>,
    http_client: reqwest::Client,
}

#[derive(Debug, Clone)]
struct CachedToken {
    value: String,
    expires_at: i64, // Unix timestamp
}

/// 飞书获取 token 的 API 响应
#[derive(Debug, serde::Deserialize)]
struct FeishuTokenResponse {
    code: i32,
    msg: Option<String>,
    #[serde(rename = "tenant_access_token")]
    tenant_access_token: Option<String>,
    expire: Option<i64>,
}

impl TokenManager {
    pub fn new() -> Self {
        Self {
            app_id: RwLock::new(None),
            app_secret: RwLock::new(None),
            token: RwLock::new(None),
            http_client: reqwest::Client::new(),
        }
    }

    /// 设置飞书应用凭证
    pub async fn set_credentials(&self, app_id: String, app_secret: String) {
        let mut id = self.app_id.write().await;
        *id = Some(app_id);
        let mut secret = self.app_secret.write().await;
        *secret = Some(app_secret);
        // 清除旧 token 强制刷新
        let mut token = self.token.write().await;
        *token = None;
    }

    /// 清除凭证
    #[allow(dead_code)]
    pub async fn clear_credentials(&self) {
        let mut id = self.app_id.write().await;
        *id = None;
        let mut secret = self.app_secret.write().await;
        *secret = None;
        let mut token = self.token.write().await;
        *token = None;
    }

    /// 检查是否已配置凭证
    pub async fn has_credentials(&self) -> bool {
        self.app_id.read().await.is_some() && self.app_secret.read().await.is_some()
    }

    /// 获取有效的 access token（自动刷新）
    pub async fn get_token(&self) -> Result<String, AppError> {
        // 检查缓存的 token 是否有效
        {
            let cached = self.token.read().await;
            if let Some(ref ct) = *cached {
                let now = chrono::Utc::now().timestamp();
                // 提前 5 分钟刷新
                if ct.expires_at - now > 300 {
                    return Ok(ct.value.clone());
                }
            }
        }

        // 需要刷新 token
        self.refresh_token().await
    }

    async fn refresh_token(&self) -> Result<String, AppError> {
        let app_id = self
            .app_id
            .read()
            .await
            .clone()
            .ok_or_else(|| AppError::Unauthorized("未配置飞书 App ID".into()))?;

        let app_secret = self
            .app_secret
            .read()
            .await
            .clone()
            .ok_or_else(|| AppError::Unauthorized("未配置飞书 App Secret".into()))?;

        let resp = self
            .http_client
            .post("https://open.feishu.cn/open-apis/auth/v3/tenant_access_token/internal")
            .json(&serde_json::json!({
                "app_id": app_id,
                "app_secret": app_secret,
            }))
            .send()
            .await
            .map_err(|e| AppError::FeishuGateway(format!("获取 token 网络错误: {}", e)))?;

        let body: FeishuTokenResponse = resp
            .json()
            .await
            .map_err(|e| AppError::FeishuGateway(format!("解析 token 响应失败: {}", e)))?;

        if body.code != 0 {
            return Err(AppError::FeishuGateway(format!(
                "获取 token 失败: {}",
                body.msg.as_deref().unwrap_or("未知错误")
            )));
        }

        let token_value = body
            .tenant_access_token
            .ok_or_else(|| AppError::FeishuGateway("token 响应中无 access_token".into()))?;

        let expires_in = body.expire.unwrap_or(7200);
        let expires_at = chrono::Utc::now().timestamp() + expires_in;

        let mut cached = self.token.write().await;
        *cached = Some(CachedToken {
            value: token_value.clone(),
            expires_at,
        });

        Ok(token_value)
    }

    /// 测试连接：尝试获取 token 验证凭证有效性
    pub async fn test_connection(&self) -> Result<(), AppError> {
        self.refresh_token().await.map(|_| ())
    }
}

// ── CredentialsManager trait impl ────────────────────────────

impl CredentialsManager for TokenManager {
    fn set_credentials(&self, credentials: serde_json::Value) -> Result<(), GatewayError> {
        let app_id = credentials["app_id"]
            .as_str()
            .ok_or_else(|| GatewayError::InvalidCredentials("缺少 app_id".into()))?
            .to_string();
        let app_secret = credentials["app_secret"]
            .as_str()
            .ok_or_else(|| GatewayError::InvalidCredentials("缺少 app_secret".into()))?
            .to_string();

        // Use block_on because trait methods are not async
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                self.set_credentials(app_id, app_secret).await;
            })
        });
        Ok(())
    }

    fn clear_credentials(&self) -> Result<(), GatewayError> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                self.clear_credentials().await;
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
                    AppError::Unauthorized(_msg) => GatewayError::Unauthorized,
                    AppError::FeishuGateway(msg) => GatewayError::Network(msg),
                    other => GatewayError::Network(other.to_string()),
                })
            })
        })
    }
}

impl Default for TokenManager {
    fn default() -> Self {
        Self::new()
    }
}
