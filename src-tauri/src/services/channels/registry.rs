//! 渠道注册中心：factory + descriptor 模式。
//!
//! 替代旧的硬编码 `init_channels()`：新增渠道只需注册一个 [`ChannelFactory`]，
//! 由 `build_all(enabled)` 按配置构造实例。

use crate::services::channels::{Channel, ChannelDescriptor};
use crate::services::core::SecretStore;
use crate::services::event_bus::EventBus;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// 渠道构造依赖：注入共享 HTTP 客户端、密钥存储、事件总线。
#[derive(Clone)]
pub struct ChannelDeps {
    pub http: reqwest::Client,
    pub secrets: Arc<dyn SecretStore>,
    pub event_bus: Arc<EventBus>,
}

impl ChannelDeps {
    pub fn new(
        http: reqwest::Client,
        secrets: Arc<dyn SecretStore>,
        event_bus: Arc<EventBus>,
    ) -> Self {
        Self {
            http,
            secrets,
            event_bus,
        }
    }
}

/// 渠道工厂：产出描述符 + 构造实例。
pub trait ChannelFactory: Send + Sync {
    fn descriptor(&self) -> &'static ChannelDescriptor;
    fn build(&self, deps: &ChannelDeps) -> Arc<dyn Channel>;
}

/// 渠道注册中心。
pub struct ChannelRegistry {
    factories: RwLock<HashMap<&'static str, Arc<dyn ChannelFactory>>>,
    instances: RwLock<HashMap<String, Arc<dyn Channel>>>,
}

impl ChannelRegistry {
    pub fn new() -> Self {
        Self {
            factories: RwLock::new(HashMap::new()),
            instances: RwLock::new(HashMap::new()),
        }
    }

    /// 注册工厂。
    pub fn register_factory(&self, factory: Arc<dyn ChannelFactory>) {
        let id = factory.descriptor().id;
        self.factories.write().unwrap().insert(id, factory);
    }

    /// 列出所有已注册工厂的描述符（供前端动态渲染）。
    pub fn list_descriptors(&self) -> Vec<&'static ChannelDescriptor> {
        self.factories
            .read()
            .unwrap()
            .values()
            .map(|f| f.descriptor())
            .collect()
    }

    /// 按 enabled 名单构造所有渠道实例。
    pub fn build_all(&self, deps: &ChannelDeps, enabled: &[String]) {
        let factories = self.factories.read().unwrap().clone();
        let mut instances = self.instances.write().unwrap();
        for name in enabled {
            if let Some(factory) = factories.get(name.as_str()) {
                let channel = factory.build(deps);
                instances.insert(name.clone(), channel);
            }
        }
    }

    /// 构造单个渠道实例（用于运行时启停单个渠道）。
    #[allow(dead_code)]
    pub fn build_one(&self, name: &str, deps: &ChannelDeps) -> Option<Arc<dyn Channel>> {
        let factory = self.factories.read().unwrap().get(name).cloned()?;
        let channel = factory.build(deps);
        self.instances
            .write()
            .unwrap()
            .insert(name.to_string(), channel.clone());
        Some(channel)
    }

    pub fn get(&self, channel: &str) -> Option<Arc<dyn Channel>> {
        self.instances.read().unwrap().get(channel).cloned()
    }

    pub fn list_channels(&self) -> Vec<String> {
        self.instances.read().unwrap().keys().cloned().collect()
    }

    /// 所有已构造实例（manager 启动用）。
    pub fn instances(&self) -> Vec<Arc<dyn Channel>> {
        self.instances.read().unwrap().values().cloned().collect()
    }
}

impl Default for ChannelRegistry {
    fn default() -> Self {
        Self::new()
    }
}
