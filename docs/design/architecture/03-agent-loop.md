# MindClaw 技术架构设计 — Agent Loop

> 完整架构文档索引见 [README.md](./README.md)

## 整体结构

```
┌─────────────────────────────────────────────────────────────┐
│                      Channel Layer                          │
│         Desktop │ Telegram Bot │ Feishu Bot                 │
└─────────────────────────┬───────────────────────────────────┘
                          │
┌─────────────────────────┼───────────────────────────────────┐
│                  Gateway Layer                              │
│         HTTP Server │ WebSocket │ Auth Guard               │
└─────────────────────────┬───────────────────────────────────┘
                          │
┌─────────────────────────┼───────────────────────────────────┐
│                  AgentLoop（驱动器）                         │
│         MessageBus │ SessionManager │ Commands │ Observer   │
│                                                             │
│    ┌───────────────────────────────────────────────────────┐│
│    │              Agent（大脑）                             ││
│    │   ContextPipeline │ Provider │ ToolRegistry │ Observer││
│    └───────────────────────────────────────────────────────┘│
└─────────────────────────┬───────────────────────────────────┘
                          │
┌─────────────────────────┼───────────────────────────────────┐
│                  Provider Layer                             │
│         OpenAI · DeepSeek · Claude · Local Embedding       │
└─────────────────────────────────────────────────────────────┘
```

架构分为两层：**Agent**（大脑）持有智能相关组件，**AgentLoop**（驱动器）负责消息编排和会话管理。Agent 无状态、无基础设施依赖，可被多个 Loop 或 SubAgent 共享。

---

## MessageBus — 双向异步消息队列

MessageBus 解耦 Channel 与 Agent，仅负责事件搬运，业务决策由 AgentLoop 决定。

```
Channel ──► publish_inbound ──► MessageBus ──► AgentLoop
                                                 │
Channel ◄── publish_outbound ◄──┘              ▼
                                     (process) ──► Outbound
```

**设计决策**：

- 事件驱动，无定时轮询
- inbound/outbound Receiver 采用 `take` 语义，单消费者
- 出站消息显式 `payload` enum，前端不解析正文状态

### 消息类型

| 方向     | 类型                      | 说明                                   |
| -------- | ------------------------- | -------------------------------------- |
| Inbound  | `InboundMessage`          | 用户消息，含 session_id, content, mode |
| Outbound | `OutboundMessage::Chunk`  | 文本片段                               |
|          | `OutboundMessage::Done`   | 完成标记                               |
|          | `OutboundMessage::Error`  | 错误信息                               |
|          | `OutboundMessage::Status` | 用户可见状态                           |

---

## 消息流水线

```
UI invoke() ──► publish_inbound ──► AgentLoop ──► SessionRuntime.enqueue
                                                            │
                                                            ▼
                                                    dispatch(message)
                                                            │
    ┌───────────────────────────────────────────────────────┼──────────┐
    │ [AgentLoop]                                           ▼          │
    │  session_mgr.get_or_create ──► SessionCommand.intercept          │
    │                                          │                       │
    │                          session_mgr.compressed_history()        │
    │                                          │                       │
    │                              ContextInput { inbound, history }   │
    │                                          │                       │
    │                                          ▼                       │
    │                         ┌── agent.run(input, cancel) ──────────┐ │
    │                         │ [Agent]                               │ │
    │                         │  Context.build                        │ │
    │                         │  Provider.chat_stream                 │ │
    │                         │    ├── TextDelta ──► observer         │ │
    │                         │    └── ToolCall  ──► tools.execute    │ │
    │                         │         └── ToolResult → next round   │ │
    │                         │  [repeat up to 8 rounds]              │ │
    │                         └───────────────────────────────────────┘ │
    │                                          │                       │
    │ [AgentLoop]  session_mgr.append_turn ◄───┘                       │
    │              OutboundMessage::Done ──► Dispatcher.send           │
    └──────────────────────────────────────────────────────────────────┘
```

**关键边界**：

