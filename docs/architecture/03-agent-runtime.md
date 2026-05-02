> **Status**: `active`

# Agent Runtime

MindClaw 的 Agent Runtime 采用四段式边界：

- Definition：定义 Agent 是谁、有什么策略和边界
- Orchestration：定义一次消息如何被编排
- Execution：定义一次 run 如何完成 LLM 与工具迭代
- Adapter：定义如何连接 Provider、Storage、Bus、MCP 等外部系统

Review & Evolution 是 Runtime 的旁路产物处理链，不是主响应链的一部分。它复用 Runtime 执行能力生成候选，但候选生命周期由业务服务持有。

---

## § 职责定位

第 3 章描述 Agent Runtime 本身，以及 Runtime 如何触发记忆、回顾、演化和观测；不描述具体 Provider API、业务 Service 内部实现或存储表结构。

---

## § 核心原则

**Definition 与 Runtime 分离**：`AgentProfile` 是静态定义，不持有 Session、Bus、Provider 或任何运行时状态。

**外层编排与内层执行分离**：`AgentLoop` 负责 turn 级编排，`AgentRunner` 负责 run 级迭代循环。

**Provider 只做适配**：Provider Adapter 只处理模型协议差异，不参与上下文构建、工具执行或 Session 管理。

**Rig 是执行内核，不是业务 Agent**：[Rig](https://github.com/0xPlaygrounds/rig) 负责 run 内部的 model ↔ tools 多轮执行、streaming、tool result 回填和最终 history；`AgentProfile`、`AgentLoop`、`SessionManager`、`ContextPipeline`、`AgentSpawnDispatcher`、Review & Evolution 仍由 MindClaw 持有。

**旁路回顾不阻塞主链路**：观察、回顾和经验教训候选通过 side path 生成，不改变主对话响应时序。

**概念与实现分离**：`SubAgent` 和 Review 后台 run 在概念层保持区分；后台委派是 SubAgent 的 detached 运行方式，不是独立 Agent 身份。

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
│ AgentRunner · Rig Agent/PromptHook/ToolServer        │
│ AgentRunSpec · AgentRunResult                        │
├──────────────────────────────────────────────────────┤
│ Adapter Layer                                        │
│ ProviderRegistry · Rig Providers · EventPublisher    │
│ StorageAdapter · MCP config/lifecycle                │
└──────────────────────────────────────────────────────┘
```

---

## § 核心对象

**AgentProfile**：静态定义对象。
关键属性：提示词策略、模型策略、工具策略、上下文策略、安全边界、委托规则。

**AgentLoop**：turn 级编排器。
关键属性：SessionManager、ContextPipeline、AgentRegistry、ModelRouter、AgentRunner、EventPublisher。
关系：从 MessageBus 收到消息后生成一次 `AgentRunSpec`，调用 Runner 执行，并在 turn 完成后发出回顾触发信号。

**AgentRunner**：run 级执行引擎。
关键属性：`AgentModelSet`、Rig `AgentBuilder`、`StreamingPromptRequest`、`PromptHook`、`ToolSet/ToolServerHandle`。
关系：消费 `AgentRunSpec` 和本次工具范围，输出 `AgentRunResult`；不感知 Session、Bus、Channel、Memory、Review Queue。

**ContextPipeline**：上下文装配器。
关键属性：Profile 策略、Session 历史、Memory 召回、Vault 召回、Skill 注入。
关系：按预算构建本次 run 的上下文，不写入记忆或知识。

**AgentSpawnDispatcher**：派生执行编排器。
关键属性：spawn source 解析、权限校验、父子 run 链路、后台任务调度。
关系：支持 inline SubAgent、detached SubAgent 和 Review 后台 run。

**Runtime Events (`events.rs`)**：运行期共享事件契约。
关键对象：`UserVisiblePhase`。
关系：被 Runner hook、Loop、Bus 和 UI 共同引用；Provider streaming 与内部执行事件由 Rig/tracing 在 Runner 内部处理，不再作为 MindClaw 公开事件契约穿透。

---

## § 主调用链

```mermaid
sequenceDiagram
    participant Bus as MessageBus
    participant Loop as AgentLoop
    participant Profile as AgentProfile
    participant Ctx as ContextPipeline
    participant Runner as AgentRunner
    participant Rig as Rig Agent

    Bus->>Loop: InboundMessage
    Loop->>Profile: resolve(agent_id / mode)
    Loop->>Ctx: build_context(session, inbound, profile)
    Loop->>Runner: run(spec, tool_scope, observer, cancel)
    Runner->>Rig: AgentBuilder + StreamingPromptRequest
    Rig->>Rig: model stream via resolved AgentModelSet
    Rig->>Rig: tool calls via ToolServer
    Rig-->>Runner: final response + history
    Runner-->>Loop: AgentRunResult
    Runner-->>Bus: chunks/status via PromptHook observer
    Loop->>Bus: done
```

---

## § 回顾旁路

```mermaid
sequenceDiagram
    participant Loop as AgentLoop
    participant Review as Review & Evolution
    participant Spawn as AgentSpawnDispatcher
    participant Runner as AgentRunner
    participant User as User
    participant Memory as Memory
    participant Vault as Markdown Vault

    Loop->>Review: turn_completed signal
    Review->>Spawn: optional detached review run
    Spawn->>Runner: run(review_spec)
    Runner-->>Review: candidate output
    Review-->>User: ReviewItem
    User->>Review: confirm / reject
    Review->>Memory: apply confirmed proposal
    Review->>Memory: save confirmed lesson as memory
    Review->>Vault: optional save lesson as Vault note
```

旁路规则：

- 回顾产物默认是候选，不是稳定记忆或 Vault 笔记。
- 后台 run 复用 `AgentRunner`，但审核状态由 Review & Evolution 持有。
- Memory 只在建议确认后更新。
- Markdown Vault 只在用户保存知识草稿后写入。

---

## § Main / Sub / Background / Review

| 形态         | AgentProfile.kind | InvocationMode            | 谁等待结果             | 用户是否直接可见 |
| ------------ | ----------------- | ------------------------- | ---------------------- | ---------------- |
| 主对话 Agent | `main`            | `interactive`             | 用户                   | 是               |
| 子代理       | `sub`             | `inline_child`            | 父 Agent               | 默认否           |
| 后台 Agent   | `background`      | `detached`                | 无人等待；返回 task id | 完成时通知       |
| 回顾 Agent   | `background`      | `detached` 或同步轻量 run | Review & Evolution     | 候选进入回顾队列 |

四者共用：

- 同一个 `AgentRunner`
- 同一个 Rig execution kernel
- 同一套 `ToolSet/ToolServerHandle` 工具入口
- 同一个 `AgentModelSet` 主模型/轻量模型集合

差异点：

- Profile 策略
- RunHooks
- 可见性与等待语义
- 产物生命周期归属

---

## § 第 3 章文档地图

| 文件                                                     | 内容                                                |
| -------------------------------------------------------- | --------------------------------------------------- |
| [03.01-agent-profile.md](./03.01-agent-profile.md)       | AgentProfile：静态定义与策略边界                    |
| [03.02-agent-loop.md](./03.02-agent-loop.md)             | AgentLoop：turn 级编排                              |
| [03.03-agent-runner.md](./03.03-agent-runner.md)         | AgentRunner：run 级迭代执行                         |
| [03.04-run-contracts.md](./03.04-run-contracts.md)       | Run 契约：Spec / Result / RunHooks / InvocationMode |
| [03.05-agent-spawn.md](./03.05-agent-spawn.md)           | Agent Spawn：SubAgent 派生执行                      |
| [03.06-context-pipeline.md](./03.06-context-pipeline.md) | ContextPipeline：上下文装配                         |
| [03.07-tool-execution.md](./03.07-tool-execution.md)     | Tool Execution：工具注册、沙箱与执行                |
| [03.08-mcp.md](./03.08-mcp.md)                           | MCP：外部能力适配                                   |
| [03.09-skills.md](./03.09-skills.md)                     | Skills：能力定义与按需注入                          |
| [03.10-memory.md](./03.10-memory.md)                     | Memory：Agent 记忆与召回                            |
| [03.11-review-evolution.md](./03.11-review-evolution.md) | Review & Evolution：回顾、演化与经验教训候选        |
| [03.12-observability.md](./03.12-observability.md)       | Observability：Runtime 可观测性                     |

---

## § 设计决策与权衡

| 决策问题                                  | 选择                                                                                 | 放弃的替代方案                                 | 理由                                                                                         |
| ----------------------------------------- | ------------------------------------------------------------------------------------ | ---------------------------------------------- | -------------------------------------------------------------------------------------------- |
| Agent 是否是运行时实体？                  | 否，`AgentProfile` 是静态定义                                                        | Agent 持有 provider/session/tools              | 定义与状态分离后，main/sub/background 共享执行引擎                                           |
| 编排与执行是否分离？                      | 是，Loop 与 Runner 分层                                                              | AgentLoop 内嵌全部迭代逻辑                     | turn 编排和 run 迭代是两个粒度                                                               |
| 回顾是否使用第二套执行引擎？              | 否，复用 `AgentRunner`                                                               | 为记忆和经验提炼单独实现 runner                | 执行机制相同，差异在产物生命周期                                                             |
| Provider 在哪一层？                       | Adapter Layer                                                                        | 放进 AgentLoop 或 AgentProfile                 | Provider 只处理协议差异，不应携带业务语义                                                    |
| 是否用第三方 LLM 框架定义 Agent Runtime？ | 否，Rig 作为 Execution Kernel 被 `AgentRunner` 使用，Runtime 业务契约由 MindClaw 定义 | 直接采用 Rig Agent 作为 MindClaw 顶层 Agent 实体 | MindClaw 需要稳定的 Profile、Context、Spawn、Session 和 Review 语义，Rig 只接管 run 内部执行 |
| 运行观测是否承载演化语义？                | 否，`UserVisiblePhase` 只描述对外可见执行状态，结构化细节进入 tracing                | 把业务演化写进运行日志                         | 演化记录是长期审计资产，不能与临时观测事件混用                                               |
