# Channels — 多通道架构

> 配置驱动的多平台消息通道支持

## 设计哲学

Channels 是 MindClaw 与外部世界交互的接口，遵循以下原则：

- **配置驱动**：通过配置文件加载通道，无需修改代码
- **多实例支持**：同一平台可配置多个实例（如多个 Telegram Bot）
- **统一抽象**：所有通道实现相同的 `Channel` trait
- **消息总线解耦**：通道只与 MessageBus 交互，不直接访问 Agent

## 架构位置

```
┌─────────────────────────────────────────────────────────┐
│                    Channel Layer                        │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐     │
│  │Desktop  │ │Telegram │ │ Feishu  │ │  ...    │     │
│  │(Tauri)  │ │  Bot    │ │  Bot    │ │         │     │
│  └────┬────┘ └────┬────┘ └────┬────┘ └────┬────┘     │
│       │           │           │           │           │
│       └───────────┴───────────┴───────────┘           │
│                   │                                     │
│              ChannelManager                           │
│                   │                                     │
│              MessageBus                                 │
└─────────────────────────────────────────────────────────┘
```

## 核心抽象

```rust
/// 通道抽象基类
#[async_trait]
pub trait Channel: Send + Sync {
    /// 通道名称
    fn name(&self) -> &str;
    
    /// 通道类型
    fn channel_type(&self) -> ChannelType;
    
    /// 启动通道（长生命周期监听器）
    async fn start(&mut self) -> Result<()>;
    
    /// 停止通道
    async fn stop(&mut self) -> Result<()>;
    
    /// 发送消息
    async fn send(&self, message: &OutboundMessage) -> Result<()>;
    
    /// 检查发送者权限
    fn check_sender(&self, sender_id: &str) -> bool;
}

/// 通道类型
pub enum ChannelType {
    Desktop,    // Tauri 桌面应用
    Telegram,   // Telegram Bot
    Feishu,     // 飞书 Bot
    Discord,    // Discord Bot
    Slack,      // Slack Bot
    Email,      // 邮件
    WebSocket,  // WebSocket 实时通信
    Http,       // HTTP Webhook
}

/// 通道配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelConfig {
    /// 通道类型
    pub channel_type: ChannelType,
    /// 通道名称（唯一标识）
    pub name: String,
    /// 是否启用
    pub enabled: bool,
    /// 发送者白名单（空表示允许所有）
    pub whitelist: Vec<String>,
    /// 平台特定配置
    pub platform_config: PlatformConfig,
}

/// 平台特定配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PlatformConfig {
    Desktop(DesktopConfig),
    Telegram(TelegramConfig),
    Feishu(FeishuConfig),
    Discord(DiscordConfig),
    // ...
}

/// Desktop 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesktopConfig {
    /// 窗口标题
    pub window_title: String,
}

/// Telegram 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramConfig {
    /// Bot Token
    pub token: String,
    /// 代理设置
    pub proxy: Option<String>,
}

/// 飞书配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeishuConfig {
    /// App ID
    pub app_id: String,
    /// App Secret
    pub app_secret: String,
    /// Encrypt Key
    pub encrypt_key: Option<String>,
}
```

## ChannelManager

```rust
pub struct ChannelManager {
    /// 消息总线
    bus: Arc<MessageBus>,
    /// 已注册的通道
    channels: HashMap<String, Box<dyn Channel>>,
    /// 出站调度器
    dispatcher: Option<JoinHandle<()>>,
}

impl ChannelManager {
    pub fn new(bus: Arc<MessageBus>) -> Self {
        Self {
            bus,
            channels: HashMap::new(),
            dispatcher: None,
        }
    }
    
    /// 从配置加载通道
    pub async fn load_from_config(&mut self, configs: Vec<ChannelConfig>) -> Result<()> {
        for config in configs {
            if !config.enabled {
                continue;
            }
            
            let channel: Box<dyn Channel> = match config.channel_type {
                ChannelType::Desktop => {
                    Box::new(DesktopChannel::new(config, self.bus.clone()))
                }
                ChannelType::Telegram => {
                    Box::new(TelegramChannel::new(config, self.bus.clone()))
                }
                ChannelType::Feishu => {
                    Box::new(FeishuChannel::new(config, self.bus.clone()))
                }
                ChannelType::Discord => {
                    Box::new(DiscordChannel::new(config, self.bus.clone()))
                }
                // ...
            };
            
            self.channels.insert(config.name.clone(), channel);
        }
        
        Ok(())
    }
    
    /// 启动所有通道
    pub async fn start_all(&mut self) -> Result<()> {
        // 启动每个通道
        for (_, channel) in &mut self.channels {
            channel.start().await?;
        }
        
        // 启动出站调度器
        self.start_dispatcher();
        
        Ok(())
    }
    
    /// 停止所有通道
    pub async fn stop_all(&mut self) -> Result<()> {
        // 停止出站调度器
        if let Some(handle) = self.dispatcher.take() {
            handle.abort();
        }
        
        // 停止每个通道
        for (_, channel) in &mut self.channels {
            channel.stop().await?;
        }
        
        Ok(())
    }
    
    /// 启动出站调度器
    fn start_dispatcher(&mut self) {
        let bus = self.bus.clone();
        let channels = self.channels.clone();
        
        self.dispatcher = Some(tokio::spawn(async move {
            loop {
                // 消费出站消息
                if let Some(message) = bus.consume_outbound().await {
                    // 根据 session_key 路由到正确的通道
                    let channel_name = Self::resolve_channel(&message.session_key);
                    
                    if let Some(channel) = channels.get(&channel_name) {
                        if let Err(e) = channel.send(&message).await {
                            log::error!("Failed to send message: {}", e);
                        }
                    }
                }
            }
        }));
    }
    
    /// 从 session_key 解析通道名称
    fn resolve_channel(session_key: &str) -> String {
        // session_key 格式: "{channel}:{chat_id}"
        session_key.split(':').next().unwrap_or("desktop").to_string()
    }
}
```

