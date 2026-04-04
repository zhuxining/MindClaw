> **Status**: `active`
>
> 本文档作为架构文档的导航入口，仅列出 `active` 状态的文档。

# MindClaw 架构设计文档

## 入口

从 [00-overview.md](./00-overview.md) 开始阅读，了解系统全貌。

---

## 文档索引

### 设计文档

设计文档回答"为什么这样设计"，在设计决策变更时更新。

#### 总览

| 文件 | 内容 |
|------|------|
| [00-overview.md](./00-overview.md) | 系统目标、约束、核心原则、边界划分、实体关系、整体流程 |

#### 通道与入口

| 文件 | 内容 |
|------|------|
| [01-channels.md](./01-channels.md) | Channels：多通道架构（Desktop / Telegram / 飞书） |
| [02-bus.md](./02-bus.md) | MessageBus：Channel 与 AgentLoop 之间的异步消息队列 |

#### Agent 核心

| 文件 | 内容 |
|------|------|
| [03-agent-core.md](./03-agent-core.md) | 双层解耦架构概览（AgentLoop + AgentRunner + AgentHook） |
| [03.01-agent-loop.md](./03.01-agent-loop.md) | AgentLoop：业务编排层，消息消费、会话管理、上下文构建 |
| [03.02-agent-runner.md](./03.02-agent-runner.md) | AgentRunner：纯执行层，LLM 迭代循环，无状态可复用 |
| [03.03-agent-spec.md](./03.03-agent-spec.md) | AgentRunSpec / AgentRunResult 契约定义 |
| [03.04-agent-hook.md](./03.04-agent-hook.md) | AgentHook：生命周期钩子 |
| [03.05-agent-context.md](./03.05-agent-context.md) | Context Building：三层上下文组装 |
| [03.06-subagent.md](./03.06-subagent.md) | SubAgent：后台任务派生与结果回注 |
| [03.07-tools.md](./03.07-tools.md) | Tools：内置工具注册、执行调度、PathGuard 沙箱 |
| [03.08-mcp.md](./03.08-mcp.md) | MCP：外部工具协议集成（stdio / streamable-http） |
| [03.09-skills.md](./03.09-skills.md) | Skills：渐进式能力扩展（Agent Skills 规范） |
| [03.10-memory.md](./03.10-memory.md) | Memory：写入路径与升华机制 |
| [03.11-observability.md](./03.11-observability.md) | Observability：可观测性架构 |

#### 数据与业务层

| 文件 | 内容 |
|------|------|
| [04-providers.md](./04-providers.md) | Providers：LLM 服务商适配层（Claude / OpenAI 兼容） |
| [05-services.md](./05-services.md) | Services：Task / Knowledge / Daily 业务服务层 |
| [06-storage.md](./06-storage.md) | Storage：SQLite / Markdown vault / OS Keychain 职责划分 |

#### 运行时

| 文件 | 内容 |
|------|------|
| [07-runtime.md](./07-runtime.md) | AppRuntime：统一运行时与依赖注入 |

### 参考文档

参考文档回答"当前是什么样"，在代码/配置变更时同步更新。

| 文件 | 内容 |
|------|------|
| [reference/directory-structure.md](./reference/directory-structure.md) | 代码目录结构现状 |
| [reference/dependencies.md](./reference/dependencies.md) | Rust 和前端依赖清单 |
| [reference/type-registry.md](./reference/type-registry.md) | 跨模块接口契约（trait）索引 |
| [reference/config.md](./reference/config.md) | 配置项清单与加载顺序 |
| [reference/database-notes.md](./reference/database-notes.md) | 数据库表结构与索引说明 |

---

## 推荐阅读路径

**初次了解系统**：`00-overview.md` → `03-agent-core.md` → `03.01-agent-loop.md` → `03.02-agent-runner.md`

**理解数据流**：`01-channels.md` → `02-bus.md` → `03.01-agent-loop.md` → `06-storage.md`

**理解上下文组装**：`03.05-agent-context.md` → `03.07-tools.md` → `03.09-skills.md`

**添加新工具**：`03.07-tools.md` → `03.08-mcp.md`

**添加新通道**：`01-channels.md` → `02-bus.md`

---

## 文档规范

本文档遵循 [arch-spec.md](../../.claude/rules/arch-spec.md) 规范：

- **设计文档**（`*.md`）：记录"为什么这样设计"，在 `docs/design/architecture/` 根目录
- **参考文档**（`reference/*.md`）：记录"当前是什么样"，在 `reference/` 子目录
- 文档头部包含 `> **Status**: active | deprecated | draft` 标注
- 所有设计决策使用 ADR 格式记录（决策问题、选择、放弃的替代方案、理由）
