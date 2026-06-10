use super::{ChannelGateway, GatewayError};
use crate::services::message_bus::ChannelMessage;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// 渠道注册中心
///
/// 持有所有已注册的渠道 Gateway，提供统一的轮询和消息发送接口。
pub struct GatewayRegistry {
    /// 渠道映射：channel_name → ChannelGateway
    gateways: RwLock<HashMap<String, Arc<dyn ChannelGateway>>>,
    /// 渠道排序列表（保持注册顺序，用于轮询）
    order: RwLock<Vec<String>>,
}

impl GatewayRegistry {
    pub fn new() -> Self {
        Self {
            gateways: RwLock::new(HashMap::new()),
            order: RwLock::new(Vec::new()),
        }
    }

    /// 注册一个渠道 Gateway（同步，可在 AppState::new() 中调用）
    pub fn register(&self, gateway: Arc<dyn ChannelGateway>) {
        let name = gateway.channel_name().to_string();
        let mut gateways = self.gateways.write().unwrap();
        let mut order = self.order.write().unwrap();

        if !gateways.contains_key(&name) {
            order.push(name.clone());
        }
        gateways.insert(name, gateway);
    }

    /// 获取指定渠道的 Gateway
    pub async fn get(&self, channel: &str) -> Option<Arc<dyn ChannelGateway>> {
        self.gateways.read().unwrap().get(channel).cloned()
    }

    /// 列出所有已注册渠道的名称
    pub async fn list_channels(&self) -> Vec<String> {
        self.order.read().unwrap().clone()
    }

    /// 轮询所有已注册渠道的消息
    ///
    /// 并行拉取每个渠道的消息，返回 `Vec<(channel_name, Vec<ChannelMessage>)>`。
    #[allow(dead_code)]
    pub async fn poll_all(
        &self,
        page_size: i32,
    ) -> Vec<(String, Result<Vec<ChannelMessage>, GatewayError>)> {
        let channels: Vec<Arc<dyn ChannelGateway>> = {
            let order = self.order.read().unwrap();
            let gateways = self.gateways.read().unwrap();
            order
                .iter()
                .filter_map(|name| gateways.get(name).cloned())
                .collect()
        };

        let mut handles = Vec::new();
        for gw in channels {
            let gw = gw.clone();
            handles.push(tokio::spawn(async move {
                let name = gw.channel_name().to_string();
                let result = gw.poll_messages(page_size, None).map(|r| r.0);
                (name, result)
            }));
        }

        let mut results = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(r) => results.push(r),
                Err(e) => {
                    // spawn error — shouldn't happen, but log it
                    eprintln!("GatewayRegistry: poll_all spawn error: {}", e);
                }
            }
        }
        results
    }

    /// 向指定渠道发送消息
    pub async fn send_message(
        &self,
        channel: &str,
        conversation_id: &str,
        content: &str,
        reply_to: Option<&str>,
    ) -> Result<(), GatewayError> {
        let gw = self
            .get(channel)
            .await
            .ok_or_else(|| GatewayError::Unsupported(format!("未知渠道: {}", channel)))?;
        gw.send_message(conversation_id, content, reply_to)
    }

    /// 设置指定渠道的凭证
    pub async fn set_credentials(
        &self,
        channel: &str,
        credentials: serde_json::Value,
    ) -> Result<(), GatewayError> {
        let gw = self
            .get(channel)
            .await
            .ok_or_else(|| GatewayError::Unsupported(format!("未知渠道: {}", channel)))?;
        gw.credentials().set_credentials(credentials)
    }

    /// 检查指定渠道是否已配置凭证
    pub async fn has_credentials(&self, channel: &str) -> bool {
        self.get(channel)
            .await
            .map(|gw| gw.credentials().has_credentials())
            .unwrap_or(false)
    }

    /// 测试指定渠道连接
    pub async fn test_connection(&self, channel: &str) -> Result<(), GatewayError> {
        let gw = self
            .get(channel)
            .await
            .ok_or_else(|| GatewayError::Unsupported(format!("未知渠道: {}", channel)))?;
        gw.credentials().test_connection()
    }
}