- `send_message` 入队后立即返回 `{ session_id, request_id }`
- AgentLoop 保证同一 session 串行化，不允许多个 dispatch 同时执行
- Agent 内部工具循环上限 8 rounds，AgentLoop 不感知内部迭代
- `Done/Error/Status` 与 `Chunk` 分离，避免正文承载协议

---

## Agent — 无状态智能核心

Agent 是纯粹的"大脑"，持有所有智能相关组件，不知道 MessageBus、Session、消息队列的存在。

```rust
pub struct Agent {
    pub(crate) context_pipeline: Arc<ContextPipeline>,  // 上下文组装
    pub(crate) provider: Arc<dyn Provider>,              // LLM 调用
    pub(crate) tools: Arc<ToolRegistry>,                 // 工具执行
    pub(crate) observer: Arc<dyn AgentObserver>,         // 观测（共享）
}

impl Agent {
    /// Context 组装 → Provider 流式调用 → Tool 执行的完整循环，最多 8 rounds。
    /// AgentLoop 只调用此方法，不感知内部迭代细节。
    pub async fn run(
        &self,
        input: ContextInput,           // inbound + history（由 AgentLoop 预提取）
        cancel: CancellationToken,
    ) -> Result<AgentRunOutput, AppError>;
}
```

**设计原则**：

- **无状态**：不持有 history、session 等可变状态，由外部（SessionManager）管理
- **封装内部循环**：Context → Provider → Tool 的迭代由 `run()` 内部处理，对 AgentLoop 不透明
- **可共享**：多个 AgentLoop 或 SubAgent 可共享同一个 `Arc<Agent>` 实例
- **由 AgentBuilder 构建**：不需要 bus/session_mgr，但需要 ContextSource 所依赖的基础设施

### AgentBuilder

```rust
pub struct AgentBuilder {
    config: Arc<AppConfig>,
    memory: Arc<MemoryManager>,           // MemoryRecallSource 依赖
    db: Arc<DbState>,                     // RAGKnowledgeSource 依赖
    services: Arc<ServiceContainer>,      // IdentitySource 依赖
    extra_tools: Vec<Arc<dyn Tool + Send + Sync>>,
    observer: Option<Arc<dyn AgentObserver>>,
}
```

AgentBuilder 负责初始化 Provider、ToolRegistry、ContextPipeline（含各 Source 的构造注入）、Observer。

---

## AgentLoop — 事件驱动编排器

AgentLoop 是驱动器，组合 Agent（大脑）+ MessageBus（消息流）+ SessionManager（会话编排）+ Commands（拦截器）。

### 架构层级

```
MessageBus
    │
    ▼
AgentLoop
    ├── Commands (拦截 /new, /stop 等)
    ├── Session router ──► SessionRuntime (queue + active_run)
    ├── dispatch()
    │       ├── SessionManager (get/create)
    │       ├── SessionCommand.intercept()
    │       ├── session_mgr.compressed_history() ──► ContextInput
    │       ├── agent.run(input, cancel)          ← 委托，不感知内部
    │       └── SessionManager (append_turn)
    └── Observer (共享，发射 loop 层事件)
```

### AgentLoop 组成

```rust
pub struct AgentLoop {
    agent: Arc<Agent>,                                        // 大脑（可共享）
    bus: Arc<MessageBus>,                                     // 消息流
    session_mgr: Arc<SessionManager>,                         // 会话编排
    commands: Arc<SessionCommandRegistry>,                    // 命令拦截器
    observer: Arc<dyn AgentObserver>,                         // 观测（共享同一个 Arc）
    sessions: DashMap<String, Mutex<SessionRuntime>>,         // 每 session 运行时状态
}
```

**Observer 共享机制**：Agent 和 AgentLoop 持有**同一个** `Arc<dyn AgentObserver>` 实例（通过 `AgentBuilder` 创建时传入，AgentLoop 从 Agent 中获取）。Agent 发射智能层事件（ContextBuilt, ToolCallStarted, RoundCompleted），AgentLoop 发射编排层事件（RunStarted, RunCompleted），两者通过同一个 Observer 实例分发到订阅者。

