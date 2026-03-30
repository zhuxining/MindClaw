# MindClaw 技术架构设计 — Channel 层

> 完整架构文档索引见 [README.md](./README.md) | Agent 核心见 [03-agent-loop.md](./03-agent-loop.md)

## Channel 层 — 统一消息通道

Channel 是所有通信平台的抽象接口。无论消息来自桌面 UI、Telegram 还是 Feishu，Channel 直接产出 `InboundMessage` 推入 MessageBus（Desktop Channel 由 `send_message` command 代为生成 `InboundMessage`，其余 Channel 在 `listen()` 中生成）。

> **设计决策**：不再定义独立的 `ChannelMessage` 结构。Channel 直接使用 `InboundMessage`（定义在 `bus/events.rs`），`request_id` 由 Channel/Command 层生成，`session_id` 可选（新会话时为 None）。

```rust
// src-tauri/src/channels/traits.rs

pub struct SendMessage {
    pub content: String,
    pub recipient: String,
    pub metadata: Option<serde_json::Value>,
}

pub enum ChannelSource {
    Desktop,
    Telegram,
    Feishu,
    Webhook,
}

#[async_trait]
pub trait Channel: Send + Sync {
    fn name(&self) -> &str;
    fn source(&self) -> ChannelSource;
    async fn send(&self, message: OutboundMessage) -> Result<(), AppError>;
    async fn listen(&self, bus: Arc<MessageBus>) -> Result<(), AppError>;
    fn supports_streaming(&self) -> bool { false }
    async fn send_chunk(&self, _chunk: &str, _session_id: &str) -> Result<(), AppError> { Ok(()) }
    async fn start_typing(&self) -> Result<(), AppError> { Ok(()) }
    async fn stop_typing(&self) -> Result<(), AppError> { Ok(()) }
}
```

### Channel 实现一览

| Channel | 传输方式 | 流式支持 | 入站机制 | Phase |
|---------|---------|---------|---------|-------|
| **Desktop** | Tauri IPC invoke + Event emit | Yes | Tauri command 桥接推入 Bus（listen 为空实现） | MVP |
| **Telegram** | HTTP API / Long polling | No | getUpdates 或 Webhook → Bus | Phase 1 后期 |
| **Feishu** | HTTP API / Webhook | No | Webhook → Bus | Phase 2 |
| **Webhook** | HTTP POST → Bus | No | Gateway 接收 → Bus | Phase 1 后期 |
