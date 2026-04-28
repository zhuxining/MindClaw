> **Status**: `active`
>
> 本文档描述 MindClaw 系统的全局架构设计。涉及 ≥2 个模块的代码变更时，需检查本文档是否仍然准确。

# MindClaw 系统架构总览

MindClaw 的核心命题是”记忆是 Agent 的，知识是共同的”。系统围绕本地 AI 知识管理构建，支持对话、笔记两大核心场景，Agent 行为模式通过 Profile 配置可自定义。数据尽量留在本地、边界清晰、运行时可控。

---

## § 系统目标与约束

**系统定位**：MindClaw 为个人用户提供跨通道的 AI 助手，帮助管理知识笔记、任务与长期记忆。

**核心约束**：

1. 所有业务逻辑在 Rust 侧执行，前端保持薄客户端。
2. API Key 等敏感信息存储在 OS Keychain，不以明文落盘。
3. Agent 文件写操作仅限受控工作区，路径沙箱强制生效。
4. 同一 `session_key` 的消息串行处理。
5. `AgentRunner` 不持有 Session、MessageBus 或持久化依赖。
6. `AgentProfile` 是静态定义，不是运行时实体。

---

## § 核心设计原则

**1. Definition 与 Runtime 分离**
`AgentProfile` 只描述身份、策略和边界；Session、上下文、取消状态和消息分发都属于 Runtime。

**2. 外层编排与内层执行分离**
`AgentLoop` 负责编排一条入站消息，`AgentRunner` 负责执行一次 run 的迭代循环。

**3. Provider 仅作为 Adapter**
LLM Provider 只处理模型协议差异，不参与 Agent 路由、上下文构建或工具执行。

**4. 概念层与实现层分离**
`SubAgent` 与 `BackgroundAgent` 在概念层保持区分；实现层共用同一个 `AgentRunner` 与 spawn 机制。

**5. Markdown 为知识真相源**
知识笔记保存在 vault 中，数据库只维护索引和辅助状态。

---

## § 关键设计决策

| 决策问题 | 选择 | 放弃的替代方案 | 理由 |
|---------|------|--------------|------|
| Agent 是否是运行时实体？ | 否，使用 `AgentProfile` | Agent 持有 provider/session/tools | 定义与状态分离更利于复用和权限控制 |
| 编排与执行是否分层？ | 是，Loop / Runner 分离 | 单个大 Agent 对象包办全部 | turn 级和 run 级粒度不同，混在一起会导致职责失控 |
| SubAgent 与 BackgroundAgent 是否统一成同一个概念？ | 否，概念层保留区分 | 统一叫 child agent | 保留发起语义和等待语义，避免把后台任务也误建模成父子关系 |
| 派生执行是否单独一套 runtime？ | 否，统一 spawn 机制 | 各做一套执行链路 | 统一调度与观测语义，减少重复逻辑 |
| Provider 在哪里选型？ | `ModelRouter + ProviderRegistry` | AgentLoop 直接硬编码 provider | Provider 选择应是声明式路由，不应散落在业务层 |
| 工具权限在哪里决定？ | `AgentProfile.ToolPolicy` + Loop 过滤 | Tool 自己决定可不可用 | 权限统一收敛在定义层与编排层 |

---

## § 全局分层

```text
┌─────────────────────────────────────────────────────┐
│ Channel 层                                           │
│ Desktop · Telegram · 飞书                           │
└───────────────────┬─────────────────────────────────┘
                    ↕ Inbound / Outbound
┌───────────────────┴─────────────────────────────────┐
│ MessageBus                                           │
└───────────────────┬─────────────────────────────────┘
                    ↕
┌───────────────────┴─────────────────────────────────┐
│ Agent Runtime                                        │
│ Definition: AgentProfile / AgentRegistry / ModelRouter│
│ Orchestration: AgentLoop / Session / Context         │
│ Execution: AgentRunner / ToolExecutor / RunHooks     │
│ Adapter: Provider / MCP / Event / Storage adapters   │
└────────┬──────────────────────────┬─────────────────┘
         ↕                          ↕
┌────────┴────────┐       ┌─────────┴──────────────────┐
│ Services        │       │ Storage                     │
│ Task / Note ... │       │ SQLite / vault / Keychain   │
└─────────────────┘       └─────────────────────────────┘
```

---

## § Runtime 主调用链

