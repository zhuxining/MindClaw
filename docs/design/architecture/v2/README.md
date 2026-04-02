# MindClaw V2 Agent Core Architecture

`v2/` 是 MindClaw Agent Core 的目标架构文档。

- 它描述的是第二版目标模型，不等于当前实现。
- 它只覆盖 Agent 核心，不展开 Gateway、Channel、Storage 等外围章节。
- 它优先解决边界一致性问题，再讨论未来扩展。

## Reading Order

1. [00-overview.md](./00-overview.md)
2. [01-session.md](./01-session.md)
3. [02-agent-loop.md](./02-agent-loop.md)
4. [03-tools-services.md](./03-tools-services.md)
5. [04-memory-subagent-bus.md](./04-memory-subagent-bus.md)
6. [05-execution-runtime.md](./05-execution-runtime.md)

## Core Principles

- `MessageBus` 是核心基础设施，负责跨入口消息传递和异步结果回注。
- `AgentLoop` 是入口编排层，`ExecutionRuntime` 是复杂任务推进层，`Agent` 负责单步运行。
- `Agent` 不持有 run 级可变状态；run 级状态统一进入 `RunContext`。
- `SubAgent` 只负责后台异步任务，不承担摘要、记忆提炼或技能派生。
- `Memory` 只保留轻量、稳定、可解释的记忆接口。
- `Services` 是业务核心，不被 memory 或 background task 污染。

## Glossary

| Term | Meaning |
| --- | --- |
| `Session` | 用户连续对话的持久单元，持有历史、记忆引用和会话设置。 |
| `Execution` | 复杂任务的持久执行单元，持有目标、checkpoint、artifact 引用和状态。 |
| `Run` | Agent 对一条入站消息的完整处理过程。 |
| `Step` | `Execution` 内的一次单步推进，通常对应一次 LLM + tools 循环。 |
| `RunContext` | 单次 run 的全部输入快照，包含 session、消息、可用工具、记忆快照和取消令牌。 |
| `StepContext` | 单次 step 的输入快照，包含 execution snapshot、工具快照和取消令牌。 |
| `AgentLoop` | 入口编排器，负责消费消息、串行化 session、创建或恢复 execution。 |
| `ExecutionRuntime` | 复杂任务推进器，负责 checkpoint、artifacts 和下一步动作。 |
| `Agent` | 纯运行内核，负责上下文组装、调用 provider、执行单步工具循环并返回结果。 |
| `ToolCatalog` | 全局静态能力目录，描述系统已知工具及其元数据。 |
| `ActiveTools` | 单次 run 的工具快照，只包含当前 run 可调用的工具。 |
| `MessageBus` | 统一消息通道，承载入站消息、出站消息和后台任务回注。 |
| `BackgroundTask` | 由主 run 派发的后台异步任务，独立执行并在完成后回注所属 session。 |
| `MemoryStore` | 轻量记忆存储接口，提供 `store` 与 `recall`。 |
| `Services` | 业务逻辑层，向 Agent 暴露稳定能力，如 knowledge、daily、task。 |

## Boundaries

- `v2` 不包含迁移方案。
- `v2` 不要求兼容旧版 03 章编号和术语。
- `Skill` 不是 Agent Core 一等概念；如需保留，只能作为后续扩展附录。
- `ExecutionRuntime` 是新的核心层，但不承接旧版大一统 loop 的摘要、steering 或 hook 职责。
