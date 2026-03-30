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
│                  Core Agent Service                         │
│                                                             │
│    ContextBuilder │ SessionManager │ Router │ Observer      │
│                                                             │
│    ┌───────────────────────┐   ┌──────────────────────────┐ │
│    │     Tool Registry     │   │   Memory / Knowledge     │ │
│    └───────────────────────┘   └──────────────────────────┘ │
└─────────────────────────┬───────────────────────────────────┘
                          │
┌─────────────────────────┼───────────────────────────────────┐
│                  Provider Layer                             │
│         OpenAI · DeepSeek · Claude · Local Embedding       │
└─────────────────────────────────────────────────────────────┘
```

Core Agent 是唯一持有完整人格模型的编排器，其他层通过 trait 解耦。

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

| 方向 | 类型 | 说明 |
|------|------|------|
| Inbound | `InboundMessage` | 用户消息，含 session_id, content, mode |
| Outbound | `OutboundPayload::Chunk` | 文本片段 |
| | `OutboundPayload::Done` | 完成标记 |
| | `OutboundPayload::Error` | 错误信息 |
| | `OutboundPayload::Status` | 用户可见状态 |

---

## 消息流水线

```
UI invoke() ──► publish_inbound ──► AgentLoop.consume ──► Session.enqueue
                                                            │
                                                            ▼
                                                    run_once(message)
                                                            │
    ┌───────────────────────────────────────────────────────┼──────────┐
    │                                                       ▼          │
    │  Session.get_or_create ──► AgentCommand.intercept ──► Context.build
    │                                                           │      │
    │  Provider.chat_stream ◄───────────────────────────────────┘      │
    │       │                                                          │
    │       ├── TextDelta ──► Outbound.Chunk                          │
    │       └── ToolCall ──► Tool.execute ──► next round               │
    │                                                                  │
    │  Session.append_turn ──► Outbound.Done ──► Dispatcher.send      │
    └──────────────────────────────────────────────────────────────────┘
```

**关键边界**：
- `send_message` 入队后立即返回 `{ session_id, request_id }`
- AgentLoop 保证同一 session 串行化，不允许多个 run 同时执行
- 工具回合是单次 run 内的有限 loop（最多 8 轮）
- `Done/Error/Status` 与 `Chunk` 分离，避免正文承载协议

---

## AgentLoop — 事件驱动编排器

### 架构层级

```
MessageBus
    │
    ▼
AgentLoop
    ├── Session router ──► SessionSlot (queue + active_run)
    ├── run_once()
    │       ├── SessionManager (get/create)
    │       ├── ContextPipeline (build)
    │       ├── Provider (chat_stream)
    │       ├── ToolRegistry (execute)
    │       └── SessionManager (append_turn)
    └── Observer / trace
```

**AgentLoop 职责**：
1. 消费 `InboundMessage` 并按 session 串行排队
2. 为每条消息创建单次 `run_once()` 执行
3. 驱动 Session → Context → Provider → Tool → Session append
4. 映射运行态为 `OutboundPayload` 和观测事件
5. 管理取消令牌与活跃 run 生命周期
6. run 完成后自旋消费同 session 的下一条消息

### Session 串行化

每个 session 一个 `SessionSlot`：
- **queue**: 待处理消息队列
- **active_run**: 当前执行的 RunHandle（含 CancellationToken）
- **steering_queue**: 运行中注入的补充指令

同一 session 同时最多一个活跃 run，后续消息入队等待。

### 单次 run 状态机

```
[*] ──► ResolvingSession
          │
          ▼
    CheckingAgentCommand ──► [Completed] (command intercepted)
          │
          ▼
    BuildingContext ──► StreamingAssistant
                              │
                    ┌─────────┴─────────┐
                    ▼                   ▼
              TextDelta            ToolCall
                    │                   │
                    │                   ▼
                    │             ExecutingTools
                    │                   │
                    └────────◄─── tool results (next round)
                              │
                    [PersistingTurn] ──► [Completed]
```

**运行规则**：
- 工具回合上限：8 轮 LLM 调用
- 取消：`/stop` 触发 CancellationToken，仅影响当前 session
- 超时：单轮工具执行 30s 超时
- Steering vs Cancel：软打断保留已完成工具轮次，硬中止丢弃

---

## SessionManager — 会话生命周期

会话保存完整 turn 记录，支持恢复、调试、历史压缩。

```
Session
├── id, sender, mode
├── turns: Vec<TurnRecord>
└── created/updated

TurnRecord
├── user_message
├── assistant_message (Option)
├── tool_trace: Vec<ToolTrace>
└── run_status
```

**核心方法**：
- `get_or_create()` — 获取或创建会话
- `append_turn()` — 成功完成后追加 turn
- `prune()` — 近 N 轮完整 + 早期摘要压缩
- `compressed_history()` — 返回压缩后的 provider messages

---

## 事件模型

三层事件分离：

| 层级 | 类型 | 用途 |
|------|------|------|
| Provider → Agent | `ProviderEvent` | TextDelta, ToolCall, Finished |
| Agent 内部 | `AgentEvent` | 日志/审计/指标（RunStarted, ToolFinished...） |
| Agent → 用户 | `OutboundPayload` | UI/Channel 可见事件 |

### AgentEvent 类型

- **生命周期**: RunStarted, SessionResolved, RunCompleted, RunCancelled, RunFailed
- **执行**: ContextBuilt, ProviderTextDelta, ProviderToolCall
- **工具**: ToolStarted, ToolFinished
- **干预**: CommandIntercepted, SteeringInjected

### UserVisiblePhase

- `Thinking` — 组装上下文 + 等待首 token
- `UsingTools` — 执行工具中
- `Streaming` — 输出文本

---

## 关键设计决策

| 决策 | 选择 | 理由 |
|------|------|------|
| 每轮定义 | 一次完整 `chat_stream` 调用 | 返回多个 tool calls 按轮计更可预测 |
| 工具执行时机 | 收完 stream 后批量执行 | 避免 stream 中途暂停的复杂状态 |
| 工具结果回传 | 追加到 messages 发起新请求 | OpenAI/Claude 无状态 API 设计 |
| 工具并行 | Semaphore(4) 限制并发 | 独立 tool calls 可并行，防资源爆炸 |
| 取消检查 | 每轮开始 + HTTP abort | 粗粒度 + 细粒度结合 |
| 运行中注入 | Steering 软打断 | 保留已完成工具轮次 |

---

## 子章节导航

| 文件 | 内容 |
|------|------|
| **本文件** | AgentLoop 核心 · MessageBus · 消息流水线 · Session |
| [03.01-context.md](./03.01-context.md) | Context Pipeline |
| [03.02-provider.md](./03.02-provider.md) | Provider 层 |
| [03.03-tools.md](./03.03-tools.md) | Tool 层（MCP、Hooks、Skills） |
| [03.04-memory.md](./03.04-memory.md) | Memory 层 |
| [03.05-services.md](./03.05-services.md) | Services 层 |
| [03.06-subagent.md](./03.06-subagent.md) | SubAgent |