**AgentLoop 职责**：

1. 消费 `InboundMessage` 并按 session 串行排队
2. 拦截 Session Commands（`/new`, `/stop`, `/restart`, `/status`）
3. 预提取会话历史，组装 `ContextInput` 传给 Agent
4. 调用 `agent.run(input, cancel)`，不感知内部 round 迭代
5. 将 run 结果持久化并映射为 `OutboundMessage`
6. 管理取消令牌与活跃 run 生命周期
7. run 完成后自旋消费同 session 的下一条消息

### Agent vs AgentLoop 职责划分

| 关注点     | Agent（大脑）                                  | AgentLoop（驱动器）                      |
| ---------- | ---------------------------------------------- | ---------------------------------------- |
| 上下文组装 | ContextPipeline                                | —                                        |
| LLM 调用   | Provider                                       | —                                        |
| 工具执行   | ToolRegistry                                   | —                                        |
| 内部循环   | Context→Provider→Tool（round 迭代，最多 8 次） | —                                        |
| 历史提取   | —                                              | session_mgr.compressed_history()         |
| 消息流     | —                                              | MessageBus                               |
| 会话管理   | —                                              | SessionManager                           |
| 命令拦截   | —                                              | SessionCommandRegistry                   |
| 串行化     | —                                              | SessionRuntime / DashMap                 |
| 取消控制   | —                                              | CancellationToken                        |
| 观测       | brain 事件（ContextBuilt, ToolCallStarted...） | loop 事件（RunStarted, RunCancelled...） |

### Session 串行化

每个 session 一个 `SessionRuntime`：

- **follow_up_queue**: 待处理的用户消息队列（Follow-up）
- **steering_queue**: 运行中注入的补充指令（Steering）
- **active_run**: 当前执行的 RunHandle（含 CancellationToken）

同一 session 同时最多一个活跃 run，后续消息入队等待。

#### Follow-up vs Steering 语义

| 维度         | Follow-up          | Steering                 |
| ------------ | ------------------ | ------------------------ |
| **触发时机** | 当前无活跃 run     | 当前有活跃 run           |
| **用户意图** | 下一条正常消息     | 打断/补充当前运行        |
| **队列**     | `follow_up_queue`  | `steering_queue`         |
| **消费时机** | run 完成后自旋消费 | 每轮结束后、下一轮开始前 |
| **效果**     | 启动新 run         | 软打断，保留已完成轮次   |

```

用户消息 A ──► dispatch(A) 执行中 ──► 用户消息 B（Steering）
│
▼
steering_queue 入队
│
当前轮次（LLM响应+工具执行）完成后
│
▼
检查 steering_queue
│
注入上下文，开始下一轮

```

#### Steering 消费策略

每轮只取一条 steering 指令（`OneAtATime`）：

- 防止用户连续发送多条 steering 指令导致上下文爆炸
- 每条 steering 作为独立消息处理，LLM 有机会响应
- 安全默认值，符合"渐进式引导"的使用模式

### 单次 run 状态机

```
[*] ──► ResolvingSession (AgentLoop: session_mgr)
          │
          ▼
    CheckingSessionCommand (AgentLoop: commands) ──► [Completed]
          │
          ▼
    PreparingContext (AgentLoop: compressed_history → ContextInput)
          │
          ▼
    ┌─── agent.run(input, cancel) ──────────────────────────────────┐
    │                        [Agent 内部]                            │
    │  BuildingContext ──► StreamingAssistant                        │
    │                            │                                   │
    │                  ┌─────────┴─────────┐                        │
    │                  ▼                   ▼                         │
    │            TextDelta            ToolCall                       │
    │                  │                   │                         │
    │                  │             ExecutingTools                  │
    │                  │                   │                         │
    │                  └─────◄─── tool results (next round)          │
    │                           [repeat, max 8 rounds]               │
    └────────────────────────────────────────────────────────────────┘
          │
          ▼
    [PersistingTurn] (AgentLoop: session_mgr) ──► [Completed]
```

