# MindClaw 技术架构设计

> 拆分自 architecture.md，完整索引见 [README.md](./README.md)

## 六、Agent 架构

> 参考 [zeroclaw](https://github.com/zeroclaw-labs/zeroclaw) 的 Channel + Agent 分层模式。

### 6.1 整体结构

```
┌─────────────────────────────────────────────────────────────┐
│                      Channel Layer                          │
│  ┌──────────┐  ┌──────────────┐  ┌───────────────┐         │
│  │ Desktop  │  │ Telegram Bot │  │  Feishu Bot   │         │
│  │ Channel  │  │   Channel    │  │   Channel     │         │
│  └─────┬────┘  └──────┬───────┘  └───────┬───────┘         │
│        └──────────────┼──────────────────┘                  │
│                       │                                     │
│              ChannelMessage / SendMessage                    │
└───────────────────────┼─────────────────────────────────────┘
                        │
┌───────────────────────┼─────────────────────────────────────┐
│                  Gateway Layer                              │
│  ┌──────────────┐  ┌──────────────┐  ┌────────────────┐    │
│  │ HTTP Server  │  │  WebSocket   │  │  Auth Guard    │    │
│  │ (PWA/API)    │  │  (实时对话)   │  │  (Token/签名)  │    │
│  └──────┬───────┘  └──────┬───────┘  └────────────────┘    │
│         └─────────────────┘                                 │
└───────────────────────┼─────────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────────────────┐
│                  Core Agent Service                         │
│                                                             │
│  ┌──────────┐ ┌──────────┐ ┌────────┐ ┌────────────────┐  │
│  │ Context  │ │ Session  │ │ Router │ │   Observer     │  │
│  │ Builder  │ │ Manager  │ │        │ │ (Layer 3 观察)  │  │
│  └──────────┘ └──────────┘ └────────┘ └────────────────┘  │
│                                                             │
│  ┌───────────────────────┐   ┌──────────────────────────┐  │
│  │     Tool Registry     │   │   Memory / Knowledge     │  │
│  │  搜索·分析·写作·文件    │   │   RAG · 观察 · 知识库    │  │
│  └───────────────────────┘   └──────────────────────────┘  │
└───────────────────────┬─────────────────────────────────────┘
                        │
┌───────────────────────┼─────────────────────────────────────┐
│                  Provider Layer                             │
│  ┌──────────┐  ┌──────────┐  ┌──────────────────────────┐  │
│  │  Haiku   │  │  Sonnet  │  │  Local Embedding         │  │
│  │ (路由/分类)│  │ (深度对话)│  │  (向量检索, Phase 2)     │  │
│  └──────────┘  └──────────┘  └──────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│               Infrastructure Layer                          │
│  ┌──────────────┐  ┌────────────────┐  ┌────────────────┐  │
│  │     Cron     │  │   Heartbeat    │  │    Logging     │  │
│  │  定时任务调度  │  │  健康检测/监控   │  │  tracing 日志  │  │
│  └──────────────┘  └────────────────┘  └────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

**Core Agent 是唯一持有完整人模型的编排器**。Channel、Gateway、Provider、Tools 都是可替换的适配层，通过 trait 解耦。Cron 和 Heartbeat 提供后台运行能力。

### 6.2 Channel Trait — 统一消息通道

Channel 是所有通信平台的抽象接口。无论消息来自桌面 UI、Telegram 还是 Feishu，Agent 看到的都是统一的 `ChannelMessage`。

```rust
// src-tauri/src/channels/traits.rs

/// 通道消息（入站）
pub struct ChannelMessage {
    pub id: String,
    pub sender: String,        // 用户标识
    pub content: String,       // 消息内容
    pub source: ChannelSource, // 来源通道
    pub timestamp: DateTime<Utc>,
    pub mode: ConversationMode, // 交互模式
}

/// 发送消息（出站）
pub struct SendMessage {
    pub content: String,
    pub recipient: String,
    pub metadata: Option<serde_json::Value>,
}

/// 通道来源
pub enum ChannelSource {
    Desktop,    // Tauri 桌面端
    Telegram,   // Telegram Bot
    Feishu,     // 飞书 Bot
    Webhook,    // 通用 Webhook
}

/// Channel trait — 所有通道实现此接口
#[async_trait]
pub trait Channel: Send + Sync {
    /// 通道名称
    fn name(&self) -> &str;

    /// 通道来源标识
    fn source(&self) -> ChannelSource;

    /// 发送消息到通道（由 outbound 消费循环调用）
    async fn send(&self, message: OutboundMessage) -> Result<(), AppError>;

    /// 监听平台消息，推入 Bus 入站队列（长运行）
    async fn listen(&self, bus: Arc<MessageBus>) -> Result<(), AppError>;

    // --- 可选能力（默认空实现）---

    /// 是否支持流式输出
    fn supports_streaming(&self) -> bool { false }

    /// 发送流式 chunk
    async fn send_chunk(&self, _chunk: &str, _session_id: &str) -> Result<(), AppError> {
        Ok(())
    }

    /// 发送 typing 指示器
    async fn start_typing(&self) -> Result<(), AppError> { Ok(()) }
    async fn stop_typing(&self) -> Result<(), AppError> { Ok(()) }
}
```

### 6.3 MessageBus — 双向异步消息队列

MessageBus 解耦 Channel 与 Agent 的消息传递。Channel 推入站消息，Agent 推出站消息，双方互不直接引用。

```
Channel.listen()                          Channel.send()
      │                                        ▲
      ▼                                        │
┌──────────────────────────────────────────────────┐
│                  MessageBus                      │
│                                                  │
│  inbound: Queue<InboundMessage>     ──► Agent 消费│
│  outbound: Queue<OutboundMessage>   ◄── Agent 推送│
│                                                  │
└──────────────────────────────────────────────────┘
```

**核心价值**：出站队列使 Channel 断线时消息不丢失，重连后可继续消费。

```rust
// src-tauri/src/bus/events.rs

/// 入站消息：Channel → Agent
pub struct InboundMessage {
    pub id: String,
    pub channel_message: ChannelMessage,
    pub source: ChannelSource,
    pub reply_to: ChannelSource,         // 响应应发回哪个通道
}

/// 出站消息：Agent → Channel
pub struct OutboundMessage {
    pub id: String,
    pub target: ChannelSource,           // 目标通道
    pub session_id: String,
    pub payload: OutboundPayload,
}

pub enum OutboundPayload {
    Text(String),                        // 完整文本响应
    Chunk { content: String, done: bool }, // 流式片段
    Typing(bool),                        // typing 指示器
    Error(String),                       // 错误消息
}
```

```rust
// src-tauri/src/bus/mod.rs

pub struct MessageBus {
    inbound: mpsc::Sender<InboundMessage>,
    inbound_rx: Mutex<Option<mpsc::Receiver<InboundMessage>>>,
    outbound: mpsc::Sender<OutboundMessage>,
    outbound_rx: Mutex<Option<mpsc::Receiver<OutboundMessage>>>,
    // 队列状态计数器（mpsc 不暴露 pending count，手动维护）
    inbound_count: AtomicUsize,
    outbound_count: AtomicUsize,
}

impl MessageBus {
    pub fn new(buffer_size: usize) -> Self {
        let (in_tx, in_rx) = mpsc::channel(buffer_size);
        let (out_tx, out_rx) = mpsc::channel(buffer_size);
        Self {
            inbound: in_tx,
            inbound_rx: Mutex::new(Some(in_rx)),
            outbound: out_tx,
            outbound_rx: Mutex::new(Some(out_rx)),
            inbound_count: AtomicUsize::new(0),
            outbound_count: AtomicUsize::new(0),
        }
    }

    /// Channel 调用：推送入站消息（返回 Result，调用方决定错误处理策略）
    pub async fn publish_inbound(&self, msg: InboundMessage) -> Result<(), AppError> {
        self.inbound.send(msg).await
            .map_err(|_| AppError::Internal("Inbound channel closed (Agent may have crashed)".into()))?;
        self.inbound_count.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// AgentLoop 调用：取出入站 receiver（返回 Result 而非 panic）
    pub fn take_inbound_rx(&self) -> Result<mpsc::Receiver<InboundMessage>, AppError> {
        self.inbound_rx.lock().unwrap().take()
            .ok_or(AppError::Internal("inbound_rx already taken".into()))
    }

    /// AgentLoop 调用：推送出站消息
    pub async fn publish_outbound(&self, msg: OutboundMessage) -> Result<(), AppError> {
        self.outbound.send(msg).await
            .map_err(|_| AppError::Internal("Outbound channel closed".into()))?;
        self.outbound_count.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// 出站消费循环调用：取出出站 receiver
    pub fn take_outbound_rx(&self) -> Result<mpsc::Receiver<OutboundMessage>, AppError> {
        self.outbound_rx.lock().unwrap().take()
            .ok_or(AppError::Internal("outbound_rx already taken".into()))
    }

    /// 队列状态（/status 指令可用）
    pub fn inbound_pending(&self) -> usize {
        self.inbound_count.load(Ordering::Relaxed)
    }
    pub fn outbound_pending(&self) -> usize {
        self.outbound_count.load(Ordering::Relaxed)
    }
}
```

**出站消费循环**：根据 `target` 路由到对应 Channel：

```rust
// src-tauri/src/channels/mod.rs

pub async fn run_outbound_dispatcher(
    mut rx: mpsc::Receiver<OutboundMessage>,
    channels: HashMap<ChannelSource, Arc<dyn Channel>>,
) {
    while let Some(msg) = rx.recv().await {
        if let Some(channel) = channels.get(&msg.target) {
            if let Err(e) = channel.send(msg).await {
                tracing::error!("Outbound dispatch failed: {}", e);
                // 失败消息可放回队列重试（Phase 2）
            }
        }
    }
}
```

### 6.4 Channel 实现

#### Desktop Channel（MVP 核心）

桌面端 Channel 是 Tauri IPC 的桥梁——前端 `invoke()` 调用通过 Desktop Channel 转化为 `ChannelMessage`，Agent 响应通过 Tauri Event 推回前端。

```rust
// src-tauri/src/channels/desktop.rs

pub struct DesktopChannel {
    app_handle: AppHandle,
}

#[async_trait]
impl Channel for DesktopChannel {
    fn name(&self) -> &str { "desktop" }
    fn source(&self) -> ChannelSource { ChannelSource::Desktop }
    fn supports_streaming(&self) -> bool { true }

    async fn send(&self, msg: OutboundMessage) -> Result<(), AppError> {
        match msg.payload {
            OutboundPayload::Text(text) => {
                self.app_handle.emit("agent_response", json!({
                    "session_id": msg.session_id, "content": text
                }))?;
            }
            OutboundPayload::Chunk { content, done } => {
                self.app_handle.emit("conversation_chunk", json!({
                    "session_id": msg.session_id, "content": content, "done": done
                }))?;
            }
            OutboundPayload::Typing(active) => {
                self.app_handle.emit("typing", json!({"active": active}))?;
            }
            OutboundPayload::Error(err) => {
                self.app_handle.emit("agent_error", json!({"error": err}))?;
            }
        }
        Ok(())
    }

    async fn listen(&self, bus: Arc<MessageBus>) -> Result<(), AppError> {
        // Desktop 的入站由 Tauri command 驱动：
        // commands/conversation.rs 接收前端 invoke() 后调用
        // bus.publish_inbound(InboundMessage { ... })
        Ok(())
    }
}
```

#### Telegram Channel（Phase 1 后期）

```rust
// src-tauri/src/channels/telegram.rs

pub struct TelegramChannel {
    bot_token: String,
    http_client: reqwest::Client,
}

#[async_trait]
impl Channel for TelegramChannel {
    fn name(&self) -> &str { "telegram" }
    fn source(&self) -> ChannelSource { ChannelSource::Telegram }

    async fn send(&self, msg: OutboundMessage) -> Result<(), AppError> {
        match msg.payload {
            OutboundPayload::Text(text) => {
                // POST https://api.telegram.org/bot{token}/sendMessage
                // ...
            }
            _ => {} // Telegram 不支持 chunk/typing
        }
        Ok(())
    }

    async fn listen(&self, bus: Arc<MessageBus>) -> Result<(), AppError> {
        // Long polling getUpdates 或 Webhook 模式
        // 将 Telegram Update 转为 ChannelMessage
        // bus.publish_inbound(InboundMessage { ... })
    }
}
```

### 6.5 Agent 三件套：Loop · Context · Session

Agent 模块内部由三个核心组件驱动，职责清晰分离：

```
              Bus.inbound (入站队列)
                        │
                        ▼
┌───────────────────────────────────────────────────────┐
│                AgentLoop (主循环)                      │
│  消费入站 → 协调 Context/Session → 调用 Provider      │
│  → 工具调用循环 → 派发 SubAgent → 推送 Bus.outbound   │
│                                                       │
│  ┌─────────────────┐  ┌────────────────────────┐      │
│  │  SessionManager │  │   ContextBuilder       │      │
│  │  会话生命周期    │  │   上下文组装引擎        │      │
│  │  历史存取/裁剪   │  │   RAG/压缩/token预算   │      │
│  └────────┬────────┘  └───────────┬────────────┘      │
│           │                       │                   │
│           └───────────┬───────────┘                   │
│                       ▼                               │
│              Provider.chat() / chat_stream()          │
│                       │                               │
│                       ▼                               │
│              ToolRegistry.execute() (工具调用循环)     │
│                       │                               │
│              ┌────────┴────────┐                      │
│              │                 │                      │
│              ▼                 ▼                      │
│   Bus.outbound 推送   SubAgent 派发 (异步)           │
│         Channel ←      ├── KnowledgeDistill          │
│                        ├── ObservationAnalyze         │
│                        ├── SessionSummarize           │
│                        ├── CaptureRoute               │
│                        └── DailySummary               │
└───────────────────────┬───────────────────────────────┘
                        │
                        ▼
                   SendMessage (出站，立即返回)
```

#### AgentLoop — 消息处理主循环

AgentLoop 是 Agent 的驱动引擎。它从 Channel 接收 `ChannelMessage`，协调 Session 和 Context 组装完整 prompt，向 Provider 发起请求，处理工具调用循环，最终通过 Channel 返回响应。

```rust
// src-tauri/src/agent/agent_loop.rs

pub struct AgentLoop {
    bus: Arc<MessageBus>,                      // 双向消息总线
    session_mgr: Arc<SessionManager>,         // 会话管理
    context_builder: Arc<ContextBuilder>,      // 上下文组装
    provider: Arc<dyn Provider>,               // LLM 调用（外部注入）
    tools: Arc<ToolRegistry>,                  // 工具注册表（外部注入）
    memory: Arc<MemoryManager>,                // 记忆层（观察/偏好/模式/召回）
    agent_commands: Arc<AgentCommandRegistry>, // 控制指令（/new /stop /restart /status）
    sub_agent_tx: mpsc::Sender<SubAgentTask>,  // SubAgent 任务派发
    identity_resolver: Arc<UserIdentityResolver>, // 跨通道用户身份解析
    cancel_token: CancellationToken,           // 优雅取消（/stop 触发）
}

impl AgentLoop {
    /// 启动消息消费循环（长运行，支持 CancellationToken 优雅退出）
    pub async fn run(&self, mut inbound_rx: mpsc::Receiver<InboundMessage>) {
        loop {
            tokio::select! {
                _ = self.cancel_token.cancelled() => {
                    tracing::info!("AgentLoop cancelled, shutting down");
                    break;
                }
                Some(inbound) = inbound_rx.recv() => {
                    let result = self.process_message(inbound.channel_message, &inbound.reply_to).await;
                    if let Err(e) = result {
                        tracing::error!("AgentLoop error: {}", e);
                        let _ = self.bus.publish_outbound(OutboundMessage {
                            id: uuid(),
                            target: inbound.reply_to,
                            session_id: String::new(),
                            payload: OutboundPayload::Error(e.to_string()),
                        }).await;
                    }
                }
                else => break,
            }
        }
    }

    /// 处理单条消息的完整生命周期
    async fn process_message(
        &self,
        message: ChannelMessage,
        reply_to: &ChannelSource,
    ) -> Result<AgentResponse, AppError> {
        // 1. 身份解析：跨通道统一用户身份（单用户场景全部映射到 "owner"）
        let canonical_user = self.identity_resolver
            .resolve(&message.sender, &message.source);

        // 2. Session：按统一身份加载或创建会话
        let session = self.session_mgr
            .get_or_create(&canonical_user, &message.mode).await?;

        // 2.5 Agent Command 拦截（/new /stop /restart /status）
        if let Some(cmd_name) = parse_agent_command(&message.content) {
            if let Some(cmd) = self.agent_commands.get(cmd_name) {
                let ctx = AgentCommandContext {
                    session: session.clone(),
                    session_mgr: self.session_mgr.clone(),
                    sub_agent_tx: self.sub_agent_tx.clone(),
                    cancel_token: self.cancel_token.clone(), // /stop 可触发取消
                };
                let result = cmd.execute(ctx).await?;
                self.bus.publish_outbound(OutboundMessage {
                    id: uuid(), target: reply_to.clone(),
                    session_id: session.id.clone(),
                    payload: OutboundPayload::Text(result.response.clone()),
                }).await;
                self.handle_action(result.action).await?;
                return Ok(AgentResponse::from_text(result.response));
            }
        }

        // 3. Context：组装完整 prompt
        let context = self.context_builder.build(&message, &session).await?;

        // 4. 选择模型
        let model = self.select_model(&message.mode);

        // 5. 智能流式调用 + 工具循环（两阶段策略，详见 call_with_tools）
        let final_response = self.call_with_tools(
            model, context, &session, reply_to,
        ).await?;

        // 6. Session：追加消息对 + 裁剪
        self.session_mgr.append(&session.id, &message, &final_response).await?;

        // 7. 后处理：写入 Memory 记忆、派发 SubAgent 任务
        self.post_process(&message, &final_response, &session).await?;

        Ok(final_response)
    }

    /// 两阶段流式策略：解决流式输出与工具调用的冲突
    ///
    /// 核心问题：如果流式推送所有 chunk，工具调用的 JSON 标记会直接暴露给用户。
    /// 解决方案：解析 SSE 事件类型，仅推送 text 内容，静默累积 tool_use blocks。
    ///
    /// 流程：
    ///   1. 流式调用 Provider，实时解析 content_block 类型
    ///   2. text 类型 → 立即推送给用户（保持流式体验）
    ///   3. tool_use 类型 → 静默累积（用户不可见）
    ///   4. 如有工具调用 → 执行工具 → 将结果注入上下文 → 再次流式调用（循环）
    ///   5. 无工具调用 → 发送 done 信号，返回完整响应
    async fn call_with_tools(
        &self,
        model: ModelTier,
        mut context: Vec<ChatMessage>,
        session: &Session,
        reply_to: &ChannelSource,
    ) -> Result<AgentResponse, AppError> {
        let mut iterations = 0;
        let mut seen_hashes = HashSet::new();
        let mut full_text = String::new(); // 累积所有轮次的文本输出

        // 发送 typing 指示器
        self.bus.publish_outbound(OutboundMessage {
            id: uuid(), target: reply_to.clone(),
            session_id: session.id.clone(),
            payload: OutboundPayload::Typing(true),
        }).await;

        loop {
            if iterations >= 10 { break; }

            // 流式调用，按 content_block 类型分流
            let stream_result = self.stream_with_tool_detection(
                model, &context, session, reply_to,
            ).await;

            // 流式中断错误恢复：通知用户 + 终止信号
            let (text_content, tool_calls) = match stream_result {
                Ok(result) => result,
                Err(e) => {
                    tracing::error!("Stream interrupted: {}", e);
                    self.bus.publish_outbound(OutboundMessage {
                        id: uuid(), target: reply_to.clone(),
                        session_id: session.id.clone(),
                        payload: OutboundPayload::Error(
                            format!("响应中断: {}", e)
                        ),
                    }).await;
                    self.bus.publish_outbound(OutboundMessage {
                        id: uuid(), target: reply_to.clone(),
                        session_id: session.id.clone(),
                        payload: OutboundPayload::Chunk {
                            content: String::new(), done: true
                        },
                    }).await;
                    // 不追加部分响应到 Session 历史
                    return Err(e);
                }
            };

            full_text.push_str(&text_content);

            // 无工具调用 → 结束
            if tool_calls.is_empty() { break; }

            // 循环检测
            let hash = hash_tool_calls(&tool_calls);
            if !seen_hashes.insert(hash) { break; }

            // 检查取消信号
            if self.cancel_token.is_cancelled() {
                tracing::info!("Tool loop cancelled by /stop");
                break;
            }

            // 执行工具（用户看到 typing 指示器）
            let results = self.tools.execute_batch(tool_calls).await;

            // 将 assistant 响应 + 工具结果追加到上下文
            context = self.context_builder
                .append_tool_results_to_context(context, &text_content, &results);

            iterations += 1;
        }

        // 完成信号
        self.bus.publish_outbound(OutboundMessage {
            id: uuid(), target: reply_to.clone(),
            session_id: session.id.clone(),
            payload: OutboundPayload::Chunk { content: String::new(), done: true },
        }).await;

        Ok(AgentResponse::from_text(full_text))
    }

    /// 单次流式调用：解析 SSE 事件，分离 text 和 tool_use
    /// 返回 (text_content, tool_calls)
    async fn stream_with_tool_detection(
        &self,
        model: ModelTier,
        context: &[ChatMessage],
        session: &Session,
        reply_to: &ChannelSource,
    ) -> Result<(String, Vec<ToolCall>), AppError> {
        let mut stream = self.provider.chat_stream(model, context).await?;
        let mut text_content = String::new();
        let mut tool_calls = Vec::new();
        let mut current_block_type: Option<String> = None;

        while let Some(event) = stream.next().await {
            // 检查取消信号
            if self.cancel_token.is_cancelled() {
                return Err(AppError::Cancelled("Stream cancelled by /stop".into()));
            }

            let event = event?;
            match event {
                StreamEvent::ContentBlockStart { block_type, .. } => {
                    current_block_type = Some(block_type);
                }
                StreamEvent::ContentBlockDelta { delta } => {
                    match current_block_type.as_deref() {
                        Some("text") => {
                            // 文本内容：立即推送给用户（保持流式体验）
                            text_content.push_str(&delta);
                            self.bus.publish_outbound(OutboundMessage {
                                id: uuid(), target: reply_to.clone(),
                                session_id: session.id.clone(),
                                payload: OutboundPayload::Chunk {
                                    content: delta, done: false
                                },
                            }).await;
                        }
                        Some("tool_use") => {
                            // 工具调用：静默累积，用户不可见
                            // tool_calls 在 ContentBlockStop 时解析完整 JSON
                        }
                        _ => {}
                    }
                }
                StreamEvent::ContentBlockStop { tool_call } => {
                    if let Some(tc) = tool_call {
                        tool_calls.push(tc);
                    }
                    current_block_type = None;
                }
                _ => {}
            }
        }

        Ok((text_content, tool_calls))
    }

    fn select_model(&self, mode: &ConversationMode) -> ModelTier {
        match mode {
            ConversationMode::Companion => ModelTier::Sonnet,
            ConversationMode::Knowledge => ModelTier::Sonnet,
            ConversationMode::Reflect   => ModelTier::Sonnet,
            ConversationMode::Challenge => ModelTier::Sonnet,
            ConversationMode::TreeHole  => ModelTier::Sonnet,
        }
    }
}
```

#### ContextBuilder — 上下文组装引擎

ContextBuilder 负责将分散的数据源组装为完整的 LLM prompt，并严格控制 token 预算。

```rust
// src-tauri/src/agent/context.rs

pub struct ContextBuilder {
    memory: Arc<MemoryManager>,       // 记忆层（观察召回 + 偏好注入）
    services: Arc<ServiceContainer>,  // 业务层（知识检索 RAG）
    db: Arc<DbState>,                // 用户角色等
}

impl ContextBuilder {
    /// 组装完整 prompt（返回 ChatMessage 数组）
    pub async fn build(
        &self,
        message: &ChannelMessage,
        session: &Session,
    ) -> Result<Vec<ChatMessage>, AppError> {
        let mut messages = Vec::new();

        // [1] System Prompt = 人格 + 模式指令 + 角色上下文 + 工具描述
        let system = self.build_system_prompt(&message.mode).await?;
        messages.push(ChatMessage::system(system));

        // [2] RAG 知识注入（三级渐进：L0 粗筛 → L1 重排序 → 注入 Top L1）
        let knowledge_l1s = self.services.knowledge
            .search_with_rerank(&message.content, 5).await?;
        if !knowledge_l1s.is_empty() {
            // 注入 L1 overview（~2k tokens/条，比传统 500 token snippet 信息量更大）
            // Agent 需要完整内容时可通过 operations.call("knowledge_get") 加载 L2
            messages.push(ChatMessage::system(
                format_knowledge_l1_context(&knowledge_l1s)
            ));
        }

        // [3] Memory 记忆召回：未浮出的记忆
        let memories = self.memory.unsurfaced(3).await?;
        if !memories.is_empty() {
            messages.push(ChatMessage::system(
                format_memories(&memories)
            ));
        }

        // [4] 压缩的对话历史（近 5 轮完整 + 早期摘要）
        messages.extend(session.compressed_history());

        // [5] 用户消息
        messages.push(ChatMessage::user(&message.content));

        // Token 预算检查
        self.enforce_budget(&mut messages, &message.mode)?;

        Ok(messages)
    }

    /// Token 预算控制（从 settings.json 读取，可配置）
    fn enforce_budget(
        &self,
        messages: &mut Vec<ChatMessage>,
        mode: &ConversationMode,
    ) -> Result<(), AppError> {
        // 预算从 Provider.max_tokens() 或 settings.json token_budgets 读取
        // 默认值：Haiku 16K, Sonnet 80K（远低于模型上限但平衡成本）
        let budget = self.settings.token_budgets
            .get(&self.select_tier(mode))
            .copied()
            .unwrap_or_else(|| match self.select_tier(mode) {
                ModelTier::Haiku => 16_000,
                ModelTier::Sonnet => 80_000,
            });
        // 超预算时：先裁剪 RAG 片段数 → 再压缩历史 → 最后截断观察
        // ...
        Ok(())
    }

    /// 追加工具执行结果到上下文（用于工具调用循环）
    pub fn append_tool_results(
        &self,
        session: &Session,
        results: &[Result<ToolOutput, AppError>],
    ) -> Vec<ChatMessage>;
}
```

#### SessionManager — 会话生命周期管理

SessionManager 管理会话的创建、历史存取、裁剪和持久化。每个 sender 维护独立的会话上下文。

```rust
// src-tauri/src/agent/session.rs

pub struct Session {
    pub id: String,
    pub sender: String,
    pub mode: ConversationMode,
    pub messages: Vec<ChatMessage>,  // 内存中的活跃历史
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
}

impl Session {
    /// 返回压缩后的历史（近 5 轮完整 + 早期摘要）
    pub fn compressed_history(&self) -> Vec<ChatMessage>;
}

pub struct SessionManager {
    db: Arc<DbState>,
    max_turns: usize,      // 内存中保留的最大轮数（默认 50）
    keep_recent: usize,    // 裁剪时保护的近期轮数（默认 5）
}

impl SessionManager {
    /// 按 sender 获取或创建 session
    pub async fn get_or_create(
        &self, sender: &str, mode: &ConversationMode
    ) -> Result<Session, AppError>;

    /// 追加消息对（user + assistant），触发自动裁剪
    pub async fn append(
        &self, session_id: &str, user_msg: &ChannelMessage, agent_resp: &AgentResponse
    ) -> Result<(), AppError> {
        // 1. 追加到内存 + SQLite
        // 2. 如果超过 max_turns，触发 prune
        Ok(())
    }

    /// 历史裁剪：保护近 N 轮 + 系统消息，压缩早期为摘要
    async fn prune(&self, session: &mut Session) -> Result<(), AppError> {
        // Phase 1: 折叠工具调用/结果对
        // Phase 2: 用 Haiku 将早期消息压缩为摘要
        // Phase 3: 删除超龄原始消息
        Ok(())
    }

    /// 持久化 session 到 SQLite（追加消息时自动调用）
    async fn persist(&self, session: &Session) -> Result<(), AppError>;
}
```

MindClaw 是单用户桌面应用，但用户可能从 Desktop、Telegram、Feishu 等不同通道发送消息。`UserIdentityResolver` 确保跨通道的用户身份统一，避免会话和记忆碎片化。

#### UserIdentityResolver — 跨通道身份统一

```rust
// src-tauri/src/agent/identity.rs

/// 将不同通道的 sender 标识映射为统一的 canonical user ID
/// 单用户场景：所有来源映射到 "owner"
/// 未来多用户：可通过配置表映射
pub struct UserIdentityResolver {
    mode: IdentityMode,
}

pub enum IdentityMode {
    /// 单用户模式：所有 sender 映射到 "owner"（默认）
    SingleUser,
    /// 映射模式：按 (source, sender) → canonical_user 查表
    Mapped(HashMap<(ChannelSource, String), String>),
}

impl UserIdentityResolver {
    pub fn single_user() -> Self {
        Self { mode: IdentityMode::SingleUser }
    }

    pub fn resolve(&self, sender: &str, source: &ChannelSource) -> String {
        match &self.mode {
            IdentityMode::SingleUser => "owner".to_string(),
            IdentityMode::Mapped(map) => {
                map.get(&(source.clone(), sender.to_string()))
                    .cloned()
                    .unwrap_or_else(|| sender.to_string())
            }
        }
    }
}
```

### 6.6 Agent 初始化与接线

应用启动时，各模块组装并注入 AgentLoop：

```rust
// src-tauri/src/agent/mod.rs

pub fn init_agent(
    db: Arc<DbState>,
    provider: Arc<dyn Provider>,
    tools: Arc<ToolRegistry>,
    memory: Arc<MemoryManager>,
    services: Arc<ServiceContainer>,
    agent_commands: Arc<AgentCommandRegistry>,
    bus: Arc<MessageBus>,
) -> (AgentLoop, CancellationToken) {
    // SubAgent 后台执行器（限制并发数，防止 API 速率爆炸）
    let (sub_tx, sub_rx) = mpsc::channel(32);
    let sub_agent = SubAgentExecutor::new(provider.clone(), db.clone(), memory.clone());
    tokio::spawn(sub_agent.run(sub_rx));

    let session_mgr = Arc::new(SessionManager::new(db.clone()));
    let context_builder = Arc::new(ContextBuilder::new(
        memory.clone(), services.clone(), db.clone(),
    ));
    let cancel_token = CancellationToken::new();

    let agent_loop = AgentLoop {
        bus,
        session_mgr,
        context_builder,
        provider,
        tools,
        memory,
        agent_commands,
        sub_agent_tx: sub_tx,
        identity_resolver: Arc::new(UserIdentityResolver::single_user()),
        cancel_token: cancel_token.clone(),
    };
    (agent_loop, cancel_token)
}
```

```rust
// src-tauri/src/lib.rs（启动时接线）

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let db = init_database()?;
            let provider = create_provider(&settings)?;
            let memory = Arc::new(MemoryManager::new(db.clone()));
            let services = Arc::new(ServiceContainer::new(db.clone()));
            let tools = ToolRegistry::default_tools(&services, &memory, vault_path, mcp_configs);

            // MessageBus：Channel ↔ Agent 双向解耦
            let bus = Arc::new(MessageBus::new(64));

            let agent_commands = Arc::new(AgentCommandRegistry::default());
            let (agent_loop, cancel_token) = init_agent(
                db.clone(), provider, tools, memory.clone(),
                services.clone(), agent_commands, bus.clone(),
            );

            // 启动 AgentLoop（消费 inbound，支持 CancellationToken 优雅退出）
            let inbound_rx = bus.take_inbound_rx()?;  // 返回 Result，不再 panic
            tokio::spawn(agent_loop.run(inbound_rx));

            // 启动 Channel 出站分发（消费 outbound）
            let desktop_channel: Arc<dyn Channel> = Arc::new(DesktopChannel::new(app.handle()));
            let mut channels = HashMap::new();
            channels.insert(ChannelSource::Desktop, desktop_channel.clone());
            let outbound_rx = bus.take_outbound_rx()?;
            tokio::spawn(run_outbound_dispatcher(outbound_rx, channels));

            // Desktop Channel 入站桥接说明：
            // DesktopChannel.listen() 为空实现，因为桌面端入站由 Tauri command 驱动。
            // commands/conversation.rs 的 conversation_send 命令内部：
            //   1. 构造 ChannelMessage + InboundMessage
            //   2. 调用 bus.publish_inbound(msg).await?
            //   3. 立即返回 session_id
            //   4. Agent 异步处理，响应通过 Tauri Event 推送（DesktopChannel.send()）

            // 注入 Tauri 状态
            app.manage(bus.clone());      // commands/conversation.rs 用 bus.publish_inbound()
            app.manage(cancel_token);     // /stop 命令可触发取消
            app.manage(db);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![...])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### 6.7 SubAgent — 异步子任务执行器

AgentLoop 负责处理主对话流，但有些任务不应阻塞对话响应，需要在后台独立完成。SubAgent 就是为这些异步任务设计的轻量执行器。

```
AgentLoop (主对话)
    │
    ├── 对话响应 → 立即返回给用户
    │
    └── 派发 SubAgent 任务（不阻塞）
         ├── CaptureRouteTask:  捕获分类（Haiku）
         ├── KnowledgeDistill:  从对话中提炼知识笔记
         ├── SessionSummarize:  会话摘要生成
         ├── ObservationAnalyze: Layer 3 模式识别
         └── DailySummary:      当日回顾生成
```

```rust
// src-tauri/src/agent/sub_agent.rs

/// 子任务类型
pub enum SubAgentTask {
    /// 捕获路由：对原始输入进行分类（Haiku 调用）
    CaptureRoute {
        capture_id: String,
        raw_content: String,
    },
    /// 知识蒸馏：从对话中提炼出值得沉淀的知识笔记
    KnowledgeDistill {
        session_id: String,
        messages: Vec<ChatMessage>,
    },
    /// 会话摘要：生成对话精华摘要
    SessionSummarize {
        session_id: String,
    },
    /// 观察分析：分析对话模式，记录 Layer 3 观察
    ObservationAnalyze {
        session_id: String,
        recent_messages: Vec<ChatMessage>,
    },
    /// 日记摘要：生成当日回顾
    DailySummary {
        date: String,
    },
}

pub struct SubAgentExecutor {
    provider: Arc<dyn Provider>,
    db: Arc<DbState>,
    memory: Arc<MemoryManager>,
    task_tx: mpsc::Sender<SubAgentTask>,
    concurrency_limit: Arc<Semaphore>,  // 限制并发 API 调用数（默认 3）
}

impl SubAgentExecutor {
    /// 启动后台任务消费循环（Semaphore 限制并发，防止 API 速率爆炸）
    pub async fn run(mut self, mut rx: mpsc::Receiver<SubAgentTask>) {
        while let Some(task) = rx.recv().await {
            let executor = self.clone_refs();
            let permit = self.concurrency_limit.clone();
            tokio::spawn(async move {
                // 获取并发许可（默认最多 3 个同时执行）
                let _permit = permit.acquire().await.unwrap();
                if let Err(e) = executor.execute(task).await {
                    tracing::error!("SubAgent task failed: {}", e);
                }
                // _permit drop 时自动释放
            });
        }
    }

    async fn execute(&self, task: SubAgentTask) -> Result<(), AppError> {
        match task {
            SubAgentTask::CaptureRoute { capture_id, raw_content } => {
                // Haiku 调用：分类为 task/thought/feeling/link
                let result = self.classify(&raw_content).await?;
                self.db.update_capture_route(&capture_id, &result).await?;
            }
            SubAgentTask::KnowledgeDistill { session_id, messages } => {
                // Sonnet 调用：从对话中提炼知识
                let draft = self.distill_knowledge(&messages).await?;
                // 写入 vault/knowledge/ 草稿，等待人类确认
                self.memory.save_knowledge_draft(&draft).await?;
            }
            SubAgentTask::SessionSummarize { session_id } => {
                // Haiku 调用：生成会话摘要
                let summary = self.summarize_session(&session_id).await?;
                self.db.update_session_summary(&session_id, &summary).await?;
            }
            SubAgentTask::ObservationAnalyze { session_id, recent_messages } => {
                // Sonnet 调用：分析模式，发现盲区
                let insights = self.analyze_patterns(&recent_messages).await?;
                for insight in insights {
                    self.memory.remember(insight).await?;
                }
            }
            SubAgentTask::DailySummary { date } => {
                // Sonnet 调用：生成当日回顾
                let summary = self.generate_daily_summary(&date).await?;
                self.memory.append_to_daily(&date, &summary).await?;
            }
        }
        Ok(())
    }
}
```

**SubAgent 与 AgentLoop 的协作**：

```rust
// 在 AgentLoop.post_process() 中，对话完成后派发后台任务
async fn post_process(
    &self,
    message: &ChannelMessage,
    response: &AgentResponse,
    session: &Session,
) -> Result<(), AppError> {
    // 对话完成后，异步派发 SubAgent 任务（不阻塞响应返回）
    if message.mode == ConversationMode::Knowledge {
        let _ = self.sub_agent_tx.send(SubAgentTask::KnowledgeDistill {
            session_id: session.id.clone(),
            messages: session.recent_messages(10),
        }).await;
    }

    // 每次对话后都尝试 Layer 3 观察分析
    let _ = self.sub_agent_tx.send(SubAgentTask::ObservationAnalyze {
        session_id: session.id.clone(),
        recent_messages: session.recent_messages(5),
    }).await;

    Ok(()) // SubAgent 后台运行，不阻塞
}
```

**SubAgent 模型选择**：

| 子任务 | 模型 | 原因 |
|--------|------|------|
| CaptureRoute | Haiku | 简单分类，低成本 |
| SessionSummarize | Haiku | 摘要生成，低成本 |
| KnowledgeDistill | Sonnet | 需要深度理解和提炼 |
| ObservationAnalyze | Sonnet | 需要跨域关联和模式识别 |
| DailySummary | Sonnet | 需要综合当日全部信息 |

### 6.8 消息处理流水线（完整）

```
┌────────────────────────────────────────────────────────────────┐
│                      完整消息流                                 │
├────────────────────────────────────────────────────────────────┤
│                                                                │
│  外部平台 (Desktop UI / Telegram / Feishu)                      │
│       │                                                        │
│       ▼                                                        │
│  Channel.listen() ──► bus.publish_inbound(InboundMessage)      │
│  （Desktop 由 Tauri command 桥接推入 Bus）                      │
│                                                                │
│  ┌──────────── MessageBus ────────────┐                        │
│  │  inbound queue ──► AgentLoop 消费   │                        │
│  │  outbound queue ◄── AgentLoop 推送  │                        │
│  └────────────────────────────────────┘                        │
│       │                                                        │
│       ▼ (inbound)                                              │
│  AgentLoop.process_message()                                   │
│       │                                                        │
│       ├─► UserIdentityResolver: 跨通道身份统一                 │
│       │     Desktop/Telegram/Feishu → canonical "owner"        │
│       │                                                        │
│       ├─► SessionManager: 按统一身份加载/创建 Session          │
│       │                                                        │
│       ├─► Agent Command 拦截 (/new /stop /restart /status)     │
│       │     命中 → 执行控制指令 → bus.outbound 返回             │
│       │     /stop → 触发 CancellationToken 取消进行中操作      │
│       │     未命中 → 继续正常对话流程 ↓                         │
│       │                                                        │
│       ├─► ContextBuilder: 组装完整 prompt                      │
│       │     [人格] + [模式指令] + [角色上下文]                   │
│       │     + [RAG 知识 L1 概要] + [压缩历史] + [记忆召回]      │
│       │     + Token 预算控制（可配置，默认 Sonnet 80K）         │
│       │                                                        │
│       ├─► call_with_tools()（两阶段流式策略，最多 10 轮）      │
│       │     ┌─ stream_with_tool_detection():                   │
│       │     │   解析 SSE content_block 类型                     │
│       │     │   text → 立即推送 Chunk（用户可见）               │
│       │     │   tool_use → 静默累积（用户不可见）               │
│       │     ├─ 有工具调用 → 显示 typing 指示器                 │
│       │     │   → ToolRegistry.execute_batch()                 │
│       │     │   → 结果注入上下文 → 再次流式调用                │
│       │     │   → 循环检测（hash 去重）                        │
│       │     │   → 检查 CancellationToken                       │
│       │     └─ 无工具调用 → 发送 done 信号                     │
│       │     流式中断 → Error + done 信号，不追加到历史          │
│       │                                                        │
│       ├─► SessionManager: 追加消息对 + 自动裁剪                │
│       │                                                        │
│       ├─► post_process() → SubAgent 派发（异步，不阻塞）       │
│       │     ├── KnowledgeDistill（知识模式下）                  │
│       │     ├── ObservationAnalyze（每次对话后）                │
│       │     └── SessionSummarize（会话结束时）                  │
│       │     （Semaphore 限制最大 3 个并发 SubAgent）            │
│       │                                                        │
│       ▼ (outbound)                                             │
│  run_outbound_dispatcher() ──► 按 target 路由到 Channel        │
│       │                                                        │
│       ▼                                                        │
│  Channel.send(OutboundMessage) ──► 外部平台渲染                │
│                                                                │
└────────────────────────────────────────────────────────────────┘
```

### 6.9 Provider Layer — LLM 抽象（独立模块）

Provider 是独立于 Agent 的顶层模块，通过 trait 注入 AgentService。未来可替换为 OpenAI、Ollama 等实现。

```rust
// src-tauri/src/providers/traits.rs

pub enum ModelTier {
    Haiku,   // 路由、分类、简单任务（~1x 成本）
    Sonnet,  // 深度对话、知识沉淀、洞见生成（~10x 成本）
}

#[async_trait]
pub trait Provider: Send + Sync {
    /// 同步调用
    async fn chat(
        &self, model: ModelTier, messages: &[ChatMessage]
    ) -> Result<ProviderResponse, AppError>;

    /// 流式调用
    async fn chat_stream(
        &self, model: ModelTier, messages: &[ChatMessage]
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String, AppError>> + Send>>, AppError>;

    /// 能力查询
    fn supports_streaming(&self) -> bool;
    fn max_tokens(&self, model: ModelTier) -> usize;
}
```

```rust
// src-tauri/src/providers/claude.rs

pub struct ClaudeProvider {
    http_client: reqwest::Client,
    api_key: String,  // 运行时持有，来源见下方构造器
}

impl ClaudeProvider {
    /// 从 OS Keychain 读取 API Key（桌面应用启动时使用）
    pub async fn from_keychain() -> Result<Self, AppError> {
        let api_key = keychain::get("claude_api_key")?;
        Ok(Self { http_client: reqwest::Client::new(), api_key })
    }

    /// 直接传入 API Key（CLI 独立二进制使用）
    pub fn from_key(api_key: &str) -> Self {
        Self { http_client: reqwest::Client::new(), api_key: api_key.to_string() }
    }
}
```

```rust
// src-tauri/src/providers/mod.rs

/// 工厂函数：根据配置创建 Provider 实例
pub fn create_provider(config: &AppSettings) -> Result<Arc<dyn Provider>, AppError> {
    match config.provider.as_str() {
        "claude" => Ok(Arc::new(ClaudeProvider::from_keychain().await?)),
        // 未来可扩展：
        // "openai" => Ok(Arc::new(OpenAIProvider::new()?)),
        // "ollama" => Ok(Arc::new(OllamaProvider::new()?)),
        _ => Err(AppError::Validation("unknown provider".into())),
    }
}
```

### 6.10 Services Layer — 核心业务逻辑

Services 是业务操作的核心层。**Web Commands、CLI Commands 和 Agent 共用同一套 Services**，保证业务逻辑单一来源。

```
Web Commands  ──► Services ──► Storage
CLI Commands  ──► Services ──► Storage
Agent         ──► operations (元工具) ──► Services ──► Storage
                                     ──► Memory   ──► Storage
```

#### ServiceContainer — 业务服务聚合

```rust
// src-tauri/src/services/mod.rs

/// 聚合所有业务 Service，注入 Commands / CLI / Agent 共用
pub struct ServiceContainer {
    pub knowledge: KnowledgeService,
    pub daily: DailyService,
    pub task: TaskService,
    pub capture: CaptureService,
}

impl ServiceContainer {
    pub fn new(db: Arc<DbState>, vault_path: PathBuf) -> Self {
        let storage = Arc::new(StorageManager::new(db, vault_path));
        Self {
            knowledge: KnowledgeService::new(storage.clone()),
            daily: DailyService::new(storage.clone()),
            task: TaskService::new(storage.clone()),
            capture: CaptureService::new(storage.clone()),
        }
    }
}
```

#### KnowledgeService — 知识笔记管理

操作人机共有的知识体系（Markdown 文件 + SQLite 索引）。

```rust
// src-tauri/src/services/knowledge.rs

pub struct KnowledgeService {
    storage: Arc<StorageManager>,
}

impl KnowledgeService {
    // ── 写入 ──

    /// 创建知识笔记（写 Markdown + 提取 tags→L0 + 生成 L1 + 更新 FTS5）
    pub async fn create(&self, title: &str, content: &str, tags: &[String])
        -> Result<KnowledgeEntry, AppError>;

    /// 更新笔记内容（人类纠偏 或 Agent 沉淀，自动更新 L0/L1 索引）
    pub async fn update(&self, path: &str, content: &str)
        -> Result<(), AppError>;

    // ── 三级检索 ──

    /// L0 搜索：FTS5 匹配 title + tags，返回候选集（tags + path + title）
    /// 成本极低，用于粗筛，典型返回 ~20 条
    pub async fn search_l0(&self, query: &str, limit: u32)
        -> Result<Vec<NoteL0>, AppError>;

    /// L1 批量加载：对 L0 候选集加载 overview，用于重排序和 RAG 注入
    pub async fn get_l1_batch(&self, paths: &[String])
        -> Result<Vec<NoteL1>, AppError>;

    /// L2 完整加载：从文件系统读取 Markdown 原文（Agent 按需调用）
    pub async fn get_l2(&self, path: &str)
        -> Result<KnowledgeNote, AppError>;

    /// 组合搜索：L0 粗筛 → 目录递归 → L1 重排序 → 返回 Top N
    pub async fn search_with_rerank(&self, query: &str, top_n: u32)
        -> Result<Vec<NoteL1>, AppError> {
        // 1. L0 粗筛（notes 表统一搜索，同时命中笔记和目录）
        let candidates = self.search_l0(query, 20).await?;

        // 2. 目录递归：命中目录（path 无 .md 后缀）时，展开子笔记补充候选
        let mut all_paths: Vec<String> = Vec::new();
        for c in &candidates {
            if !c.path.ends_with(".md") {
                // 高分目录 → 加载目录下所有子笔记
                let children = self.list_children(&c.path).await?;
                all_paths.extend(children.iter().map(|n| n.path.clone()));
            } else {
                all_paths.push(c.path.clone());
            }
        }
        all_paths.dedup();

        // 3. 加载 L1 → 按关键词重叠度 + tags 匹配度排序
        let l1s = self.get_l1_batch(&all_paths).await?;
        let ranked = self.rerank(query, l1s, top_n);
        Ok(ranked)
    }

    // ── 辅助 ──

    /// 按标签筛选
    pub async fn list(&self, tag: Option<&str>)
        -> Result<Vec<KnowledgeEntry>, AppError>;

    /// 列出目录下直接子节点（WHERE path LIKE '{parent}/%' AND path NOT LIKE '{parent}/%/%'）
    pub async fn list_children(&self, parent: &str)
        -> Result<Vec<NoteL0>, AppError>;

    /// 提取 wikilinks 并更新 links 表
    pub async fn sync_links(&self, path: &str)
        -> Result<(), AppError>;

    /// 重建索引（Markdown → SQLite L0/L1 + FTS5）
    pub async fn rebuild_index(&self, path: Option<&str>)
        -> Result<(), AppError>;
}

/// L0 视图：仅 tags + 路径（~100 tokens/条，适合批量扫描）
pub struct NoteL0 {
    pub path: String,       // 有 .md 后缀 = 笔记，无后缀 = 目录
    pub title: String,
    pub tags: Vec<String>,
}

/// L1 视图：概要（~2k tokens/条，适合 RAG 注入）
pub struct NoteL1 {
    pub path: String,
    pub title: String,
    pub tags: Vec<String>,
    pub overview: String,  // ~2k tokens
}
```

#### DailyService — 日记管理

```rust
// src-tauri/src/services/daily.rs

pub struct DailyService {
    storage: Arc<StorageManager>,
}

impl DailyService {
    /// 获取日记（不存在则从模板创建）+ 关联任务
    pub async fn get(&self, date: &str)
        -> Result<DailyNote, AppError>;

    /// 保存日记内容
    pub async fn save(&self, date: &str, content: &str)
        -> Result<(), AppError>;

    /// 追加条目到日记指定区域
    pub async fn append_entry(&self, date: &str, content: &str, section: Option<&str>)
        -> Result<(), AppError>;

    /// 日记列表（元数据）
    pub async fn list(&self, limit: u32)
        -> Result<Vec<DailyMeta>, AppError>;
}
```

#### TaskService — 任务管理

```rust
// src-tauri/src/services/task.rs

pub struct TaskService {
    storage: Arc<StorageManager>,
}

impl TaskService {
    pub async fn create(&self, content: &str, due: Option<&str>, context: Option<&str>, note_path: Option<&str>)
        -> Result<Task, AppError>;
    pub async fn update(&self, id: &str, status: Option<&str>, content: Option<&str>, due: Option<&str>)
        -> Result<Task, AppError>;
    pub async fn list(&self, status: Option<&str>)
        -> Result<Vec<Task>, AppError>;
    pub async fn complete(&self, id: &str)
        -> Result<(), AppError>;
}
```

#### CaptureService — 捕获队列管理

```rust
// src-tauri/src/services/capture.rs

pub struct CaptureService {
    storage: Arc<StorageManager>,
}

impl CaptureService {
    pub async fn submit(&self, raw: &str, source: &str) -> Result<CaptureItem, AppError>;
    pub async fn list_pending(&self) -> Result<Vec<CaptureItem>, AppError>;
    pub async fn set_route(&self, id: &str, route: &str) -> Result<(), AppError>;
    pub async fn confirm_route(&self, id: &str, route: &str, adjusted: bool) -> Result<(), AppError>;
}
```

### 6.11 Memory Layer — Agent 私有记忆

> PRD 核心命题：**记忆是 Agent 的，知识是共同的。**

Memory 管理 Agent 对用户的私有认知——观察、偏好、模式识别等。这些信息存在 SQLite 中，用户不直接操作。Knowledge（Markdown）是人机共有的，由 Services 管理。

```
Memory (Agent 私有, SQLite)          Knowledge (人机共有, Markdown)
├── 观察：第三次提到工作疲惫感        ├── vault/knowledge/工作节奏.md
├── 偏好：偏好简短直接的回复           ├── vault/knowledge/投资策略.md
├── 模式：周一情绪通常低落             └── vault/knowledge/育儿方法.md
└── 召回：按相关性检索记忆
                                      ↑
    记忆可以升华为知识 ────────────────┘
    （Agent 发现模式 → 沉淀为知识笔记，需人类确认）
```

#### 单表 `memories` 设计

所有记忆存入单表，通过 `category` 区分类型，`key` 去重，`superseded_by` 追踪认知演进：

```rust
// src-tauri/src/memory/mod.rs

/// 统一记忆结构（对应 memories 表）
pub struct Memory {
    pub id: String,
    pub key: String,                        // 唯一去重键，同一认知 upsert
    pub content: String,                    // 记忆内容
    pub category: MemoryCategory,           // 6 类，隐含 owner（user/agent）
    pub importance: f32,                    // 重要度 0.0-1.0（衰减基准、recall 排序）
    pub session_id: Option<String>,         // 关联会话（溯源）
    pub related_path: Option<String>,       // 关联笔记路径
    pub surfaced: bool,                     // 是否已浮出给用户
    pub superseded_by: Option<String>,      // 被哪条新记忆替代
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
}

pub enum MemoryCategory {
    // user 拥有
    Profile,      // 用户基本信息（角色、背景、目标）
    Preferences,  // 用户偏好（沟通风格、主题偏好）
    Entities,     // 实体记忆（人物、项目、组织）
    Events,       // 事件记录（决策、里程碑、事故）
    // agent 拥有
    Cases,        // 学到的案例（成功方案、调试经验）
    Patterns,     // 学到的模式（行为规律、偏好趋势）
}
```

#### MemoryManager — 统一入口

```rust
pub struct MemoryManager {
    db: Arc<DbState>,
}

impl MemoryManager {
    /// 写入记忆（upsert by key，旧记忆标记 superseded_by）
    pub async fn remember(&self, memory: Memory) -> Result<(), AppError> {
        // 如果 key 已存在：旧记忆.superseded_by = 新记忆.id，再 insert 新记忆
        // 认知演进而非覆盖
    }

    /// 记忆召回：按 importance 排序，过滤 superseded_by IS NULL
    pub async fn recall(&self, query: &str, limit: u32) -> Result<Vec<Memory>, AppError> {
        // Phase 1: FTS5 关键词匹配 + importance 排序
        // Phase 2: embedding 向量语义检索
    }

    /// 按 category 召回
    pub async fn recall_by_category(&self, category: MemoryCategory, limit: u32)
        -> Result<Vec<Memory>, AppError>;

    /// 获取未浮出的记忆（ContextBuilder 注入 prompt 用）
    pub async fn unsurfaced(&self, limit: u32) -> Result<Vec<Memory>, AppError> {
        // WHERE surfaced = 0 AND superseded_by IS NULL ORDER BY importance DESC
    }

    /// 标记已浮出
    pub async fn mark_surfaced(&self, id: &str) -> Result<(), AppError>;

    /// 记忆衰减：按 category 差异化降低 importance（Cron 定期调用）
    /// Preference 衰减极慢（偏好稳定），Pattern 衰减最快（时效性强）
    pub async fn decay(&self) -> Result<u32, AppError> {
        // 按 category 差异化衰减系数：
        // UPDATE memories SET importance = importance * CASE category
        //   WHEN 'profile'     THEN 0.99   -- 用户信息稳定，几乎不衰减
        //   WHEN 'preferences' THEN 0.99   -- 偏好稳定
        //   WHEN 'entities'    THEN 0.98   -- 实体信息较稳定
        //   WHEN 'events'      THEN 0.95   -- 事件中等衰减
        //   WHEN 'cases'       THEN 0.95   -- 案例中等衰减
        //   WHEN 'patterns'    THEN 0.90   -- 模式时效性强，快速衰减
        // END
        // WHERE superseded_by IS NULL AND importance > 0.1
        // 返回受影响行数
    }

    /// 记忆升华：高 importance 观察 → 知识笔记草稿
    pub async fn propose_crystallization(&self, id: &str) -> Result<KnowledgeDraft, AppError> {
        // 取出记忆 → 生成知识草稿 → 等人类确认后写入 vault/knowledge/
    }

    /// 清理：importance 低于阈值的旧记忆
    pub async fn cleanup(&self, threshold: f32) -> Result<u32, AppError> {
        // DELETE WHERE importance < threshold AND superseded_by IS NOT NULL
    }
}
```

#### 记忆类别与示例

| category | 说明 | key 示例 |
|----------|------|---------|
| **profile** | "用户是创业者，关注教育和投资" | `profile:role_entrepreneur` |
| **preferences** | "偏好直接简洁的沟通方式" | `pref:communication_style` |
| **entities** | "张三是用户的合伙人，负责技术" | `entity:person_zhangsan` |
| **events** | "2026-03 决定转型做教育方向" | `event:pivot_education_202603` |
| **cases** | "工作压力与陪孩子质量高度相关" | `case:work_parenting_correlation` |
| **patterns** | "晚上 10 点后对话质量最高" | `pat:engagement_peak_time` |

#### 认知演进链（superseded_by）

```
记忆 A: "用户对教育有兴趣" (importance: 0.6)
  ↓ 新对话后 Agent 理解更深
记忆 B: "用户关注蒙特梭利教育方法，孩子 3 岁" (importance: 0.8)
  A.superseded_by = B.id

recall() 只返回 B（superseded_by IS NULL）
但 A 仍保留在库中，可追溯认知变化
```

#### 记忆生命周期

```
写入 → 演进 → 衰减 → 升华/清理

1. 写入：SubAgent 对话后分析 → remember() upsert by key
2. 演进：同一 key 的新认知替代旧认知（superseded_by 链）
3. 衰减：Cron 定期 decay()，importance *= 0.95
4. 升华：高 importance 观察 → propose_crystallization()
         → 知识笔记草稿 → 人类确认 → vault/knowledge/
5. 清理：被替代 + importance < 阈值的旧记忆 cleanup()
```

#### Memory 与 ContextBuilder 的关系

```rust
// ContextBuilder 从 Memory 拉取记忆注入 prompt
let memories = self.memory.recall(&message.content, 5).await?;
let unsurfaced = self.memory.unsurfaced(3).await?;
// → 注入 System Prompt 的 [Agent 记忆] 区域
```

### 6.12 Tool Layer — Agent 可用工具

Agent 上下文**常驻仅 4 个 Tool Schema**，业务操作通过 `operations` 元工具按需发现和调用，避免上下文膨胀。

```
Tools（常驻上下文，4 个 Schema）
├── filesystem   → 文件系统操作
├── shell        → 受限命令执行
├── mcp_client   → 外部 MCP 工具
└── operations   → 元工具（按需发现 Services + Memory 操作）
        ├── operations.list(category?) → 返回可用操作及参数 Schema
        └── operations.call(name, args) → 执行具体操作
```

#### Tool Trait

```rust
// src-tauri/src/tools/traits.rs

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn json_schema(&self) -> serde_json::Value;
    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, AppError>;
}
```

#### 基础能力工具

| 工具 | 文件 | 操作 | 安全约束 |
|------|------|------|---------|
| **filesystem** | `filesystem.rs` | read/write/append/list/move/delete | vault 内限定，private/ 禁入，审计日志 |
| **shell** | `shell.rs` | exec (白名单) | 白名单命令，禁管道/重定向，30s 超时，10KB 输出截断 |
| **mcp_client** | `mcp_client.rs` | call_tool, list_tools | MCP 协议调用外部工具服务 |

#### MCP Client — 接入外部工具服务

Agent 作为 MCP Client，通过 MCP 协议调用外部工具服务（如浏览器、日历、邮件等）。

```rust
// src-tauri/src/tools/mcp_client.rs

pub struct McpClientTool {
    connections: HashMap<String, McpConnection>,  // server_name → connection
}

impl McpClientTool {
    /// 连接 MCP Server（启动时从配置读取）
    pub async fn connect(&mut self, name: &str, config: McpServerConfig) -> Result<(), AppError>;

    /// 列举所有已连接 Server 的可用工具
    pub fn list_tools(&self) -> Vec<ToolSpec>;

    /// 调用外部工具
    pub async fn call_tool(&self, server: &str, tool: &str, args: serde_json::Value)
        -> Result<ToolOutput, AppError>;
}
```

MCP Server 配置（`config/settings.json`）：

```json
{
  "mcp_servers": [
    { "name": "browser", "command": "npx", "args": ["@anthropic/mcp-browser"] },
    { "name": "calendar", "command": "npx", "args": ["@anthropic/mcp-calendar"] }
  ]
}
```

#### Operations — 业务操作元工具

`operations` 是连接 Agent 与 Services/Memory 的唯一通道。Agent 通过 `list` 按需发现可用操作（含参数 Schema），再通过 `call` 执行。**操作的 JSON Schema 不常驻上下文，仅在 list 时返回**。

```rust
// src-tauri/src/tools/operations.rs

pub struct OperationsTool {
    services: Arc<ServiceContainer>,
    memory: Arc<MemoryManager>,
    registry: OperationRegistry,
}

/// 单个操作定义（Schema 按需返回，不常驻上下文）
pub struct OperationDef {
    pub name: String,           // "knowledge_create"
    pub category: String,       // "knowledge"
    pub description: String,
    pub parameters: Value,      // JSON Schema
}

impl OperationsTool {
    pub fn new(services: Arc<ServiceContainer>, memory: Arc<MemoryManager>) -> Self {
        let registry = Self::build_registry();
        Self { services, memory, registry }
    }

    fn build_registry() -> OperationRegistry {
        let mut r = OperationRegistry::new();
        // Knowledge（三级索引：L0 tags → L1 overview → L2 detail）
        r.register("knowledge_create", "knowledge", "创建知识笔记（自动生成 L0 tags + L1 overview 索引）",
            json!({"properties": {"title": {"type": "string"}, "content": {"type": "string"}, "tags": {"type": "array"}}}));
        r.register("knowledge_search", "knowledge", "搜索知识库（返回 L1 overview，支持目录递归检索）",
            json!({"properties": {"query": {"type": "string"}, "limit": {"type": "integer", "default": 5}}}));
        r.register("knowledge_get", "knowledge", "获取知识笔记完整内容（L2 detail，按需加载）",
            json!({"properties": {"path": {"type": "string"}}}));
        r.register("knowledge_list_tags", "knowledge", "列出所有 L0 tags 及频次（快速浏览知识全貌）",
            json!({"properties": {"dir": {"type": "string", "description": "可选：限定目录"}}}));
        // Daily
        r.register("daily_get", "daily", "获取/创建日记",
            json!({"properties": {"date": {"type": "string"}}}));
        r.register("daily_append", "daily", "追加内容到日记",
            json!({"properties": {"date": {"type": "string"}, "content": {"type": "string"}, "section": {"type": "string"}}}));
        // Task
        r.register("task_create", "task", "创建任务",
            json!({"properties": {"content": {"type": "string"}, "due": {"type": "string"}, "context": {"type": "string"}}}));
        r.register("task_list", "task", "列出任务",
            json!({"properties": {"status": {"type": "string"}}}));
        r.register("task_complete", "task", "完成任务",
            json!({"properties": {"id": {"type": "string"}}}));
        // Capture
        r.register("capture_submit", "capture", "快速捕获",
            json!({"properties": {"raw": {"type": "string"}, "source": {"type": "string"}}}));
        // Search（跨知识 + 记忆）
        r.register("memory_search", "search", "搜索 Agent 记忆（观察/偏好/模式）",
            json!({"properties": {"query": {"type": "string"}, "limit": {"type": "integer", "default": 5}}}));
        r
    }
}

#[async_trait]
impl Tool for OperationsTool {
    fn name(&self) -> &str { "operations" }
    fn description(&self) -> &str {
        "业务操作元工具。常用操作：knowledge_search（返回L1概要）, knowledge_get（加载L2全文）, \
         knowledge_create, knowledge_list_tags（浏览L0标签）, daily_get, daily_append, \
         task_create, task_list, task_complete, capture_submit, memory_search。\
         可直接 call(name, args)，或用 list(category?) 查看完整参数 Schema。"
    }
    fn json_schema(&self) -> Value {
        // 常驻上下文的 Schema 非常小
        json!({
            "type": "object",
            "properties": {
                "action": { "enum": ["list", "call"] },
                "category": {
                    "type": "string",
                    "description": "筛选类别: knowledge | daily | task | capture | search"
                },
                "name": {
                    "type": "string",
                    "description": "操作名称（call 时必填）"
                },
                "args": {
                    "type": "object",
                    "description": "操作参数（call 时必填）"
                }
            },
            "required": ["action"]
        })
    }

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, AppError> {
        match input.args["action"].as_str() {
            Some("list") => {
                let category = input.args["category"].as_str();
                let ops = self.registry.list(category);
                // 返回操作列表 + 参数 Schema（此时才注入上下文）
                Ok(ToolOutput::success(serde_json::to_string_pretty(&ops)?))
            }
            Some("call") => {
                let name = input.args["name"].as_str()
                    .ok_or(AppError::Validation("name required".into()))?;
                let args = input.args.get("args").cloned().unwrap_or(json!({}));
                self.dispatch(name, args).await
            }
            _ => Err(AppError::Validation("action must be 'list' or 'call'".into()))
        }
    }
}

impl OperationsTool {
    async fn dispatch(&self, name: &str, args: Value) -> Result<ToolOutput, AppError> {
        match name {
            // Knowledge
            "knowledge_create" => {
                let entry = self.services.knowledge.create(
                    args["title"].as_str().unwrap_or_default(),
                    args["content"].as_str().unwrap_or_default(),
                    &[],
                ).await?;
                Ok(ToolOutput::success(format!("Created: {}", entry.path)))
            }
            "knowledge_search" => {
                // L0 粗筛 → L1 重排序 → 返回 Top N 的 L1 overview
                let results = self.services.knowledge.search_with_rerank(
                    args["query"].as_str().unwrap_or_default(),
                    args["limit"].as_u64().unwrap_or(5) as u32,
                ).await?;
                Ok(ToolOutput::success(serde_json::to_string(&results)?))
            }
            "knowledge_get" => {
                // L2 完整加载（从文件系统读取 Markdown）
                let note = self.services.knowledge.get_l2(
                    args["path"].as_str().unwrap_or_default(),
                ).await?;
                Ok(ToolOutput::success(serde_json::to_string(&note)?))
            }
            "knowledge_list_tags" => {
                // 列出 L0 tags 及频次，Agent 可快速浏览知识全貌
                let dir = args["dir"].as_str();
                let tags = self.services.knowledge.list_tags(dir).await?;
                Ok(ToolOutput::success(serde_json::to_string(&tags)?))
            }
            // Daily
            "daily_get" => {
                let note = self.services.daily.get(
                    args["date"].as_str().unwrap_or_default(),
                ).await?;
                Ok(ToolOutput::success(serde_json::to_string(&note)?))
            }
            "daily_append" => {
                self.services.daily.append_entry(
                    args["date"].as_str().unwrap_or_default(),
                    args["content"].as_str().unwrap_or_default(),
                    args["section"].as_str(),
                ).await?;
                Ok(ToolOutput::success("Appended".into()))
            }
            // Task
            "task_create" => {
                let task = self.services.task.create(
                    args["content"].as_str().unwrap_or_default(),
                    args["due"].as_str(),
                    args["context"].as_str(),
                    None,
                ).await?;
                Ok(ToolOutput::success(format!("Task created: {}", task.id)))
            }
            "task_list" => {
                let tasks = self.services.task.list(args["status"].as_str()).await?;
                Ok(ToolOutput::success(serde_json::to_string(&tasks)?))
            }
            "task_complete" => {
                self.services.task.complete(
                    args["id"].as_str().unwrap_or_default(),
                ).await?;
                Ok(ToolOutput::success("Task completed".into()))
            }
            // Capture
            "capture_submit" => {
                let item = self.services.capture.submit(
                    args["raw"].as_str().unwrap_or_default(),
                    args["source"].as_str().unwrap_or("agent"),
                ).await?;
                Ok(ToolOutput::success(format!("Captured: {}", item.id)))
            }
            // Search（记忆）
            "memory_search" => {
                let results = self.memory.recall(
                    args["query"].as_str().unwrap_or_default(),
                    args["limit"].as_u64().unwrap_or(5) as u32,
                ).await?;
                Ok(ToolOutput::success(serde_json::to_string(&results)?))
            }
            _ => Err(AppError::Validation(format!("unknown operation: {}", name)))
        }
    }
}
```

#### Agent 调用流程示例

```
场景：Agent 需要搜索用户的学习相关知识

  // 常用操作名已在 operations.description 中列出，可直接 call 跳过 list

  Round 1: knowledge_search 返回 L1 概要（~2k tokens/条）
    tool_call("operations", {action: "call", name: "knowledge_search", args: {query: "学习方法", limit: 5}})
    返回:
      [{path: "knowledge/教育/有效学习.md", title: "有效学习方法",
        tags: ["学习", "间隔重复", "主动回忆", "费曼技巧"],
        overview: "间隔重复利用遗忘曲线...主动回忆比被动复习效果高 3 倍...费曼技巧四步法..."
       }]

  Round 2: Agent 判断需要完整内容 → 加载 L2
    tool_call("operations", {action: "call", name: "knowledge_get", args: {path: "knowledge/教育/有效学习.md"}})
    返回: 完整 Markdown 内容

  或者: Agent 想浏览知识全貌 → 列出 L0 tags
    tool_call("operations", {action: "call", name: "knowledge_list_tags", args: {dir: "knowledge/教育"}})
    返回: [{"tag": "学习", "count": 5}, {"tag": "费曼技巧", "count": 2}, ...]
```

**三级渐进加载的优势**：
- ContextBuilder 自动注入 L1（Agent 无需调工具即可感知知识全貌）
- Agent 主动搜索也返回 L1（比 500 token snippet 信息量大 4 倍，但保持结构完整）
- L2 仅在真正需要完整细节时加载，避免上下文膨胀
- `list` 仅在需要查看完整参数 Schema 或发现不常用操作时使用

#### ToolRegistry

```rust
// src-tauri/src/tools/mod.rs

impl ToolRegistry {
    pub fn default_tools(
        services: &ServiceContainer,
        memory: &MemoryManager,
        vault_path: PathBuf,
        mcp_configs: Vec<McpServerConfig>,
    ) -> Self {
        let tools: Vec<Arc<dyn Tool>> = vec![
            Arc::new(FilesystemTool::new(vault_path)),
            Arc::new(ShellTool::new_sandboxed()),
            Arc::new(McpClientTool::new(mcp_configs)),
            Arc::new(OperationsTool::new(
                Arc::new(services.clone()),
                Arc::new(memory.clone()),
            )),
        ];
        Self { tools }
    }
}
```

**工具调用循环**（在 AgentLoop.tool_loop() 中）：

```
LLM 响应 → 解析工具调用 → ToolRegistry.execute_batch()
    → 将工具结果追加到上下文 → 再次调用 Provider
    → 重复直到 LLM 不再请求工具（最多 10 轮）
    → 循环检测：输出 hash 去重，防止无限循环
```

### 6.13 Gateway Layer — HTTP/WebSocket 服务（独立模块）

Gateway 是桌面端对外暴露的网络服务层。为移动端 PWA 提供静态文件和 API，为 Webhook 通道提供接入点。

```rust
// src-tauri/src/gateway/mod.rs

pub struct GatewayServer {
    bus: Arc<MessageBus>,                          // 通过 Bus 解耦，不直接引用 Agent
    webhook_channel: Arc<WebhookChannel>,          // 实现 Channel trait，桥接 HTTP → Bus
    auth: AuthGuard,
    port: u16,  // 默认 7878，可配置
}

/// WebhookChannel：将 HTTP/WebSocket 请求桥接为 ChannelMessage → Bus
/// Webhook 端点（Telegram/Feishu/通用）通过此 Channel 推入 Bus inbound，
/// Agent 响应通过 Bus outbound 回流到 WebhookChannel.send()。
pub struct WebhookChannel {
    bus: Arc<MessageBus>,
    // 等待中的响应：request_id → oneshot::Sender（同步 HTTP 请求用）
    pending_responses: Mutex<HashMap<String, oneshot::Sender<OutboundMessage>>>,
}

impl GatewayServer {
    /// 启动 HTTP + WebSocket 服务
    pub async fn start(&self) -> Result<(), AppError> {
        // 绑定本地端口，启动 axum/actix-web 服务
    }
}
```

**REST API 端点**：

| 端点 | 方法 | 说明 | Phase |
|------|------|------|-------|
| `/api/chat` | POST | 发送消息，返回 Agent 响应 | Phase 1 后期 |
| `/api/daily/:date` | GET | 获取日记内容 | Phase 2 |
| `/api/knowledge` | GET | 知识库搜索 | Phase 2 |
| `/api/tasks` | GET | 任务列表 | Phase 2 |
| `/api/capture` | POST | 提交捕获（Webhook 入口） | Phase 1 后期 |
| `/ws/chat` | WS | WebSocket 实时对话 | Phase 2 |
| `/webhook/telegram` | POST | Telegram Bot Webhook 接收 | Phase 1 后期 |
| `/webhook/feishu` | POST | 飞书 Bot Webhook 接收 | Phase 2 |
| `/` | GET | PWA 静态文件服务 | Phase 2 |

**认证**（`gateway/auth.rs`）：

```rust
pub struct AuthGuard {
    // Bearer Token 存入 OS Keychain（与 API Key 同级安全），不存明文文件
    // 验证时从 Keychain 读取比对
    bearer_token_hash: String,  // bcrypt hash 缓存（避免每次请求读 Keychain）
}

impl AuthGuard {
    /// 验证请求：Header / Query Param / WebSocket Subprotocol
    pub fn verify(&self, request: &Request) -> Result<(), AppError>;
}
```

- 本地 WiFi 直连：Bearer Token 认证（用户在桌面端设置中生成）
- Tailscale 穿透：Tailscale 本身提供加密 + 身份验证，Gateway 再加 Token 双重保护
- Webhook：平台签名验证（Telegram: X-Telegram-Bot-Api-Secret-Token, 飞书: 签名校验）

### 6.14 Cron — 定时任务调度（独立模块）

Agent 不仅被动响应用户消息，还需主动执行后台任务。Cron 模块基于 tokio 定时驱动。

```rust
// src-tauri/src/cron/mod.rs

pub struct CronScheduler {
    agent: Arc<AgentService>,
    db: Arc<DbState>,
    jobs: Vec<CronJob>,
}

pub struct CronJob {
    pub name: String,
    pub schedule: CronSchedule,  // cron 表达式
    pub enabled: bool,
    pub last_run: Option<DateTime<Utc>>,
}
```

**内置定时任务**：

| 任务 | 默认频率 | 说明 | Phase |
|------|---------|------|-------|
| `daily_summary` | 每日 22:00 | Agent 生成当日回顾摘要，写入日记 | MVP |
| `resource_process` | 每 5 分钟 | 处理 pending 状态的资源（解析 + 结晶） | MVP |
| `history_prune` | 每日 03:00 | 压缩旧对话历史，超 90 天转冷归档 | Phase 2 |
| `knowledge_review` | 每周日 10:00 | Agent 回顾知识库，发现新关联（Layer 2） | Phase 2 |
| `index_rebuild` | 每日 04:00 | 增量重建 SQLite 索引（Markdown → notes 表） | MVP |
| `memory_surface` | 每日 09:00 | 检查未浮出记忆是否到浮出时机 | Phase 2 |
| `heartbeat_check` | 每 30 秒 | 系统健康检测 | MVP |

```rust
// src-tauri/src/cron/scheduler.rs

impl CronScheduler {
    /// 启动调度循环（应用启动时调用）
    /// 使用 tokio-cron-scheduler 精确调度，避免 loop+sleep 的时钟漂移
    pub async fn start(&mut self) -> Result<(), AppError> {
        let scheduler = JobScheduler::new().await?;
        for job in &self.jobs {
            if !job.enabled { continue; }
            let agent = self.agent.clone();
            let db = self.db.clone();
            let job_name = job.name.clone();
            scheduler.add(Job::new_async(
                job.schedule.as_str(),  // cron 表达式，如 "0 22 * * *"
                move |_uuid, _lock| {
                    let agent = agent.clone();
                    let db = db.clone();
                    let name = job_name.clone();
                    Box::pin(async move {
                        if let Err(e) = Self::run_job(&name, &agent, &db).await {
                            tracing::error!("cron job {} failed: {}", name, e);
                        }
                    })
                },
            )?).await?;
        }
        scheduler.start().await?;
        Ok(())
    }

    async fn run_job(name: &str, agent: &AgentService, db: &DbState) -> Result<(), AppError> {
        match name {
            "daily_summary" => { /* Agent 生成日记摘要 */ }
            "capture_process" => { /* 批量处理捕获队列 */ }
            "index_rebuild" => { /* 增量同步 Markdown → SQLite */ }
            _ => {}
        }
        Ok(())
    }
}
```

### 6.15 Heartbeat — 健康检测与系统状态

Heartbeat 持续监控系统各组件的运行状态，确保服务可靠性。

```rust
// src-tauri/src/heartbeat/mod.rs

pub struct HeartbeatMonitor {
    db: Arc<DbState>,
    provider: Arc<dyn Provider>,
    gateway: Option<Arc<GatewayServer>>,
    channels: Vec<Arc<dyn Channel>>,
}

/// 系统健康状态
pub struct SystemHealth {
    pub status: HealthStatus,          // healthy | degraded | down
    pub db_connected: bool,            // SQLite 连接正常
    pub api_key_valid: bool,           // Claude API Key 存在且可用
    pub vault_accessible: bool,        // Vault 目录可读写
    pub gateway_running: bool,         // Gateway 服务运行中
    pub channels: Vec<ChannelHealth>,  // 各通道状态
    pub last_check: DateTime<Utc>,
    pub uptime_seconds: u64,
}

pub struct ChannelHealth {
    pub name: String,
    pub connected: bool,
    pub last_message: Option<DateTime<Utc>>,
}

impl HeartbeatMonitor {
    /// 执行一次健康检查
    pub async fn check(&self) -> SystemHealth;

    /// 通道重连（带指数退避：2s → 4s → 8s → ... → 60s）
    pub async fn reconnect_channel(&self, name: &str) -> Result<(), AppError>;
}
```

**前端集成**：通过 IPC 命令 `system_health` 查询，Settings 页面展示系统状态。

### 6.16 System Prompt 组装

```
┌─────────────────────────────────────────────┐
│ [1] 基础人格                                 │ 固定
│     MindClaw 身份、沟通风格                │
├─────────────────────────────────────────────┤
│ [2] 模式指令                                 │ 按当前模式切换
│     陪伴 / 反思 / 挑战 / 知识 / 树洞          │
├─────────────────────────────────────────────┤
│ [3] 用户画像上下文                             │ 从 memories 表读取
│     category='profile' 的记忆                 │
├─────────────────────────────────────────────┤
│ [4] RAG 知识 L1 概要                          │ 动态检索，3-5 条
│     每条 ~2k tokens（L1 overview）            │
├─────────────────────────────────────────────┤
│ [5] 压缩对话历史                             │ 动态
│     近 5 轮完整 + 早期摘要                    │
├─────────────────────────────────────────────┤
│ [6] Agent 观察                               │ Layer 3 候选
│     未浮出的模式识别                          │
├─────────────────────────────────────────────┤
│ [7] 用户消息                                 │
└─────────────────────────────────────────────┘
```

### 6.17 模型分层调用

| 任务类型 | 模型 | 成本比 |
|---------|------|--------|
| 内容分类 · 路由 · 任务提取 | Haiku | 1x |
| 日常捕获处理 · 简单提醒判断 | Haiku | 1x |
| 知识沉淀 · 综合分析 · 异步总结 | Sonnet | ~10x |
| Layer 3 洞见生成 · 深度对话 | Sonnet | ~10x |

### 6.18 上下文工程

Token 管理是核心产品能力：

| 策略 | 实现 |
|------|------|
| 知识库注入 | L0 tags 粗筛 → L1 overview 重排序 → Top 5 L1 注入（~10k tokens） |
| 对话历史 | 近 5 轮完整 + Haiku 压缩早期为摘要 |
| Token 预算 | Haiku 默认 ≤ 16K，Sonnet 默认 ≤ 80K（settings.json 可配置） |

---