## Desktop Channel（Tauri）

```rust
pub struct DesktopChannel {
    config: ChannelConfig,
    bus: Arc<MessageBus>,
}

impl DesktopChannel {
    pub fn new(config: ChannelConfig, bus: Arc<MessageBus>) -> Self {
        Self { config, bus }
    }
}

#[async_trait]
impl Channel for DesktopChannel {
    fn name(&self) -> &str {
        &self.config.name
    }
    
    fn channel_type(&self) -> ChannelType {
        ChannelType::Desktop
    }
    
    async fn start(&mut self) -> Result<()> {
        // Desktop 通道由 Tauri 前端驱动
        // 这里只需要注册到 MessageBus
        log::info!("Desktop channel '{}' started", self.name());
        Ok(())
    }
    
    async fn stop(&mut self) -> Result<()> {
        log::info!("Desktop channel '{}' stopped", self.name());
        Ok(())
    }
    
    async fn send(&self, message: &OutboundMessage) -> Result<()> {
        // 通过 Tauri 的 emit 发送到前端
        // 实际实现由 Tauri 命令处理
        Ok(())
    }
    
    fn check_sender(&self, _sender_id: &str) -> bool {
        // Desktop 通道不检查发送者
        true
    }
}

/// Tauri 命令：发送消息
#[tauri::command]
pub async fn send_message(
    message: String,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let inbound = InboundMessage {
        session_key: format!("desktop:default"),
        sender_id: "user".to_string(),
        content: message,
        attachments: vec![],
        timestamp: chrono::Utc::now(),
    };
    
    state.bus.publish_inbound(inbound).await;
    
    Ok("Message sent".to_string())
}
```

## Telegram Channel

```rust
pub struct TelegramChannel {
    config: ChannelConfig,
    telegram_config: TelegramConfig,
    bus: Arc<MessageBus>,
    bot: Option<Bot>,
}

impl TelegramChannel {
    pub fn new(config: ChannelConfig, bus: Arc<MessageBus>) -> Self {
        let telegram_config = match &config.platform_config {
            PlatformConfig::Telegram(c) => c.clone(),
            _ => panic!("Invalid config for Telegram channel"),
        };
        
        Self {
            config,
            telegram_config,
            bus,
            bot: None,
        }
    }
}

#[async_trait]
impl Channel for TelegramChannel {
    fn name(&self) -> &str {
        &self.config.name
    }
    
    fn channel_type(&self) -> ChannelType {
        ChannelType::Telegram
    }
    
    async fn start(&mut self) -> Result<()> {
        let bot = Bot::new(&self.telegram_config.token);
        
        // 启动消息轮询
        let handler = Update::filter_message()
            .filter(|msg: Message| msg.text().is_some())
            .endpoint(|bot: Bot, msg: Message, bus: Arc<MessageBus>| async move {
                let inbound = InboundMessage {
                    session_key: format!("telegram:{}", msg.chat.id),
                    sender_id: msg.from.map(|u| u.id.to_string()).unwrap_or_default(),
                    content: msg.text().unwrap_or("").to_string(),
                    attachments: vec![],
                    timestamp: chrono::Utc::now(),
                };
                
                bus.publish_inbound(inbound).await;
                
                Ok(())
            });
        
        // 启动 Dispatcher
        Dispatcher::builder(bot.clone(), handler)
            .dependencies(dptree::deps![self.bus.clone()])
            .build()
            .dispatch();
        
        self.bot = Some(bot);
        
        log::info!("Telegram channel '{}' started", self.name());
        Ok(())
    }
    
    async fn stop(&mut self) -> Result<()> {
        self.bot = None;
        log::info!("Telegram channel '{}' stopped", self.name());
        Ok(())
    }
    
    async fn send(&self, message: &OutboundMessage) -> Result<()> {
        let bot = self.bot.as_ref().unwrap();
        
        // 解析 chat_id
        let chat_id: i64 = message.session_key
            .split(':')
            .nth(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or_default();
        
        // 发送消息
        match &message.payload {
            OutboundPayload::Chunk { content, .. } => {
                bot.send_message(ChatId(chat_id), content).await?;
            }
            OutboundPayload::Done { content, .. } => {
                bot.send_message(ChatId(chat_id), content).await?;
            }
            _ => {}
        }
        
        Ok(())
    }
    
    fn check_sender(&self, sender_id: &str) -> bool {
        if self.config.whitelist.is_empty() {
            return true;
        }
        self.config.whitelist.contains(&sender_id.to_string())
    }
}
```

