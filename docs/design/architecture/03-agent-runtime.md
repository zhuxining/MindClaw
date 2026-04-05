> **Status**: `active`

# Agent Runtime

MindClaw 的 Agent Runtime 采用四段式边界：

- Definition：定义 Agent 是谁、有什么边界
- Orchestration：定义一次消息如何被编排
- Execution：定义一次 run 如何完成 LLM 与工具迭代
- Adapter：定义如何连接 Provider、Storage、Bus、MCP 等外部系统

---

## § 职责定位

第 3 章描述 Agent Runtime 本身，不描述具体 Provider API、业务 Service 或存储实现细节。

---

## § 核心原则

**Definition 与 Runtime 分离**：`AgentProfile` 是静态定义，不持有 Session、Bus、Provider 或任何运行时状态。

**外层编排与内层执行分离**：`AgentLoop` 负责 turn 级编排，`AgentRunner` 负责 run 级迭代循环。

**Provider 只做适配**：Provider Adapter 只处理模型协议差异，不参与上下文构建、工具执行或 Session 管理。

**概念与实现分离**：`SubAgent` 与 `BackgroundAgent` 在概念层保持区分，但实现层共用 `AgentRunner` 和 spawn 机制。

**单文件优先**：当前 Rust Runtime 代码除 `tools/` 外，优先用单文件承接边界，不预先拆成多层目录。

**类型就近放置**：Rust 类型默认与其主职责模块同文件放置；只有在文件明显膨胀或纯数据类型被高频复用时，才抽出独立 `types.rs`。

**边界先于颗粒度**：先把 `AgentProfile`、`AgentLoop`、`AgentRunner`、`RunHooks`、`AgentSpawnDispatcher` 的职责拉直，再决定是否继续拆文件。

---

## § 运行时分层

```text
┌──────────────────────────────────────────────────────┐
│ Definition Layer                                     │
│ AgentProfile · AgentRegistry · ModelRouter           │
├──────────────────────────────────────────────────────┤
│ Orchestration Layer                                  │
│ AgentLoop · SessionManager · ContextPipeline         │
│ AgentResolver · AgentSpawnDispatcher                 │
├──────────────────────────────────────────────────────┤
│ Execution Layer                                      │
│ AgentRunner · ToolExecutor · RunHooks                │
│ AgentRunSpec · AgentRunResult                        │
├──────────────────────────────────────────────────────┤
│ Adapter Layer                                        │
│ ProviderRegistry · LLMProviderClient · EventPublisher│
│ StorageAdapter · MCP / Tool adapters                 │
└──────────────────────────────────────────────────────┘
```

---

## § 核心对象

**AgentProfile**：静态定义对象。
关键属性：提示词策略、模型策略、工具策略、上下文策略、安全边界、委托规则。
关系：由 AgentRegistry 持有，AgentLoop 每次处理消息时读取。

**AgentLoop**：turn 级编排器。
关键属性：SessionManager、ContextPipeline、AgentRegistry、ModelRouter、AgentRunner、EventPublisher。
关系：从 MessageBus 收到消息后生成一次 `AgentRunSpec`，调用 Runner 执行，并负责持久化与流式输出。

**AgentRunner**：run 级执行引擎。
关键属性：ProviderRegistry、ToolExecutor。
关系：消费 `AgentRunSpec`，输出 `AgentRunResult`；不感知 Session、Bus、Channel。

**AgentSpawnDispatcher**：派生执行编排器。
关键属性：spawn source 解析、权限校验、父子 run 链路、后台任务调度。
关系：由 AgentLoop、Scheduler 或其他系统入口调用；支持同步 `SubAgent` 与异步 `BackgroundAgent`。

---

## § 统一调用链

```mermaid
sequenceDiagram
    participant Bus as MessageBus
    participant Loop as AgentLoop
    participant Profile as AgentProfile
    participant Ctx as ContextPipeline
    participant Runner as AgentRunner
    participant Provider as ProviderRegistry

    Bus->>Loop: InboundMessage
    Loop->>Profile: resolve(agent_id / mode)
    Loop->>Ctx: build_context(session, inbound, profile)
    Loop->>Provider: resolve model/provider
    Loop->>Runner: run(spec, hooks)
    Runner->>Provider: chat / chat_stream
    Provider-->>Runner: response / stream
    Runner-->>Loop: AgentRunResult
    Loop->>Bus: publish chunks / done
```

---

## § Main / Sub / Background

| 形态 | AgentProfile.kind | InvocationMode | 谁等待结果 | 用户是否直接可见 |
|------|-------------------|----------------|-----------|-----------------|
| 主对话 Agent | `main` | `interactive` | 用户 | 是 |
| 子代理 | `sub` | `inline_child` | 父 Agent | 默认否 |
| 后台 Agent | `background` | `detached` | 无人等待；若来自主 Agent 则同步返回 task id | 完成时单独通知 |

三者不拥有三套 Runtime。它们共用：

- 同一个 `AgentRunner`
- 同一套 ToolExecutor
- 同一套 Provider Adapter

---

## § 第 3 章文档地图

| 文件 | 内容 |
|------|------|
| [03.01-agent-profile.md](./03.01-agent-profile.md) | AgentProfile：静态定义与策略边界 |
| [03.02-agent-loop.md](./03.02-agent-loop.md) | AgentLoop：turn 级编排 |
| [03.03-agent-runner.md](./03.03-agent-runner.md) | AgentRunner：run 级迭代执行 |
| [03.04-run-contracts.md](./03.04-run-contracts.md) | Run 契约：Spec / Result / RunHooks / InvocationMode |
| [03.05-agent-spawn.md](./03.05-agent-spawn.md) | Agent Spawn：SubAgent 与 BackgroundAgent |
| [03.06-context-pipeline.md](./03.06-context-pipeline.md) | ContextPipeline：上下文装配 |
| [03.07-tool-execution.md](./03.07-tool-execution.md) | Tool Execution：工具注册、沙箱与执行 |
| [03.08-mcp.md](./03.08-mcp.md) | MCP：外部能力适配 |
| [03.09-skills.md](./03.09-skills.md) | Skills：能力定义与按需注入 |
| [03.10-memory.md](./03.10-memory.md) | Memory：后台提取与升华 |
| [03.11-observability.md](./03.11-observability.md) | Observability：运行时观测 |

---

## § 设计决策与权衡

| 决策问题 | 选择 | 放弃的替代方案 | 理由 |
|---------|------|--------------|------|
| Agent 是否是运行时实体？ | 否，`AgentProfile` 是静态定义 | Agent 持有 provider/session/tools | 定义与状态分离后，main/sub/background 共享执行引擎，边界稳定 |
| 编排与执行是否分离？ | 是，Loop 与 Runner 分层 | AgentLoop 内嵌全部迭代逻辑 | turn 编排和 run 迭代是两个粒度，混在一起会导致对象过胖 |
| SubAgent 与 BackgroundAgent 是否合并为单一概念？ | 否，概念层保留区分 | 统一叫 child agent | 保留谁发起、是否等待等语义差异，同时实现层仍可复用 |
| 派生执行是否共用同一套实现？ | 是，统一走 AgentSpawnDispatcher + AgentRunner | 子代理、后台代理各做一套执行流程 | 执行链路与控制点高度重叠，复用更稳 |
| Provider 在哪一层？ | Adapter Layer | 放进 AgentLoop 或 AgentProfile | Provider 只处理协议差异，不应携带业务语义 |