```mermaid
sequenceDiagram
    participant User as User
    participant Channel as Channel
    participant Bus as MessageBus
    participant Loop as AgentLoop
    participant Profile as AgentProfile
    participant Runner as AgentRunner
    participant Provider as ProviderAdapter

    User->>Channel: 输入
    Channel->>Bus: InboundMessage
    Bus->>Loop: consume
    Loop->>Profile: resolve
    Loop->>Loop: build context + run spec
    Loop->>Runner: run(spec, hooks)
    Runner->>Provider: chat / chat_stream
    Provider-->>Runner: response
    Runner-->>Loop: result
    Loop->>Bus: OutboundMessage
    Bus->>Channel: deliver
    Channel->>User: 输出
```

---

## § Main / Sub / Background 统一模型

| 类型 | Profile.kind | InvocationMode | 结果去向 |
|------|--------------|----------------|---------|
| 主 Agent | `main` | `interactive` | 直接给用户 |
| 子代理 | `sub` | `inline_child` | 返回给父 Agent |
| 后台代理 | `background` | `detached` | 后续独立通知 |

统一点：

- 共用 `AgentRunner`
- 共用 ToolExecutor
- 共用 Provider Adapter

差异点：

- Profile 策略
- RunHooks
- 可见性与等待语义

---

## § 跨切关注点

| 关注点 | 实现策略 | 说明 |
|--------|---------|------|
| 认证 | Provider Adapter 统一处理 | API Key 只在 Adapter 层读取和持有 |
| 并发控制 | Session Lock + Global Gate | Session 级串行、全局级限流 |
| 指标 | Runtime Events | Loop / Runner / Tool / Child Dispatch 统一产生日志与指标 |
| 错误翻译 | 模块边界显式转换 | Provider、Tool、Storage 错误分别在边界处翻译 |
| 权限控制 | Profile + PathGuard + Tool filtering | 权限决策集中，不下放给单个工具实现 |

---

## § 相关文档

### 设计文档

| 文件 | 内容 |
|------|------|
| [00-overview.md](./00-overview.md) | 系统架构总览 |
| [01-channels.md](./01-channels.md) | Channels：多通道架构 |
| [02-bus.md](./02-bus.md) | MessageBus：异步消息队列 |
| [03-agent-runtime.md](./03-agent-runtime.md) | Agent Runtime：Definition / Orchestration / Execution 总览 |
| [03.01-agent-profile.md](./03.01-agent-profile.md) | AgentProfile：静态定义与策略边界 |
| [03.02-agent-loop.md](./03.02-agent-loop.md) | AgentLoop：turn 级编排层 |
| [03.03-agent-runner.md](./03.03-agent-runner.md) | AgentRunner：run 级执行引擎 |
| [03.04-run-contracts.md](./03.04-run-contracts.md) | Run 契约：Spec / Result / RunHooks / InvocationMode |
| [03.05-agent-spawn.md](./03.05-agent-spawn.md) | Agent Spawn：SubAgent 与 BackgroundAgent |
| [03.06-context-pipeline.md](./03.06-context-pipeline.md) | ContextPipeline：上下文装配 |
| [03.07-tool-execution.md](./03.07-tool-execution.md) | Tool Execution：工具执行系统 |
| [03.08-mcp.md](./03.08-mcp.md) | MCP：外部能力适配 |
| [03.09-skills.md](./03.09-skills.md) | Skills：能力定义与按需注入 |
| [03.10-memory.md](./03.10-memory.md) | Memory：后台提取与升华 |
| [03.11-observability.md](./03.11-observability.md) | Observability：Runtime 可观测性 |
| [04-providers.md](./04-providers.md) | Providers：LLM Provider Adapter 层 |
| [05-services.md](./05-services.md) | Services：业务服务层 |
| [06-storage.md](./06-storage.md) | Storage：存储层设计 |
| [07-runtime.md](./07-runtime.md) | AppRuntime：统一运行时与依赖注入 |

### 参考文档

| 文件 | 内容 |
|------|------|
| [reference/directory-structure.md](./reference/directory-structure.md) | 代码目录结构现状 |
| [reference/dependencies.md](./reference/dependencies.md) | Rust 和前端依赖清单 |
| [reference/type-registry.md](./reference/type-registry.md) | 跨模块接口契约索引 |
| [reference/config.md](./reference/config.md) | 配置项清单与加载顺序 |
| [reference/database-notes.md](./reference/database-notes.md) | 数据库表结构与索引说明 |