## Feishu Channel

```rust
pub struct FeishuChannel {
    config: ChannelConfig,
    feishu_config: FeishuConfig,
    bus: Arc<MessageBus>,
}

impl FeishuChannel {
    pub fn new(config: ChannelConfig, bus: Arc<MessageBus>) -> Self {
        let feishu_config = match &config.platform_config {
            PlatformConfig::Feishu(c) => c.clone(),
            _ => panic!("Invalid config for Feishu channel"),
        };
        
        Self {
            config,
            feishu_config,
            bus,
        }
    }
}

#[async_trait]
impl Channel for FeishuChannel {
    fn name(&self) -> &str {
        &self.config.name
    }
    
    fn channel_type(&self) -> ChannelType {
        ChannelType::Feishu
    }
    
    async fn start(&mut self) -> Result<()> {
        // 启动 HTTP Webhook 服务器
        // 飞书通过 Webhook 推送消息
        log::info!("Feishu channel '{}' started", self.name());
        Ok(())
    }
    
    async fn stop(&mut self) -> Result<()> {
        log::info!("Feishu channel '{}' stopped", self.name());
        Ok(())
    }
    
    async fn send(&self, message: &OutboundMessage) -> Result<()> {
        // 调用飞书 API 发送消息
        // 使用 feishu-rs 或 reqwest
        Ok(())
    }
    
    fn check_sender(&self, sender_id: &str) -> bool {
        if self.config.whitelist.is_empty() {
            return true;
        }
        self.config.whitelist.contains(&sender_id.to_string())
    }
}
```

## 配置示例

```yaml
# channels.yaml
channels:
  # Desktop 通道（主入口）
  - name: "desktop"
    channel_type: "Desktop"
    enabled: true
    whitelist: []
    platform_config:
      type: "Desktop"
      window_title: "MindClaw"
  
  # Telegram Bot（个人使用）
  - name: "telegram_personal"
    channel_type: "Telegram"
    enabled: true
    whitelist:
      - "123456789"  # 个人 Telegram ID
    platform_config:
      type: "Telegram"
      token: "${TELEGRAM_BOT_TOKEN}"
      proxy: "socks5://127.0.0.1:1080"
  
  # Telegram Bot（团队使用）
  - name: "telegram_team"
    channel_type: "Telegram"
    enabled: false
    whitelist: []
    platform_config:
      type: "Telegram"
      token: "${TELEGRAM_TEAM_BOT_TOKEN}"
  
  # 飞书 Bot
  - name: "feishu"
    channel_type: "Feishu"
    enabled: true
    whitelist: []
    platform_config:
      type: "Feishu"
      app_id: "${FEISHU_APP_ID}"
      app_secret: "${FEISHU_APP_SECRET}"
      encrypt_key: "${FEISHU_ENCRYPT_KEY}"
```

## Session Key 格式

```
{channel_name}:{chat_id}

示例：
- desktop:default
- telegram:123456789
- feishu:oc_123456789
```

## 与 VikingBot 的差异

| 维度 | VikingBot | MindClaw |
|------|-----------|----------|
| 通道数量 | 11 种 | 3+ 种（可扩展） |
| 配置方式 | 代码配置 | YAML 配置文件 |
| 多实例 | 支持 | 支持 |
| Channel trait | 有 | 有（类似设计） |
| ChannelManager | 有 | 有（类似设计） |
| 白名单 | 支持 | 支持 |
| 出站调度 | 通过 session_key 路由 | 相同 |
| Desktop | Tauri | Tauri |
| Telegram | 支持 | 支持 |
| Discord | 支持 | 可选 |
| Feishu | 支持 | 支持 |

## 设计决策

| 决策 | 选择 | 理由 |
|------|------|------|
| Channel trait | async_trait | 统一接口，易于扩展 |
| 配置格式 | YAML | 易读易写，支持注释 |
| 多实例 | 支持 | 同一平台多个 Bot |
| 白名单 | 通道级 | 细粒度权限控制 |
| Session key | `{channel}:{chat_id}` | 唯一标识，便于路由 |
| 环境变量 | `${VAR_NAME}` | 敏感信息不硬编码 |
| 热重载 | 不支持（需重启） | 简化实现 |