**AgentLoop** 只感知 `agent.run()` 的输入/输出边界，不感知内部 round 迭代。

**运行规则**：

- 工具 round 上限：8 次 LLM 调用
- 取消：`/stop` 触发 CancellationToken，仅影响当前 session
- 超时：单次工具执行 30s 超时
- Steering vs Cancel：软打断保留已完成工具 round，硬中止丢弃

---

## 工具执行流水线

工具执行遵循三阶段生命周期：**准备 → 执行 → 终结**。支持顺序执行和并行执行两种模式。

### 执行模式

| 模式                 | 准备阶段 | 执行阶段                   | 适用场景                           |
| -------------------- | -------- | -------------------------- | ---------------------------------- |
| **Sequential**       | 逐个进行 | 逐个进行                   | 具有共享副作用的工具（如文件写入） |
| **Parallel**（默认） | 顺序预检 | 并发执行（Semaphore 限制） | 独立的只读操作（如多个 read_file） |

**关键设计**：并行模式下，准备阶段仍然是顺序运行的——`PreToolUse` Hook 可能阻止某些调用，必须在分派前确定哪些调用被阻止。所有准备完成后，未被阻止的调用并发启动。

### 三阶段流水线

```
ToolCall
    │
    ▼
[阶段 1: 准备] prepare_tool_call()
    ├── 工具查找（ToolRegistry）
    ├── 参数验证（JSON Schema）
    └── PreToolUse Hook（Agent Hooks，可阻止执行）
    │
    ▼（未被阻止）
[阶段 2: 执行] execute_tool()
    ├── 并发执行（Parallel 模式）
    ├── 超时控制（30s）
    └── 进度事件（ToolRunProgress）
    │
    ▼
[阶段 3: 终结] finalize_tool_call()
    ├── PostToolUse Hook（Agent Hooks，可覆盖结果）
    └── 构建 ToolResultMessage
```

> 工具执行的业务拦截（验证、审计、结果覆盖）统一通过 Agent Hooks 的 `PreToolUse` / `PostToolUse` 实现，详见 [03.03-tools.md](./03.03-tools.md)。

### 执行模式声明

工具通过实现 `Tool::execution_mode()` 声明自身的执行语义，默认 `Parallel`：

```rust
impl Tool for FileSystemTool {
    fn execution_mode(&self) -> ToolExecutionMode {
        ToolExecutionMode::Sequential  // 写操作必须顺序
    }
}
```

对于无法实现 `Tool` trait 的外部工具（如 MCP），`register_with_mode()` 提供覆盖入口，优先级高于 trait 方法。

### 并发控制

```rust
// 全局工具执行信号量，限制并发数
static TOOL_SEMAPHORE: Semaphore = Semaphore::new(4);

// Parallel 模式：获取 permit 后并发执行
let permits = TOOL_SEMAPHORE.acquire_many(n).await?;
let results = join_all(tools.map(|t| t.execute())).await;
```

**设计理由**：

- 防止资源爆炸（同时打开过多文件/网络连接）
- 文件写入类工具应声明为 `Sequential`，避免并行竞态

---

## SessionManager — 会话生命周期

会话保存完整 turn 记录，支持恢复、调试、历史压缩。

```
Session
├── id, sender, mode
├── turns: Vec<TurnRecord>
├── summary: Option<String>         // 早期轮次的 LLM 摘要
├── summary_covers_turns: usize     // 摘要覆盖到的 turn 索引（不含）
└── created/updated

TurnRecord
├── user_message
├── assistant_message (Option)
├── tool_trace: Vec<ToolTrace>
│       ├── tool_name, inputs, status, token_count  // 元数据，始终保留
│       └── raw_output: Option<String>              // 工具原始输出，可被微压缩清除
└── run_status
```

**核心方法**：

