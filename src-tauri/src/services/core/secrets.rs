//! 密钥存储抽象。
//!
//! service 层只依赖 [`SecretStore`] trait，不引入 `tauri::*`，符合分层原则。
//! 具体实现（Stronghold / SQLite 加密）在更高层（commands / lib.rs）注入。

use crate::services::gateway::GatewayError;

/// 密钥存储 trait：渠道凭证的加密读写边界。
#[async_trait::async_trait]
pub trait SecretStore: Send + Sync {
    /// 写入（覆盖）一个密钥。
    async fn put(&self, key: &str, value: &[u8]) -> Result<(), GatewayError>;

    /// 读取一个密钥，不存在返回 `None`。
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, GatewayError>;

    /// 删除一个密钥，返回旧值。
    #[allow(dead_code)]
    async fn delete(&self, key: &str) -> Result<Option<Vec<u8>>, GatewayError>;

    /// 便捷：读取并反序列化为 JSON Value。
    async fn get_json(&self, key: &str) -> Result<Option<serde_json::Value>, GatewayError> {
        match self.get(key).await? {
            Some(bytes) => serde_json::from_slice(&bytes)
                .map(Some)
                .map_err(|e| GatewayError::Network(format!("密钥反序列化失败: {e}"))),
            None => Ok(None),
        }
    }

    /// 便捷：序列化 JSON Value 并写入。
    async fn put_json(&self, key: &str, value: &serde_json::Value) -> Result<(), GatewayError> {
        let bytes = serde_json::to_vec(value)
            .map_err(|e| GatewayError::Network(format!("密钥序列化失败: {e}")))?;
        self.put(key, &bytes).await
    }
}

/// 凭证 key 规范：`channel:{id}:credentials`。
pub fn credential_key(channel: &str) -> String {
    format!("channel:{channel}:credentials")
}
