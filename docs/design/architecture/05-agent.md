# MindClaw 技术架构设计

> 完整架构文档索引见 [README.md](./README.md)

## 六、Agent 架构

> 参考 [zeroclaw](https://github.com/zeroclaw-labs/zeroclaw) 的 Channel + Agent 分层模式。

### 6.1 整体结构

```text
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
│  ┌─────────────────────────────────────────────────────┐   │
│  │            ProviderRegistry（配置驱动）               │   │
│  │  ┌────────────────────────────────────────────────┐ │   │
│  │  │  OpenAICompatProvider（async-openai）           │ │   │
│  │  │  OpenAI · DeepSeek · Moonshot · Groq · …       │ │   │
│  │  └────────────────────────────────────────────────┘ │   │
│  │  ┌──────────────┐  ┌──────────────────────────┐     │   │
│  │  │ Claude       │  │  Local Embedding         │     │   │
│  │  │ (独立实现)    │  │  (向量检索, Phase 2)      │     │   │
│  │  └──────────────┘  └──────────────────────────┘     │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│               Infrastructure Layer                          │
│  ┌──────────────┐  ┌────────────────┐  ┌────────────────┐  │
│  │     Cron     │  │   Heartbeat    │  │    Logging     │  │
│  │  定时任务调度  │  │  健康检测/监控   │  │  tracing 日志  │  │
│  └──────────────┘  └────────────────┘  └────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```text

**Core Agent 是唯一持有完整人模型的编排器**。Channel、Gateway、Provider、Tools 都是可替换的适配层，通过 trait 解耦。Cron 和 Heartbeat 提供后台运行能力。

### 6.2 Channel 层 — 统一消息通道

Channel 是所有通信平台的抽象接口。无论消息来自桌面 UI、Telegram 还是 Feishu，Agent 看到的都是统一的 `ChannelMessage`。

```rust
// src-tauri/src/channels/traits.rs

pub struct ChannelMessage {
    pub id: String,
    pub sender: String,
    pub content: String,
    pub source: ChannelSource,
    pub timestamp: DateTime<Utc>,
    pub mode: ConversationMode,
}

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
```text

#### Channel 实现一览

| Channel | 传输方式 | 流式支持 | 入站机制 | Phase |
|---------|---------|---------|---------|-------|
| **Desktop** | Tauri IPC invoke + Event emit | Yes | Tauri command 桥接推入 Bus（listen 为空实现） | MVP |
| **Telegram** | HTTP API / Long polling | No | getUpdates 或 Webhook → Bus | Phase 1 后期 |
| **Feishu** | HTTP API / Webhook | No | Webhook → Bus | Phase 2 |
| **Webhook** | HTTP POST → Bus | No | Gateway 接收 → Bus | Phase 1 后期 |

### 6.3 MessageBus — 双向异步消息队列

MessageBus 解耦 Channel 与 Agent 的消息传递。Channel 推入站消息，Agent 推出站消息，双方互不直接引用。

```text
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
```text

**设计决策**：出站队列使 Channel 断线时消息不丢失，重连后可继续消费。使用 tokio mpsc channel 实现，Receiver 通过 `take` 语义确保单消费者。

```rust
// src-tauri/src/bus/events.rs

pub struct InboundMessage {
    pub id: String,
    pub channel_message: ChannelMessage,
    pub source: ChannelSource,
    pub reply_to: ChannelSource,
}

pub struct OutboundMessage {
    pub id: String,
    pub target: ChannelSource,
    pub session_id: String,
    pub payload: OutboundPayload,
}

pub enum OutboundPayload {
    Text(String),
    Chunk { content: String, done: bool },
    Typing(bool),
    Error(String),
}
```text

```rust
// src-tauri/src/bus/mod.rs

pub struct MessageBus {
    inbound: mpsc::Sender<InboundMessage>,
    inbound_rx: Mutex<Option<mpsc::Receiver<InboundMessage>>>,
    outbound: mpsc::Sender<OutboundMessage>,
    outbound_rx: Mutex<Option<mpsc::Receiver<OutboundMessage>>>,
    inbound_count: AtomicUsize,
    outbound_count: AtomicUsize,
}
```text

| 方法 | 调用方 | 说明 |
|------|--------|------|
| `publish_inbound(msg)` | Channel | 推送入站消息 |
| `take_inbound_rx()` | AgentLoop | 取出入站 Receiver（仅一次） |
| `publish_outbound(msg)` | AgentLoop | 推送出站消息 |
| `take_outbound_rx()` | Dispatcher | 取出出站 Receiver（仅一次） |
| `inbound_pending()` | /status | 入站队列待处理数 |
| `outbound_pending()` | /status | 出站队列待处理数 |

出站消费循环 `run_outbound_dispatcher()` 根据 `OutboundMessage.target` 路由到对应 Channel。

### 6.4 消息流水线（端到端）

```text
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
│       ├─► Hooks: PreMessage（可修改消息、注入上下文、阻止处理）  │
│       │                                                        │
│       ├─► ContextPipeline: 组装完整 prompt                     │
│       │     [人格] + [模式指令] + [角色上下文]                   │
│       │     + [RAG 知识 L1 概要] + [压缩历史] + [记忆召回]      │
│       │     + Token 预算控制（可配置，默认 Sonnet 80K）         │
│       │                                                        │
│       ├─► call_with_tools()（两阶段流式策略，最多 10 轮）      │
│       │     ┌─ stream_with_tool_detection():                   │
│       │     │   解析 SSE content_block 类型                     │
│       │     │   text → 立即推送 Chunk（用户可见）               │
│       │     │   tool_use → 静默累积（用户不可见）               │
│       │     ├─ Hooks: PreToolUse / PostToolUse                 │
│       │     ├─ 有工具调用 → ToolRegistry.execute_batch()       │
│       │     │   → 结果注入上下文 → 再次流式调用                │
│       │     │   → 循环检测（hash 去重）+ CancellationToken     │
│       │     └─ 无工具调用 → 发送 done 信号                     │
│       │     流式中断 → Error + done 信号，不追加到历史          │
│       │                                                        │
│       ├─► SessionManager: 追加消息对 + 自动裁剪                │
│       │                                                        │
│       ├─► Hooks: PostMessage（可触发副作用）                    │
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
```text

### 6.5 Agent 核心：Loop · Session · Identity

Agent 模块内部由三个核心组件驱动，职责清晰分离：

```text
              Bus.inbound (入站队列)
                        │
                        ▼
┌───────────────────────────────────────────────────────┐
│                AgentLoop (主循环)                      │
│  消费入站 → 协调 Context/Session → 调用 Provider      │
│  → 工具调用循环 → 派发 SubAgent → 推送 Bus.outbound   │
│                                                       │
│  ┌─────────────────┐  ┌────────────────────────┐      │
│  │  SessionManager │  │   ContextPipeline      │      │
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
│                        └── DailySummary               │
└───────────────────────┬───────────────────────────────┘
                        │
                        ▼
                   SendMessage (出站，立即返回)
```text

#### AgentLoop — 消息处理主循环

AgentLoop 是 Agent 的驱动引擎，从 Bus 接收消息，协调所有子组件完成响应。

```rust
// src-tauri/src/agent/agent_loop.rs

pub struct AgentLoop {
    bus: Arc<MessageBus>,
    session_mgr: Arc<SessionManager>,
    context_pipeline: Arc<ContextPipeline>,
    hooks: Arc<HookRegistry>,
    provider: Arc<dyn Provider>,
    tools: Arc<ToolRegistry>,
    memory: Arc<MemoryManager>,
    agent_commands: Arc<AgentCommandRegistry>,
    sub_agent_tx: mpsc::Sender<SubAgentDispatch>,
    identity_resolver: Arc<UserIdentityResolver>,
    cancel_token: CancellationToken,
}
```text

**`process_message()` 生命周期**：身份解析 → Session 获取 → Agent Command 拦截 → PreMessage Hook → ContextPipeline 组装 → Provider 流式调用 + 工具循环 → Session 追加 → PostMessage Hook → SubAgent 派发。详见 6.4 流水线图。

**两阶段流式策略**：Claude API 的 SSE 流中 text 和 tool_use 是不同的 content_block 类型。流式输出时实时解析 block 类型——text block 立即推送给用户保持流式体验，tool_use block 静默累积等完整解析后批量执行。这样既保证了用户的实时感受，又避免了工具调用 JSON 片段暴露给前端。工具执行完成后将结果注入上下文再次调用 Provider，循环最多 10 轮，通过输出 hash 去重防止无限循环，`CancellationToken` 支持 `/stop` 中断。

**初始化**：应用启动时 `init_agent()` 组装所有依赖注入 AgentLoop，然后 `tokio::spawn` 启动消息消费循环和出站分发循环。Bus、CancellationToken 等通过 `app.manage()` 注入 Tauri 状态供 commands 使用。

#### SessionManager — 会话生命周期管理

```rust
// src-tauri/src/agent/session.rs

pub struct Session {
    pub id: String,
    pub sender: String,
    pub mode: ConversationMode,
    pub messages: Vec<ChatMessage>,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
}

pub struct SessionManager {
    db: Arc<DbState>,
    max_turns: usize,   // 内存中最大轮数（默认 50）
    keep_recent: usize,  // 裁剪时保护的近期轮数（默认 5）
}
```text

| 方法 | 说明 |
|------|------|
| `get_or_create(sender, mode)` | 按 sender 获取或创建 session |
| `append(session_id, user_msg, agent_resp)` | 追加消息对，超限自动裁剪 |
| `prune(session)` | 折叠工具调用对 → Haiku 压缩早期消息为摘要 → 删除超龄原始消息 |
| `persist(session)` | 持久化到 SQLite |

`Session.compressed_history()` 返回压缩后的历史：近 5 轮完整 + 早期摘要。

#### UserIdentityResolver — 跨通道身份统一

MindClaw 是单用户桌面应用，但用户可能从多个通道发消息。`UserIdentityResolver` 将不同通道的 sender 映射为统一的 canonical user ID，避免会话和记忆碎片化。

```rust
// src-tauri/src/agent/identity.rs

pub struct UserIdentityResolver {
    mode: IdentityMode,
}

pub enum IdentityMode {
    SingleUser,                                          // 所有 sender → "owner"（默认）
    Mapped(HashMap<(ChannelSource, String), String>),    // 按 (source, sender) 查表
}
```text

### 6.6 Context Pipeline — 可插拔上下文组装

将上下文组装从硬编码改为有序的 `ContextSource` 管线。每个源有优先级（决定注入顺序）和独立 token 预算。Skills 可注册自定义源插入管线。

#### System Prompt 组装结构

System Prompt 由固定部分和动态检索部分组成。固定部分从配置文件加载，动态部分由 ContextSource 按需检索注入。

```text
┌─────────────────────────────────────────────────────────────┐
│  固定层（启动时加载，每次对话不变）                            │
├─────────────────────────────────────────────────────────────┤
│ SOUL.md        基础人格、沟通风格、价值观                     │
│ IDENTITY.md    模式指令（按当前模式切换对应段落）               │
│                陪伴 / 反思 / 挑战 / 知识 / 树洞               │
│ USER.md        用户基础定义（角色、背景、关系框架）             │
│ Tool Schema    4 个常驻工具的 JSON Schema                     │
├─────────────────────────────────────────────────────────────┤
│  动态层（每次对话按需检索注入）                                │
├─────────────────────────────────────────────────────────────┤
│ Memory 召回    按消息内容检索相关记忆（非全量注入）             │
│                importance 排序 + 未浮出优先                   │
│ RAG 知识       L0 粗筛 → L1 重排序 → Top N L1 注入           │
│                仅当消息与知识库有关联时才触发                   │
│ 对话历史       近 5 轮完整 + 早期摘要                         │
│ Agent 观察     未浮出的 Layer 3 模式识别（候选浮出）           │
├─────────────────────────────────────────────────────────────┤
│ 用户消息                                                     │
└─────────────────────────────────────────────────────────────┘
```text

#### ContextSource Trait 与 Pipeline

```rust
// src-tauri/src/agent/context_pipeline.rs

pub struct ContextFragment {
    pub role: MessageRole,
    pub content: String,
    pub token_estimate: usize,
    pub label: String,
}

#[async_trait]
pub trait ContextSource: Send + Sync {
    fn name(&self) -> &str;
    fn priority(&self) -> i32;          // 数值越小越先注入
    fn enabled(&self) -> bool { true }
    async fn inject(
        &self, ctx: &ContextBuildContext<'_>, budget: usize,
    ) -> Result<Vec<ContextFragment>, AppError>;
}

pub struct ContextBuildContext<'a> {
    pub message: &'a ChannelMessage,
    pub session: &'a Session,
    pub memory: &'a MemoryManager,
    pub services: &'a ServiceContainer,
    pub db: &'a DbState,
}

pub struct ContextPipeline {
    sources: Vec<Arc<dyn ContextSource>>,
    total_budget: usize,
    budget_allocations: HashMap<String, usize>,
}
```text

构建逻辑：按 priority 顺序遍历所有 source，每个 source 在分配的 budget 内注入 fragment，累计消耗超过 total_budget 时后续 source 被压缩或跳过。

#### 内置源映射

| Source | Priority | 默认预算 | 数据来源 | 注入方式 |
|--------|----------|---------|---------|---------|
| `SystemPromptSource` | 0 | ~2K | SOUL.md + IDENTITY.md + USER.md + Tool Schema | 固定加载 |
| `RAGKnowledgeSource` | 10 | ~10K | knowledge/ 目录 | 按消息内容检索，search_with_rerank top 5 L1 |
| `MemoryRecallSource` | 20 | ~2K | memories 表 | 按消息内容检索 + 未浮出优先 |
| `ConversationHistorySource` | 30 | ~50K | session 历史 | 近 5 轮完整 + 早期摘要 |
| `UserMessageSource` | 100 | ~1K | 当前消息 | 直接注入（始终最后） |

Skills 注册的自定义 ContextSource 按 priority 自动插入管线。例如 "weekly_context" source（priority: 15）在 RAG 之后、Memory 之前注入本周关键事项。

#### Token 预算管理

| 策略 | 实现 |
|------|------|
| 知识库注入 | L0 tags 粗筛 → L1 overview 重排序 → Top 5 L1 注入（~10k tokens） |
| 对话历史 | 近 5 轮完整 + Haiku 压缩早期为摘要 |
| Token 预算 | Haiku 默认 ≤ 16K，Sonnet 默认 ≤ 80K（settings.json 可配置） |

超预算裁剪顺序：先减 RAG 片段数 → 再压缩历史 → 最后截断观察。

#### settings.json 配置

```json
{
  "context_pipeline": {
    "total_budget": 80000,
    "allocations": {
      "system_prompt": 2000,
      "rag_knowledge": 10000,
      "memory_recall": 2000,
      "conversation_history": 50000,
      "user_message": 1000
    },
    "disabled_sources": []
  }
}
```text

### 6.7 Provider 层 — LLM 抽象

Provider 是独立于 Agent 的顶层模块，通过 trait 注入 AgentLoop。未来可替换为 OpenAI、Ollama 等实现。API Key 从 OS Keychain 读取（桌面端）或直接传入（CLI）。

```rust
// src-tauri/src/providers/traits.rs

pub enum ModelTier {
    Haiku,   // 路由、分类、简单任务
    Sonnet,  // 深度对话、知识沉淀、洞见生成
}

#[async_trait]
pub trait Provider: Send + Sync {
    async fn chat(
        &self, model: ModelTier, messages: &[ChatMessage],
    ) -> Result<ProviderResponse, AppError>;
    async fn chat_stream(
        &self, model: ModelTier, messages: &[ChatMessage],
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent, AppError>> + Send>>, AppError>;
    fn supports_streaming(&self) -> bool;
    fn max_tokens(&self, model: ModelTier) -> usize;
}
```text

#### 模型分层调用

| 任务类型 | 模型 | 成本比 |
|---------|------|--------|
| 内容分类 · 路由 · 任务提取 | Haiku | 1x |
| 日常输入处理 · 简单提醒判断 | Haiku | 1x |
| 知识沉淀 · 综合分析 · 异步总结 | Sonnet | ~10x |
| Layer 3 洞见生成 · 深度对话 | Sonnet | ~10x |

### 6.8 Tool 层 — Agent 可用工具

Agent 上下文**常驻仅 4 个 Tool Schema**，业务操作通过 `operations` 元工具按需发现和调用，避免上下文膨胀。

```text
Tools（常驻上下文，4 个 Schema）
├── filesystem   → 文件系统操作
├── shell        → 受限命令执行
├── mcp_client   → 外部 MCP 工具
└── operations   → 元工具（按需发现 Services + Memory 操作）
        ├── operations.list(category?) → 返回可用操作及参数 Schema
        └── operations.call(name, args) → 执行具体操作
```text

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
```text

#### 基础能力工具

| 工具 | 操作 | 安全约束 |
|------|------|---------|
| **filesystem** | read/write/append/list/move/delete | vault 内限定，private/ 禁入，审计日志 |
| **shell** | exec (白名单) | 白名单命令，禁管道/重定向，30s 超时，10KB 输出截断 |
| **mcp_client** | call_tool, list_tools | MCP 协议调用外部工具服务 |

#### MCP Client — 接入外部工具服务

Agent 作为 MCP Client，通过 MCP 协议调用外部工具服务（浏览器、日历、邮件等）。

```rust
// src-tauri/src/tools/mcp_client.rs

pub struct McpClientTool {
    connections: HashMap<String, McpConnection>,
}
```text

| 方法 | 说明 |
|------|------|
| `connect(name, config)` | 连接 MCP Server |
| `list_tools()` | 列举所有已连接 Server 的可用工具 |
| `call_tool(server, tool, args)` | 调用外部工具 |

MCP Server 配置（`~/MindClaw/config/settings.json`）：

```json
{
  "mcp_servers": [
    { "name": "browser", "command": "npx", "args": ["@anthropic/mcp-browser"] },
    { "name": "calendar", "command": "npx", "args": ["@anthropic/mcp-calendar"] }
  ]
}
```text

**注意**：MCP Server 配置存储在用户数据目录 `~/MindClaw/config/settings.json`，而非代码目录。该文件不包含敏感信息（如 API Key），可以明文存储。

#### Operations — 业务操作元工具

`operations` 是连接 Agent 与 Services/Memory 的唯一通道。**操作的 JSON Schema 不常驻上下文，仅在 list 时返回**。常用操作名已列在 `operations.description` 中，Agent 可直接 call 跳过 list。

```rust
// src-tauri/src/tools/operations.rs

pub struct OperationDef {
    pub name: String,
    pub category: String,
    pub description: String,
    pub parameters: Value,  // JSON Schema
}
```text

常驻上下文的 Schema（极小）：

```json
{
  "type": "object",
  "properties": {
    "action": { "enum": ["list", "call"] },
    "category": { "type": "string", "description": "筛选: knowledge | daily | task | search" },
    "name": { "type": "string", "description": "操作名称（call 时必填）" },
    "args": { "type": "object", "description": "操作参数（call 时必填）" }
  },
  "required": ["action"]
}
```text

#### 已注册操作

| 操作名 | 类别 | 说明 |
|--------|------|------|
| `knowledge_create` | knowledge | 创建知识笔记（自动生成 L0 tags + L1 overview） |
| `knowledge_search` | knowledge | 搜索知识库（返回 L1 overview，支持目录递归） |
| `knowledge_get` | knowledge | 获取笔记完整内容（L2 detail，按需加载） |
| `knowledge_list_tags` | knowledge | 列出 L0 tags 及频次（浏览知识全貌） |
| `daily_get` | daily | 获取/创建日记 |
| `daily_append` | daily | 追加内容到日记 |
| `task_create` | task | 创建任务 |
| `task_list` | task | 列出任务 |
| `task_complete` | task | 完成任务 |
| `memory_search` | search | 搜索 Agent 记忆 |

#### 三级渐进加载

```text
场景：Agent 搜索用户的学习相关知识

  Round 1: knowledge_search 返回 L1 概要（~2k tokens/条）
    → [{path, title, tags, overview: "间隔重复利用遗忘曲线..."}]

  Round 2: Agent 需要完整内容 → knowledge_get 加载 L2
    → 完整 Markdown 内容

  或者: Agent 浏览知识全貌 → knowledge_list_tags
    → [{"tag": "学习", "count": 5}, ...]
```text

**优势**：ContextPipeline 自动注入 L1（Agent 无需调工具即可感知知识全貌）；Agent 主动搜索也返回 L1（比 500 token snippet 信息量大 4 倍）；L2 仅在真正需要完整细节时加载，避免上下文膨胀。

#### 工具调用循环

```text
LLM 响应 → 解析工具调用 → ToolRegistry.execute_batch()
    → 将工具结果追加到上下文 → 再次调用 Provider
    → 重复直到 LLM 不再请求工具（最多 10 轮）
    → 循环检测：输出 hash 去重，防止无限循环
```text

### 6.9 Services 层 — 核心业务逻辑

Services 是业务操作的核心层。**Web Commands、CLI Commands 和 Agent 共用同一套 Services**，保证业务逻辑单一来源。

```text
Web Commands  ──► Services ──► Storage
CLI Commands  ──► Services ──► Storage
Agent         ──► operations (元工具) ──► Services ──► Storage
                                     ──► Memory   ──► Storage
```text

```rust
// src-tauri/src/services/mod.rs

pub struct ServiceContainer {
    pub knowledge: KnowledgeService,
    pub daily: DailyService,
    pub task: TaskService,
}
```text

#### KnowledgeService — 知识笔记管理

操作人机共有的知识体系（Markdown 文件 + SQLite 索引）。检索采用三级渐进：L0 粗筛 → L1 重排序 → L2 按需加载。`search_with_rerank` 中命中目录（path 无 .md 后缀）时自动展开子笔记补充候选。

| 方法 | 说明 |
|------|------|
| `create(title, content, tags)` | 写 Markdown + 提取 L0 tags + 生成 L1 + 更新 FTS5 |
| `update(path, content)` | 更新笔记，自动更新 L0/L1 索引 |
| `search_l0(query, limit)` | FTS5 匹配 title + tags，返回候选集（~100 tokens/条） |
| `get_l1_batch(paths)` | 批量加载 L1 overview（~2k tokens/条） |
| `get_l2(path)` | 从文件系统读取完整 Markdown |
| `search_with_rerank(query, top_n)` | L0 粗筛 → 目录递归 → L1 重排序 → Top N |
| `list(tag?)` | 按标签筛选 |
| `list_children(parent)` | 列出目录下直接子节点 |
| `sync_links(path)` | 提取 wikilinks 并更新 links 表 |
| `rebuild_index(path?)` | 重建 Markdown → SQLite 索引 |

```rust
pub struct NoteL0 {
    pub path: String,       // 有 .md = 笔记，无后缀 = 目录
    pub title: String,
    pub tags: Vec<String>,
}

pub struct NoteL1 {
    pub path: String,
    pub title: String,
    pub tags: Vec<String>,
    pub overview: String,   // ~2k tokens
}
```text

#### DailyService — 日记管理

| 方法 | 说明 |
|------|------|
| `get(date)` | 获取日记（不存在则从模板创建）+ 关联任务 |
| `save(date, content)` | 保存日记内容 |
| `append_entry(date, content, section?)` | 追加条目到指定区域 |
| `list(limit)` | 日记列表（元数据） |

#### TaskService — 任务管理

| 方法 | 说明 |
|------|------|
| `create(content, due?, context?, note_path?)` | 创建任务 |
| `update(id, status?, content?, due?)` | 更新任务 |
| `list(status?)` | 列出任务 |
| `complete(id)` | 完成任务 |

### 6.10 Memory 层 — Agent 私有记忆

> PRD 核心命题：**记忆是 Agent 的，知识是共同的。**

Memory 管理 Agent 对用户的私有认知——观察、偏好、模式识别等。存在 SQLite 中，用户不直接操作。Knowledge（Markdown）是人机共有的，由 Services 管理。

```text
Memory (Agent 私有, SQLite)          Knowledge (人机共有, Markdown)
├── 观察：第三次提到工作疲惫感        ├── vault/knowledge/工作节奏.md
├── 偏好：偏好简短直接的回复           ├── vault/knowledge/投资策略.md
├── 模式：周一情绪通常低落             └── vault/knowledge/育儿方法.md
└── 召回：按相关性检索记忆
                                      ↑
    记忆可以升华为知识 ────────────────┘
    （Agent 发现模式 → 沉淀为知识笔记，需人类确认）
```text

#### 单表 `memories` 设计

所有记忆存入单表，通过 `category` 区分类型，`key` 去重，`superseded_by` 追踪认知演进：

```rust
// src-tauri/src/memory/mod.rs

pub struct Memory {
    pub id: String,
    pub key: String,                     // 唯一去重键，同一认知 upsert
    pub content: String,
    pub category: MemoryCategory,        // 6 类，隐含 owner（user/agent）
    pub importance: f32,                 // 重要度 0.0-1.0
    pub session_id: Option<String>,      // 关联会话（溯源）
    pub related_path: Option<String>,    // 关联笔记路径
    pub surfaced: bool,                  // 是否已浮出给用户
    pub superseded_by: Option<String>,   // 被哪条新记忆替代
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
```text

#### 记忆类别与示例

| category | 说明 | key 示例 |
|----------|------|---------|
| **profile** | "用户是创业者，关注教育和投资" | `profile:role_entrepreneur` |
| **preferences** | "偏好直接简洁的沟通方式" | `pref:communication_style` |
| **entities** | "张三是用户的合伙人，负责技术" | `entity:person_zhangsan` |
| **events** | "2026-03 决定转型做教育方向" | `event:pivot_education_202603` |
| **cases** | "工作压力与陪孩子质量高度相关" | `case:work_parenting_correlation` |
| **patterns** | "晚上 10 点后对话质量最高" | `pat:engagement_peak_time` |

#### MemoryManager

```rust
pub struct MemoryManager {
    db: Arc<DbState>,
}
```text

| 方法 | 说明 |
|------|------|
| `remember(memory)` | 写入记忆（upsert by key，旧记忆标记 superseded_by） |
| `recall(query, limit)` | FTS5 关键词匹配 + importance 排序（Phase 2: embedding 向量检索） |
| `recall_by_category(category, limit)` | 按类别召回 |
| `unsurfaced(limit)` | 获取未浮出的记忆（ContextPipeline 注入用） |
| `mark_surfaced(id)` | 标记已浮出 |
| `decay()` | 按 category 差异化衰减 importance（Cron 定期调用） |
| `propose_crystallization(id)` | 高 importance 记忆 → 知识笔记草稿（需人类确认） |
| `cleanup(threshold)` | 删除 superseded + importance 低于阈值的旧记忆 |

#### 衰减系数

| category | 衰减系数 | 原因 |
|----------|---------|------|
| profile | 0.99 | 用户信息稳定 |
| preferences | 0.99 | 偏好稳定 |
| entities | 0.98 | 实体信息较稳定 |
| events | 0.95 | 事件中等衰减 |
| cases | 0.95 | 案例中等衰减 |
| patterns | 0.90 | 模式时效性强，快速衰减 |

#### 认知演进链（superseded_by）

**设计决策**：记忆更新不覆盖旧值，而是通过 `superseded_by` 链追踪认知变化。`recall()` 只返回 `superseded_by IS NULL` 的最新认知，但旧记忆保留可追溯。

```text
记忆 A: "用户对教育有兴趣" (importance: 0.6)
  ↓ 新对话后 Agent 理解更深
记忆 B: "用户关注蒙特梭利教育方法，孩子 3 岁" (importance: 0.8)
  A.superseded_by = B.id
```text

#### 记忆生命周期

```text
写入 → 演进 → 衰减 → 升华/清理

1. 写入：SubAgent 对话后分析 → remember() upsert by key
2. 演进：同一 key 的新认知替代旧认知（superseded_by 链）
3. 衰减：Cron 定期 decay()，importance *= 衰减系数
4. 升华：高 importance 观察 → propose_crystallization()
         → 知识笔记草稿 → 人类确认 → vault/knowledge/
5. 清理：被替代 + importance < 阈值的旧记忆 cleanup()
```text

### 6.11 SubAgent — 异步后台任务

AgentLoop 负责主对话流，SubAgent 处理不应阻塞响应的后台任务。从硬编码 enum 演进为 trait + 注册表模式，Skills 可动态添加新任务类型。

```text
AgentLoop (主对话)
    │
    ├── 对话响应 → 立即返回给用户
    │
    └── 派发 SubAgent 任务（不阻塞）
         ├── KnowledgeDistill:  从对话中提炼知识笔记
         ├── SessionSummarize:  会话摘要生成
         ├── ObservationAnalyze: Layer 3 模式识别
         └── DailySummary:      当日回顾生成
```text

#### SubAgentTask Trait

```rust
// src-tauri/src/agent/sub_agent.rs

#[async_trait]
pub trait SubAgentTask: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn model_tier(&self) -> ModelTier;
    async fn execute(
        &self, ctx: SubAgentContext, input: Value,
    ) -> Result<SubAgentOutput, AppError>;
}

pub struct SubAgentContext {
    pub provider: Arc<dyn Provider>,
    pub db: Arc<DbState>,
    pub memory: Arc<MemoryManager>,
    pub services: Arc<ServiceContainer>,
}

pub struct SubAgentOutput {
    pub success: bool,
    pub summary: String,
    pub artifacts: Vec<Artifact>,
}

pub struct Artifact {
    pub kind: String,    // "knowledge_draft" | "summary" | "observation"
    pub content: String,
    pub metadata: Value,
}
```text

#### SubAgentRegistry 与派发

```rust
pub struct SubAgentRegistry {
    tasks: HashMap<String, Arc<dyn SubAgentTask>>,
}

pub struct SubAgentDispatch {
    pub task_name: String,
    pub input: Value,
}
```text

`SubAgentRegistry::with_builtins()` 注册 4 个内置任务。`SubAgentExecutor` 消费 `mpsc::Receiver<SubAgentDispatch>`，通过 `Semaphore` 限制最大 3 个并发 API 调用，防止速率爆炸。

#### 内置任务与模型选择

| 任务 | model_tier() | 原因 |
|------|-------------|------|
| `knowledge_distill` | Sonnet | 需深度理解和提炼 |
| `session_summarize` | Haiku | 摘要生成，低成本 |
| `observation_analyze` | Sonnet | 跨域关联和模式识别 |
| `daily_summary` | Sonnet | 综合当日全部信息 |

**派发时机**：`post_process()` 中，知识模式下派发 KnowledgeDistill，每次对话后派发 ObservationAnalyze，会话结束时派发 SessionSummarize。

### 6.12 扩展性：Hooks · Skills · 基础设施

#### Agent Hooks — 事件钩子系统

AgentLoop 在关键位置引入事件驱动的扩展点，支持 Rust trait 实现和 Shell 命令两种 handler 类型。

```text
process_message() 中的 Hook 触发点：

  1. Identity resolution
  2. Session get_or_create ──► OnSessionCreate（新会话时）
  3. Agent Command interception
  4. ► PreMessage ◄ ──────── 可修改消息、注入额外上下文、或阻止处理
  5. ContextPipeline.build()
  6. call_with_tools() 工具循环内：
     ├── ► PreToolUse ◄ ──── 可验证/修改输入、或阻止工具执行
     ├── tools.execute_batch()
     └── ► PostToolUse ◄ ─── 可审计/修改工具输出
  7. Session append
  8. ► PostMessage ◄ ──────── 可触发副作用（通知、分析等）
  9. post_process (SubAgent dispatch)
  10. Session close ──────── ► OnSessionClose ◄
```text

```rust
// src-tauri/src/agent/hooks.rs

pub enum HookEvent {
    PreMessage,
    PostMessage,
    PreToolUse,
    PostToolUse,
    OnSessionCreate,
    OnSessionClose,
}

pub enum HookResult<T> {
    Continue,
    Modified(T),
    Block(String),
}

#[async_trait]
pub trait HookHandler: Send + Sync {
    fn name(&self) -> &str;
    fn event(&self) -> HookEvent;
    fn priority(&self) -> i32 { 0 }
    async fn on_pre_message(&self, _ctx: &HookContext<'_>, payload: PreMessagePayload)
        -> Result<HookResult<PreMessagePayload>, AppError> { Ok(HookResult::Continue) }
    async fn on_post_message(&self, _ctx: &HookContext<'_>, message: &ChannelMessage, response: &AgentResponse)
        -> Result<(), AppError> { Ok(()) }
    async fn on_pre_tool_use(&self, _ctx: &HookContext<'_>, payload: PreToolUsePayload)
        -> Result<HookResult<PreToolUsePayload>, AppError> { Ok(HookResult::Continue) }
    async fn on_post_tool_use(&self, _ctx: &HookContext<'_>, payload: PostToolUsePayload)
        -> Result<HookResult<PostToolUsePayload>, AppError> { Ok(HookResult::Continue) }
}
```text

**Shell 命令钩子**（Claude Code 风格）：通过 `settings.json` 配置，无需编译。

```rust
pub struct CommandHook {
    pub name: String,
    pub event: HookEvent,
    pub matcher: Option<String>,  // 工具名匹配（仅 ToolUse 事件）
    pub command: String,          // 支持 ${tool_name} ${tool_input} 变量
    pub timeout_ms: u64,
    pub priority: i32,
}
```text

```json
{
  "hooks": {
    "PreToolUse": [
      { "name": "audit-shell", "matcher": "shell",
        "command": "echo '${tool_name}: ${tool_input}' >> ~/MindClaw/data/audit.log" }
    ],
    "PostMessage": [
      { "name": "notify-mobile",
        "command": "curl -s https://api.telegram.org/bot${TG_TOKEN}/sendMessage ..." }
    ]
  }
}
```text

`HookRegistry` 按 `priority` 排序执行所有 handler，遇到 `Block` 立即返回阻止后续流程。

#### Agent Skills — 技能系统

Skill 是核心扩展机制。一个技能包可提供 Tools、ContextSources、HookHandlers、SubAgentTasks、Operations 的任意组合，通过 SkillRegistry 统一分发到各注册表。

```mermaid
graph TB
    subgraph "Skill Package"
        S["Skill trait"]
        S --> T["Tools"]
        S --> CS["ContextSources"]
        S --> H["HookHandlers"]
        S --> SA["SubAgentTasks"]
        S --> O["Operations"]
    end

    subgraph "Core Registries"
        TR["ToolRegistry"]
        CP["ContextPipeline"]
        HR["HookRegistry"]
        SR["SubAgentRegistry"]
        OR["OperationRegistry"]
    end

    T --> TR
    CS --> CP
    H --> HR
    SA --> SR
    O --> OR

    subgraph "AgentLoop"
        PM["process_message"]
        CWT["call_with_tools"]
        PP["post_process"]
    end

    HR -.-> PM
    CP -.-> PM
    TR -.-> CWT
    HR -.-> CWT
    SR -.-> PP
```text

```rust
// src-tauri/src/agent/skills.rs

pub trait Skill: Send + Sync {
    fn manifest(&self) -> &SkillManifest;
    fn tools(&self) -> Vec<Arc<dyn Tool>> { vec![] }
    fn context_sources(&self) -> Vec<Arc<dyn ContextSource>> { vec![] }
    fn hooks(&self) -> Vec<Arc<dyn HookHandler>> { vec![] }
    fn sub_agent_tasks(&self) -> Vec<Arc<dyn SubAgentTask>> { vec![] }
    fn operations(&self) -> Vec<OperationDef> { vec![] }
    fn init(&self, _ctx: &SkillInitContext) -> Result<(), AppError> { Ok(()) }
}

pub struct SkillManifest {
    pub name: String,
    pub version: String,
    pub description: String,
}
```text

```toml
# config/skills/weekly-report/skill.toml
[skill]
name = "weekly-report"
version = "0.1.0"
description = "Generate weekly reports from knowledge and daily notes"

[provides]
tools = ["weekly_report"]
context_sources = ["weekly_context"]
hooks = { PostMessage = "on_weekly_check" }
sub_agent_tasks = ["weekly_report_generate"]
operations = ["weekly_report_generate", "weekly_report_list"]
```text

#### 扩展性分阶段实现

| 组件 | MVP | Phase 1 后期 | Phase 2 |
|------|-----|-------------|---------|
| HookRegistry（Rust handlers） | Y | | |
| HookRegistry（command hooks） | | Y | |
| ContextPipeline（内置源） | Y | | |
| ContextPipeline（自定义源） | | Y | |
| SubAgentRegistry（内置任务） | Y | | |
| SubAgentRegistry（自定义任务） | | Y | |
| SkillRegistry（built-in skills） | | Y | |
| SkillRegistry（外部加载/WASM） | | | Y |

#### Gateway Layer — HTTP/WebSocket 服务

Gateway 为移动端 PWA 提供静态文件和 API，为 Webhook 通道提供接入点。通过 Bus 解耦，不直接引用 Agent。

| 端点 | 方法 | 说明 | Phase |
|------|------|------|-------|
| `/api/chat` | POST | 发送消息，返回 Agent 响应 | Phase 1 后期 |
| `/api/daily/:date` | GET | 获取日记内容 | Phase 2 |
| `/api/knowledge` | GET | 知识库搜索 | Phase 2 |
| `/api/tasks` | GET | 任务列表 | Phase 2 |
| `/ws/chat` | WS | WebSocket 实时对话 | Phase 2 |
| `/webhook/telegram` | POST | Telegram Bot Webhook | Phase 1 后期 |
| `/webhook/feishu` | POST | 飞书 Bot Webhook | Phase 2 |
| `/` | GET | PWA 静态文件服务 | Phase 2 |

**认证**：

| 场景 | 认证方式 | 说明 |
|------|---------|------|
| 本地 WiFi（PWA /api/*） | Bearer Token | Token 存储在 OS Keychain，客户端请求时携带 |
| Tailscale 远程接入 | 双重保护 | Bearer Token + Tailscale 身份验证 |
| Webhook（Telegram/Feishu） | 平台签名验证 | 验证平台发送的签名（如 Telegram 的 `X-Telegram-Date`），**不需要** Bearer Token |
| WebSocket（/ws/chat） | Bearer Token | 连接时认证，之后保持会话 |

#### Cron — 定时任务调度

| 任务 | 默认频率 | 说明 | Phase |
|------|---------|------|-------|
| `daily_summary` | 每日 22:00 | 生成当日回顾，写入日记 | MVP |
| `resource_process` | 每 5 分钟 | 处理 pending 资源（解析 + 结晶） | MVP |
| `history_prune` | 每日 03:00 | 压缩旧对话历史，超 90 天转冷归档 | Phase 2 |
| `knowledge_review` | 每周日 10:00 | 回顾知识库，发现新关联 | Phase 2 |
| `index_rebuild` | 每日 04:00 | 增量重建 Markdown → SQLite 索引 | MVP |
| `memory_surface` | 每日 09:00 | 检查未浮出记忆的浮出时机 | Phase 2 |
| `heartbeat_check` | 每 30 秒 | 系统健康检测 | MVP |

基于 `tokio-cron-scheduler` 精确调度，避免 loop+sleep 的时钟漂移。

#### Heartbeat — 健康检测

```rust
pub struct SystemHealth {
    pub status: HealthStatus,          // healthy | degraded | down
    pub db_connected: bool,
    pub api_key_valid: bool,
    pub vault_accessible: bool,
    pub gateway_running: bool,
    pub channels: Vec<ChannelHealth>,
    pub last_check: DateTime<Utc>,
    pub uptime_seconds: u64,
}
```text

通道断线时自动重连，指数退避（2s → 4s → 8s → ... → 60s）。前端通过 IPC 命令 `system_health` 查询，Settings 页面展示。