- `get_or_create()` — 获取或创建会话
- `append_turn()` — 追加 turn，并触发 Level 1 微压缩检查
- `prune()` — 近 N 轮完整 + 早期摘要压缩（Level 2 摘要生成）
- `compressed_history()` — 返回压缩后的 provider messages，含 Level 3 兜底

### 三级压缩策略

#### Level 1 — ToolTrace 微压缩（无 LLM，eagerly 执行）

触发：每次 `append_turn()` 完成后，自动对 `turns[0..len-3]` 执行。

操作：清除 `tool_trace[*].raw_output`（工具原始输出），保留元数据（tool_name、inputs、status、token_count）。

理由：工具输出（文件读取、搜索结果）体积大但时效短；assistant_message 已综合了重要内容；保留元数据可用于调试追溯。类比 Claude Code 的 microCompact。

#### Level 2 — Session 摘要（LLM 异步，SubAgent 执行）

触发：`append_turn()` 后，若 `session.turns.len() > SUMMARIZE_THRESHOLD`（默认 10），异步派发 `session-summarize` SubAgent。

操作：SubAgent 读取 `turns[0..len-5]`（早期轮次），生成结构化摘要后写回 `session.summary` 和 `session.summary_covers_turns`。

摘要结构：
```
- 已完成的主要任务
- 关键技术决策与代码改动
- 重要上下文（文件路径、变量名、接口约定）
- 未解决的问题与待办事项
```

特性：
- 不阻塞当前 run（摘要有滞后一轮的容忍）
- 下次 `compressed_history()` 调用时，`summary_covers_turns` 以内的轮次被摘要替代
- 类比 Claude Code 的 compact.ts（全量结构化压缩）

手动触发：`/compact` SessionCommand → 同步执行 Level 2（阻塞，返回完成状态给用户）

#### Level 3 — 紧急预算削减（同步兜底，在 compressed_history() 内）

触发：`compressed_history()` 估算 token 数超过 `ConversationHistorySource` 预算（~50K）。

操作（递进）：
```
Step 1: recent_turns 窗口收窄 5 → 3 → 1
Step 2: 仅保留 summary（若存在）
Step 3: 单轮内截断过长的 assistant_message
```

类比 Claude Code 的 autoCompact（93% 阈值触发）。

### 压缩触发汇总

| 触发条件 | 级别 | 同步/异步 | 类比 |
|---|---|---|---|
| 每次 `append_turn()` | Level 1 微压缩 | 同步 | microCompact |
| `turns.len() > 10` | Level 2 摘要 | 异步 SubAgent | compact.ts |
| `/compact` 命令 | Level 2 摘要 | 同步（阻塞） | 手动 /compact |
| `compressed_history()` token 超预算 | Level 3 削减 | 同步兜底 | autoCompact |

---

## 事件模型

三层事件分离：

| 层级                 | 类型              | 用途                                          |
| -------------------- | ----------------- | --------------------------------------------- |
| Provider → Agent     | `ProviderEvent`   | TextDelta, ToolCall, Finished                 |
| Agent/AgentLoop 内部 | `AgentEvent`      | 日志/审计/指标（RunStarted, ToolFinished...） |
| AgentLoop → 用户     | `OutboundMessage` | UI/Channel 可见事件                           |

### AgentEvent 类型

Observer 是横切关注点，Agent 和 AgentLoop 共享同一个 `Arc<dyn AgentObserver>`，各自发射所属层的事件：

**AgentLoop 层事件**（生命周期 + 编排）：

- RunStarted, SessionResolved, RunCompleted, RunCancelled, RunFailed
- CommandIntercepted, SteeringInjected

**Agent 层事件**（智能执行）：

- ContextBuilt, ProviderTextDelta, LlmToolCallRequested
- ToolCallStarted, ToolCallFinished — 一次 LLM tool_call 从接收到结果回传的完整生命周期

**Round 级别事件**（一次 `chat_stream` 调用的完整生命周期）：

