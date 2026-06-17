//! Telegram Bot Token 凭证管理（async，经 SecretStore 持久化）。

use crate::services::core::{credential_key, SecretStore};
use crate::services::gateway::GatewayError;
use tokio::sync::RwLock;

/// Telegram 凭证。
pub struct TelegramCredentials {
    token: RwLock<Option<String>>,
}

impl TelegramCredentials {
    pub fn new() -> Self {
        Self {
            token: RwLock::new(None),
        }
    }

    /// 从 SecretStore 载入。
    pub async fn load(&self, store: &dyn SecretStore) -> Result<(), GatewayError> {
        if let Some(creds) = store.get_json(&credential_key("telegram")).await? {
            let token = creds["bot_token"].as_str().map(|s| s.to_string());
            *self.token.write().await = token.filter(|s| !s.is_empty());
        }
        Ok(())
    }

    pub async fn has_credentials(&self) -> bool {
        self.token.read().await.is_some()
    }

    pub async fn get_token(&self) -> Result<String, GatewayError> {
        self.token
            .read()
            .await
            .clone()
            .ok_or(GatewayError::Unauthorized)
    }
}

impl Default for TelegramCredentials {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl crate::services::channels::CredentialsManager for TelegramCredentials {
    async fn set_credentials(
        &self,
        credentials: serde_json::Value,
        store: &dyn SecretStore,
    ) -> Result<(), GatewayError> {
        let token = credentials["bot_token"]
            .as_str()
            .ok_or_else(|| GatewayError::InvalidCredentials("缺少 bot_token".into()))?
            .to_string();
        store
            .put_json(&credential_key("telegram"), &credentials)
            .await?;
        *self.token.write().await = Some(token);
        Ok(())
    }

    async fn clear_credentials(&self, store: &dyn SecretStore) -> Result<(), GatewayError> {
        store.delete(&credential_key("telegram")).await?;
        *self.token.write().await = None;
        Ok(())
    }

    async fn has_credentials(&self, store: &dyn SecretStore) -> bool {
        store
            .get_json(&credential_key("telegram"))
            .await
            .ok()
            .flatten()
            .and_then(|c| {
                c.get("bot_token")
                    .and_then(|v| v.as_str())
                    .map(|s| !s.is_empty())
            })
            .unwrap_or(false)
    }

    async fn test_connection(&self) -> Result<(), GatewayError> {
        let token = self.get_token().await?;
        let resp = reqwest::get(format!("https://api.telegram.org/bot{token}/getMe"))
            .await
            .map_err(|_| GatewayError::Network("测试连接网络错误: 连接失败".into()))?;
        #[derive(serde::Deserialize)]
        struct R {
            ok: bool,
        }
        let body: R = resp
            .json()
            .await
            .map_err(|e| GatewayError::Network(format!("解析响应失败: {e}")))?;
        if !body.ok {
            return Err(GatewayError::InvalidCredentials("Token 无效".into()));
        }
        Ok(())
    }
}

#[allow(dead_code)]
pub type TelegramTokenManager = TelegramCredentials;
