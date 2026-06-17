//! SecretStore 实现：Memory（当前默认）+ Stronghold 适配器（待接入）。
//!
//! 此模块位于 services 层之外，允许 `use tauri::*` / `iota_stronghold`。
//!
//! # Stronghold 集成（TODO）
//!
//! `tauri-plugin-stronghold` 2.3.1 的托管 StrongholdCollection 是私有的——
//! Rust 代码无法通过 `app.state::<...>()` 获取。直接使用 `iota_stronghold`
//! 需要构建 KeyProvider、加载/创建 snapshot、管理 client 生命周期。
//! 本模块提供 `StrongholdSecretStore` 存根供编译，具体接入待后续补充。
//!
//! 当前默认实现为 `MemorySecretStore`，行为与重构前一致（凭证仅内存态）。
//! SecretStore trait 接缝已就绪，注入点：`ChannelDeps::new`。

use crate::services::core::SecretStore;
use crate::services::gateway::GatewayError;
use std::collections::HashMap;
use tokio::sync::RwLock;

/// 内存密钥存储（默认实现）。
///
/// 凭证仅在进程生命周期内存在。替代 `StrongholdSecretStore` 直至接入完成。
pub struct MemorySecretStore {
    data: RwLock<HashMap<String, Vec<u8>>>,
}

impl MemorySecretStore {
    pub fn new() -> Self {
        Self {
            data: RwLock::new(HashMap::new()),
        }
    }
}

#[async_trait::async_trait]
impl SecretStore for MemorySecretStore {
    async fn put(&self, key: &str, value: &[u8]) -> Result<(), GatewayError> {
        self.data
            .write()
            .await
            .insert(key.to_string(), value.to_vec());
        Ok(())
    }

    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, GatewayError> {
        Ok(self.data.read().await.get(key).cloned())
    }

    async fn delete(&self, key: &str) -> Result<Option<Vec<u8>>, GatewayError> {
        Ok(self.data.write().await.remove(key))
    }
}

// ── StrongholdSecretStore 存根 ────────────────────────────────────────

/// Stronghold 密钥存储。
///
/// # 状态
///
/// 编译通过但未接入 `iota_stronghold` KeyProvider / snapshot 管理。
/// `put` / `get` / `delete` 返回 `Unsupported("StrongholdSecretStore 未接入")`。
///
/// 接入步骤：
/// 1. 构建 `iota_stronghold::KeyProvider`（密码派生 / macOS Keychain）。
/// 2. `Stronghold::default()` → `load_client_from_snapshot("mindclaw", &kp, snapshot_path)`。
/// 3. `client.store().insert(key, value, None)` / `store.get(key)` / `store.delete(key)`。
/// 4. 定期 `write_to_snapshot` 持久化。
#[allow(dead_code)]
pub struct StrongholdSecretStore;

#[allow(dead_code)]
impl StrongholdSecretStore {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl SecretStore for StrongholdSecretStore {
    async fn put(&self, _key: &str, _value: &[u8]) -> Result<(), GatewayError> {
        Err(GatewayError::Unsupported(
            "StrongholdSecretStore 未接入，请使用 MemorySecretStore".into(),
        ))
    }

    async fn get(&self, _key: &str) -> Result<Option<Vec<u8>>, GatewayError> {
        Err(GatewayError::Unsupported(
            "StrongholdSecretStore 未接入，请使用 MemorySecretStore".into(),
        ))
    }

    async fn delete(&self, _key: &str) -> Result<Option<Vec<u8>>, GatewayError> {
        Err(GatewayError::Unsupported(
            "StrongholdSecretStore 未接入，请使用 MemorySecretStore".into(),
        ))
    }
}