- RoundStarted — 开始一次新的 LLM 调用（发送上下文到 Provider）
- RoundCompleted — 本次调用完成（所有工具执行完毕，结果已追加到上下文）

```rust
pub struct RoundStarted {
    pub round_index: usize,     // 当前 run 中的第几个 round（0-based）
    pub message_count: usize,   // 上下文消息数
}

pub struct RoundCompleted {
    pub round_index: usize,
    pub assistant_message: AssistantMessage,
    pub tool_results: Vec<ToolResult>,
}
```

**工具运行进度事件**（`ToolCallStarted` 内部，长时间运行工具的渐进式反馈）：

- ToolRunStarted — 工具开始实际执行
- ToolRunProgress — 执行中的部分结果/进度
- ToolRunCompleted — 工具执行完成

```rust
pub struct ToolRunProgress {
    pub tool_call_id: String,
    pub partial_result: ToolOutput,  // 部分结果（如搜索到的前几条）
    pub progress: Option<f32>,       // 可选进度百分比
}
```

**事件层级关系**：

```
RunStarted
    ├── RoundStarted (round 0)
    │       ├── ProviderTextDelta...
    │       ├── LlmToolCallRequested
    │       │       └── ToolCallStarted          ← tool_call 处理生命周期
    │       │               ├── ToolRunStarted   ← 实际执行开始
    │       │               ├── ToolRunProgress (多次)
    │       │               └── ToolRunCompleted
    │       │       └── ToolCallFinished
    │       └── RoundCompleted
    ├── RoundStarted (round 1)...
    └── RunCompleted
```

### OutboundMessage 与 AgentEvent 映射

| OutboundMessage（用户可见） | 触发事件                          | 说明           |
| --------------------------- | --------------------------------- | -------------- |
| `Status(Thinking)`          | `RoundStarted`                    | 开始组装上下文 |
| `Status(UsingTools)`        | `ToolCallStarted`                 | 开始执行工具   |
| `Chunk`                     | `ProviderTextDelta`               | 流式文本片段   |
| `Done`                      | `RoundCompleted` / `RunCompleted` | 完成标记       |
| `Error`                     | `RunFailed`                       | 错误信息       |

**设计原则**：OutboundMessage 是聚合后的用户可见状态，AgentEvent 是细粒度的内部观测事件。一个 OutboundMessage 可能由多个 AgentEvent 触发。

### UserVisiblePhase

- `Thinking` — 组装上下文 + 等待首 token
- `UsingTools` — 执行工具中
- `Streaming` — 输出文本

---

## 关键设计决策

| 决策         | 选择                        | 理由                               |
| ------------ | --------------------------- | ---------------------------------- |
| 每轮定义     | 一次完整 `chat_stream` 调用 | 返回多个 tool calls 按轮计更可预测 |
| 工具执行时机 | 收完 stream 后批量执行      | 避免 stream 中途暂停的复杂状态     |
| 工具结果回传 | 追加到 messages 发起新请求  | OpenAI/Claude 无状态 API 设计      |
| 工具并行     | Semaphore(4) 限制并发       | 独立 tool calls 可并行，防资源爆炸 |
| 取消检查     | 每轮开始 + HTTP abort       | 粗粒度 + 细粒度结合                |
| 运行中注入   | Steering 软打断             | 保留已完成工具轮次                 |

---

## 子章节导航

| 文件                                     | 内容                                               |
| ---------------------------------------- | -------------------------------------------------- |
| **本文件**                               | AgentLoop 核心 · MessageBus · 消息流水线 · Session |
| [03.01-context.md](./03.01-context.md)   | Context Pipeline                                   |
| [03.02-provider.md](./03.02-provider.md) | Provider 层                                        |
| [03.03-tools.md](./03.03-tools.md)       | Tool 层（MCP、Hooks、Skills）                      |
| [03.04-memory.md](./03.04-memory.md)     | Memory 层                                          |
| [03.05-services.md](./03.05-services.md) | Services 层                                        |
| [03.06-subagent.md](./03.06-subagent.md) | SubAgent                                           |
