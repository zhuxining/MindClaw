> **Status**: `active`

# Agent 核心架构

MindClaw Agent 核心采用双层解耦架构，将业务编排与 LLM 迭代执行严格分离。

---

## § 职责定位

Agent Core 负责定义业务编排层与执行层的边界契约，不负责具体的会话持久化、LLM 调用或工具实现。

---

## § 边界与实体

**输入**：来自 MessageBus 的 `InboundMessage`，携带用户输入和会话标识。
**输出**：发布到 MessageBus 的 `OutboundMessage`，携带 Agent 响应文本和工具执行轨迹。

**核心实体**：

**AgentRunSpec**：一次 Agent 执行的完整声明式配置，构建后不可修改。
关键属性：预组装的消息历史、可用工具集、LLM 模型标识、迭代上限、并行工具开关。
关系：由 AgentLoop 构建，传递给 AgentRunner 执行。

**AgentRunResult**：一次 Agent 执行的完整结构化结果。
关键属性：最终响应文本、完整消息链、使用的工具列表、停止原因、Token 用量。
关系：由 AgentRunner 返回，由 AgentLoop 处理持久化。

---

## § 双层架构边界

```
┌─────────────────────────────────────────────────────┐
│  AgentLoop（业务编排层）                              │
│  MessageBus I/O · SessionManager · ContextPipeline  │
│  MemoryStore · CommandRouter · 流式分发              │
├─────────────────────────────────────────────────────┤
│           AgentHook（六个生命周期扩展点）              │
│  wants_streaming · before_iteration · on_stream      │
│  on_stream_end · before_execute_tools · after_iteration │
│  finalize_content                                    │
├─────────────────────────────────────────────────────┤
│  AgentRunner（纯执行层）                              │
│  LLM 迭代循环 · ToolRegistry 执行 · 无状态            │
└─────────────────────────────────────────────────────┘
```

**边界规则**：

- AgentLoop 不直接调用 Provider 或 ToolRegistry，通过 AgentRunner 执行。
- AgentRunner 不感知 Session、MessageBus、Channel，只通过 AgentHook 与业务层通信。

---

## § 关键流程

```
MessageBus ──► InboundMessage
    │
    ▼
┌─────────────────────────────────────────────────────────────────────┐
│ [AgentLoop]                                                         │
│  Session Lock.acquire()                                             │
│       │                                                             │
│       ▼                                                             │
│  ContextPipeline.build() ──► AgentRunSpec                         │
│       │                                                             │
│       ▼                                                             │
│  LoopHook.new() ──► AgentRunner.run(spec, hook)                   │
│       │                                          │                  │
│       │                                          ▼                  │
│       │                              ┌───────────────────────────┐  │
│       │                              │ [AgentRunner]             │  │
│       │                              │  LLM.call()               │  │
│       │                              │       │                   │  │
│       │                              │       ▼                   │  │
│       │                              │  ToolRegistry.execute()   │  │
│       │                              │       │                   │  │
│       │                              │       ▼                   │  │
│       │                              │  [循环直到无工具调用]      │  │
│       │                              └───────────┬───────────────┘  │
│       │                                          │                  │
│       ▼                                          │                  │
│  LoopHook.on_stream(delta) ◄── stream ───────────┘                  │
│       │                                                             │
│       ▼                                                             │
│  MessageBus.publish_outbound()                                      │
│       │                                                             │
│       ▼                                                             │
│  SessionManager.append_turn() ──► Session Lock.release()            │
└─────────────────────────────────────────────────────────────────────┘
```

---

## § 与相关模块的关系

| 依赖方向 | 模块 | AgentLoop 使用什么 | AgentRunner 使用什么 |
|---------|------|-------------------|---------------------|
| 输入依赖 | MessageBus | `consume_inbound()` | — |
| 输入依赖 | SessionManager | 加载/写入会话历史 | — |
| 输入依赖 | ContextPipeline | 构建 AgentRunSpec.messages | — |
| 执行依赖 | AgentRunner | 调用 `run(spec, hook)` | — |
| 执行依赖 | Providers | — | 调用 `chat()` / `chat_stream()` |
| 执行依赖 | ToolRegistry | — | 调用 `execute_calls()` |
| 输出依赖 | MessageBus | `publish_outbound()` | — |

双层分离的直接体现：AgentLoop 的依赖列表中没有 Providers 和 ToolRegistry，AgentRunner 的依赖列表中没有 MessageBus 和 SessionManager。

---

## § 设计决策与权衡

| 决策问题 | 选择 | 放弃的替代方案 | 理由 |
|---------|------|--------------|------|
| LLM 迭代逻辑放在哪一层？ | 独立 AgentRunner | AgentLoop 内部嵌入循环 | Runner 无状态，子代理、CLI、Cron 可直接复用，不需要 MessageBus |
| 业务层如何向执行层注入行为？ | AgentHook trait（六个扩展点） | 回调闭包或事件总线 | trait 方法签名明确，编译期约束可注入的行为类型 |
| AgentRunSpec 是否可变？ | 构建后不可变 | 允许执行中修改 | 不可变保证执行可预测，便于复现和测试 |
| AgentHook 的实现数量？ | 三种（LoopHook / NoOpHook / TestHook） | 一种通用 Hook 含大量布尔配置 | 不同场景行为差异显著（流式/非流式），独立实现比条件分支更清晰，各实现职责单一 |
