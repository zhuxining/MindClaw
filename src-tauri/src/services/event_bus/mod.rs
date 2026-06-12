mod types;

pub use types::RuntimeEvent;

use tokio::sync::broadcast;

const DEFAULT_CAPACITY: usize = 256;

/// Runtime 事件总线。
///
/// 这是 Pub/Sub 事件中心，只用于 UI、日志、审计和监控订阅。
pub struct EventBus {
    sender: broadcast::Sender<RuntimeEvent>,
}

impl EventBus {
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    /// 发布事件，返回当前成功接收该事件的订阅者数量。
    ///
    /// 没有订阅者时返回 0，不视为错误。
    pub fn publish(&self, event: RuntimeEvent) -> usize {
        self.sender.send(event).unwrap_or(0)
    }

    /// 订阅后续事件。
    #[allow(dead_code)]
    pub fn subscribe(&self) -> broadcast::Receiver<RuntimeEvent> {
        self.sender.subscribe()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::broadcast::error::TryRecvError;

    #[test]
    fn publish_succeeds_without_subscribers() {
        let bus = EventBus::new();

        assert_eq!(bus.publish(RuntimeEvent::RuntimeStarted), 0);
    }

    #[tokio::test]
    async fn publish_delivers_event_to_all_subscribers() {
        let bus = EventBus::new();
        let mut first = bus.subscribe();
        let mut second = bus.subscribe();

        assert_eq!(bus.publish(RuntimeEvent::RuntimeStarted), 2);

        assert_eq!(first.recv().await.unwrap(), RuntimeEvent::RuntimeStarted);
        assert_eq!(second.recv().await.unwrap(), RuntimeEvent::RuntimeStarted);
    }

    #[test]
    fn lagging_subscriber_does_not_break_publish() {
        let bus = EventBus::with_capacity(1);
        let mut lagging = bus.subscribe();
        let mut current = bus.subscribe();

        assert_eq!(bus.publish(RuntimeEvent::RuntimeStarted), 2);
        assert_eq!(current.try_recv().unwrap(), RuntimeEvent::RuntimeStarted);

        assert_eq!(bus.publish(RuntimeEvent::RuntimeStopped), 2);

        assert!(matches!(lagging.try_recv(), Err(TryRecvError::Lagged(1))));
        assert_eq!(current.try_recv().unwrap(), RuntimeEvent::RuntimeStopped);
    }
}
