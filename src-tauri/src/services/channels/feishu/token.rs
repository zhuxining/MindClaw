//! 飞书凭证与 tenant_access_token 管理（async，凭证经 SecretStore 持久化）。

use crate::services::core::{credential_key, SecretStore};
use crate::services::gateway::GatewayError;
use std::sync::Arc;
use tokio::sync::RwLock;

const TOKEN_API: &str = "https://open.feishu.cn/open-apis/auth/v3/tenant_access_token/internal";
/// 提前刷新余量（秒）。
const REFRESH_SKEW_SECS: i64 = 300;
/// 飞书 token 无效业务码。
pub const INVALID_TOKEN_CODE: i64 = 99_991_663;

#[derive(Debug, Clone)]
struct CachedToken {
    value: String,
    expires_at: i64,
}

#[derive(Debug, serde::Deserialize)]
struct TokenResponse {
    code: i32,
    #[serde(default)]
    msg: Option<String>,
    #[serde(rename = "tenant_access_token")]
    tenant_access_token: Option<String>,
    #[serde(default)]
    expire: Option<i64>,
}

/// 飞书凭证 + token 缓存。
pub struct FeishuCredentials {
    app_id: RwLock<Option<String>>,
    app_secret: RwLock<Option<String>>,
    token: RwLock<Option<CachedToken>>,
    http: reqwest::Client,
}

impl FeishuCredentials {
    pub fn new(http: reqwest::Client) -> Self {
        Self {
            app_id: RwLock::new(None),
            app_secret: RwLock::new(None),
            token: RwLock::new(None),
            http,
        }
    }

    /// 从 SecretStore 载入凭证到内存（start 时调用）。
    pub async fn load(&self, store: &dyn SecretStore) -> Result<(), GatewayError> {
        if let Some(creds) = store.get_json(&credential_key("feishu")).await? {
            let app_id = creds["app_id"].as_str().map(|s| s.to_string());
            let app_secret = creds["app_secret"].as_str().map(|s| s.to_string());
            *self.app_id.write().await = app_id.filter(|s| !s.is_empty());
            *self.app_secret.write().await = app_secret.filter(|s| !s.is_empty());
            *self.token.write().await = None;
        }
        Ok(())
    }

    pub async fn app_id(&self) -> Option<String> {
        self.app_id.read().await.clone()
    }

    pub async fn app_secret(&self) -> Option<String> {
        self.app_secret.read().await.clone()
    }

    pub async fn has_credentials(&self) -> bool {
        self.app_id.read().await.is_some() && self.app_secret.read().await.is_some()
    }

    /// 获取有效 tenant_access_token（自动刷新）。
    pub async fn get_token(&self) -> Result<String, GatewayError> {
        {
            let cached = self.token.read().await;
            if let Some(ct) = cached.as_ref() {
                let now = chrono::Utc::now().timestamp();
                if ct.expires_at - now > REFRESH_SKEW_SECS {
                    return Ok(ct.value.clone());
                }
            }
        }
        self.refresh_token().await
    }

    /// 作废缓存 token（API 报无效时调用，强制下次刷新）。
    pub async fn invalidate(&self) {
        *self.token.write().await = None;
    }

    async fn refresh_token(&self) -> Result<String, GatewayError> {
        let app_id = self
            .app_id
            .read()
            .await
            .clone()
            .ok_or(GatewayError::Unauthorized)?;
        let app_secret = self
            .app_secret
            .read()
            .await
            .clone()
            .ok_or(GatewayError::Unauthorized)?;

        let resp = self
            .http
            .post(TOKEN_API)
            .json(&serde_json::json!({ "app_id": app_id, "app_secret": app_secret }))
            .send()
            .await
            .map_err(|_| GatewayError::Network("获取 token 网络错误: 请求失败".into()))?;

        let body: TokenResponse = resp
            .json()
            .await
            .map_err(|e| GatewayError::Network(format!("解析 token 响应失败: {e}")))?;

        if body.code != 0 {
            return Err(GatewayError::InvalidCredentials(
                body.msg.unwrap_or_else(|| "获取 token 失败".into()),
            ));
        }

        let value = body
            .tenant_access_token
            .ok_or_else(|| GatewayError::Network("token 响应中无 access_token".into()))?;
        let expires_in = body.expire.unwrap_or(7200);
        let expires_at = chrono::Utc::now().timestamp() + expires_in;

        *self.token.write().await = Some(CachedToken {
            value: value.clone(),
            expires_at,
        });
        Ok(value)
    }
}

#[async_trait::async_trait]
impl crate::services::channels::CredentialsManager for FeishuCredentials {
    async fn set_credentials(
        &self,
        credentials: serde_json::Value,
        store: &dyn SecretStore,
    ) -> Result<(), GatewayError> {
        let app_id = credentials["app_id"]
            .as_str()
            .ok_or_else(|| GatewayError::InvalidCredentials("缺少 app_id".into()))?
            .to_string();
        let app_secret = credentials["app_secret"]
            .as_str()
            .ok_or_else(|| GatewayError::InvalidCredentials("缺少 app_secret".into()))?
            .to_string();

        store
            .put_json(&credential_key("feishu"), &credentials)
            .await?;

        *self.app_id.write().await = Some(app_id);
        *self.app_secret.write().await = Some(app_secret);
        *self.token.write().await = None;
        Ok(())
    }

    async fn clear_credentials(&self, store: &dyn SecretStore) -> Result<(), GatewayError> {
        store.delete(&credential_key("feishu")).await?;
        *self.app_id.write().await = None;
        *self.app_secret.write().await = None;
        *self.token.write().await = None;
        Ok(())
    }

    async fn has_credentials(&self, store: &dyn SecretStore) -> bool {
        store
            .get_json(&credential_key("feishu"))
            .await
            .ok()
            .flatten()
            .map(|c| {
                c.get("app_id").and_then(|v| v.as_str()).is_some()
                    && c.get("app_secret").and_then(|v| v.as_str()).is_some()
            })
            .unwrap_or(false)
    }

    async fn test_connection(&self) -> Result<(), GatewayError> {
        self.refresh_token().await.map(|_| ())
    }
}

/// 共享凭证别名。
#[allow(dead_code)]
pub type TokenManager = FeishuCredentials;

#[allow(dead_code)]
pub fn arc(http: reqwest::Client) -> Arc<FeishuCredentials> {
    Arc::new(FeishuCredentials::new(http))
}
