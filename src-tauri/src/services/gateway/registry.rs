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