impl Default for GatewayRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::gateway::CredentialsManager;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Mutex;

    struct FakeCredentials {
        has_credentials: Mutex<bool>,
        tested: AtomicBool,
    }

    impl FakeCredentials {
        fn new() -> Self {
            Self {
                has_credentials: Mutex::new(false),
                tested: AtomicBool::new(false),
            }
        }
    }

    impl CredentialsManager for FakeCredentials {
        fn set_credentials(&self, _credentials: serde_json::Value) -> Result<(), GatewayError> {
            *self.has_credentials.lock().unwrap() = true;
            Ok(())
        }

        fn clear_credentials(&self) -> Result<(), GatewayError> {
            *self.has_credentials.lock().unwrap() = false;
            Ok(())
        }

        fn has_credentials(&self) -> bool {
            *self.has_credentials.lock().unwrap()
        }

        fn test_connection(&self) -> Result<(), GatewayError> {
            self.tested.store(true, Ordering::SeqCst);
            Ok(())
        }
    }

    struct FakeGateway {
        name: String,
        credentials: FakeCredentials,
        messages: Vec<ChannelMessage>,
        sent_messages: Mutex<Vec<String>>,
    }

    impl FakeGateway {
        fn new(name: &str, message_id: &str) -> Self {
            Self {
                name: name.to_string(),
                credentials: FakeCredentials::new(),
                messages: vec![message(message_id)],
                sent_messages: Mutex::new(Vec::new()),
            }
        }
    }

    impl ChannelGateway for FakeGateway {
        fn channel_name(&self) -> &str {
            &self.name
        }

        fn poll_messages(
            &self,
            _page_size: i32,
            _page_token: Option<&str>,
        ) -> Result<(Vec<ChannelMessage>, Option<String>), GatewayError> {
            Ok((self.messages.clone(), None))
        }

        fn send_message(
            &self,
            _conversation_id: &str,
            content: &str,
            _reply_to: Option<&str>,
        ) -> Result<(), GatewayError> {
            self.sent_messages.lock().unwrap().push(content.to_string());
            Ok(())
        }

        fn credentials(&self) -> &dyn CredentialsManager {
            &self.credentials
        }
    }

    fn message(id: &str) -> ChannelMessage {
        ChannelMessage {
            message_id: id.to_string(),
            channel: "feishu".to_string(),
            conversation_id: "chat-1".to_string(),
            sender_id: "user-1".to_string(),
            sender_name: "User".to_string(),
            content: "hello".to_string(),
            timestamp: 1,
            is_reply: false,
            reply_to: None,
        }
    }

    #[tokio::test]
    async fn register_get_and_list_channels_keep_registration_order() {
        let registry = GatewayRegistry::new();

        registry.register(Arc::new(FakeGateway::new("feishu", "msg-1")));
        registry.register(Arc::new(FakeGateway::new("telegram", "msg-2")));

        assert!(registry.get("feishu").await.is_some());
        assert_eq!(registry.list_channels().await, vec!["feishu", "telegram"]);
    }

    #[tokio::test]
    async fn register_replaces_existing_gateway_without_duplicating_order() {
        let registry = GatewayRegistry::new();

        registry.register(Arc::new(FakeGateway::new("feishu", "old")));
        registry.register(Arc::new(FakeGateway::new("feishu", "new")));

        let gateway = registry.get("feishu").await.unwrap();
        let (messages, _) = gateway.poll_messages(10, None).unwrap();
        assert_eq!(messages[0].message_id, "new");
        assert_eq!(registry.list_channels().await, vec!["feishu"]);
    }

    #[tokio::test]
    async fn send_message_returns_unsupported_for_unknown_channel() {
        let registry = GatewayRegistry::new();

        let error = registry
            .send_message("missing", "chat-1", "hello", None)
            .await
            .unwrap_err();

        assert!(matches!(error, GatewayError::Unsupported(_)));
    }

    #[tokio::test]
    async fn credentials_are_proxied_to_target_gateway() {
        let registry = GatewayRegistry::new();
        registry.register(Arc::new(FakeGateway::new("feishu", "msg-1")));

        assert!(!registry.has_credentials("feishu").await);
        registry
            .set_credentials("feishu", serde_json::json!({ "token": "secret" }))
            .await
            .unwrap();
        assert!(registry.has_credentials("feishu").await);
        registry.test_connection("feishu").await.unwrap();
    }

    #[tokio::test]
    async fn credential_methods_return_unsupported_for_unknown_channel() {
        let registry = GatewayRegistry::new();

        let set_error = registry
            .set_credentials("missing", serde_json::json!({}))
            .await
            .unwrap_err();
        let test_error = registry.test_connection("missing").await.unwrap_err();

        assert!(matches!(set_error, GatewayError::Unsupported(_)));
        assert!(matches!(test_error, GatewayError::Unsupported(_)));
        assert!(!registry.has_credentials("missing").await);
    }

    #[tokio::test]
    async fn poll_all_returns_result_for_each_registered_channel() {
        let registry = GatewayRegistry::new();
        registry.register(Arc::new(FakeGateway::new("feishu", "msg-1")));
        registry.register(Arc::new(FakeGateway::new("telegram", "msg-2")));

        let results = registry.poll_all(10).await;

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, "feishu");
        assert_eq!(results[0].1.as_ref().unwrap()[0].message_id, "msg-1");
        assert_eq!(results[1].0, "telegram");
        assert_eq!(results[1].1.as_ref().unwrap()[0].message_id, "msg-2");
    }
}
