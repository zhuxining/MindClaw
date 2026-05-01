> **Status**: `draft`

# MindClaw 桌面端 PRD 总览

MindClaw 桌面端是一个**人机知识共建与 Agent 演化工作站**。用户在同一个 Markdown 知识空间中记录、整理、审阅、协作；Agent 围绕这些内容执行任务、形成记忆、留下演化记录，并把稳定经验沉淀为可共享知识。

本 PRD 集合只描述用户可见功能、交互行为、验收标准和范围边界。产品定位与长期方向见 [产品蓝图](../blueprint/00-overview.md)，技术分层见 [架构总览](../architecture/00-overview.md)。

## 产品原则

- Markdown + Frontmatter 是知识内容、Inbox 待处理产物与 Agent 演化资产的真相源；索引层只作为可重建查询和运行时加速层。
- Frontmatter 的 `tags`、`overview` 与 `confidence` 是知识分层加载入口，服务人类检索、Agent 预读和召回重排。
- 共有知识可以按主题、项目或用户规则组织；Inbox 处理结果优先进入合适位置，Archive 只作为无明确去向时的兜底。
- Agent 记忆、旁路观察、记忆更新建议、演化记录和经验教训候选都必须可查看、可纠偏、可迁移。
- 旁路观察、轻量回顾、外部解析结果和经验教训候选必须先进入 Inbox 待处理流程，不能绕过审核直接写入共有知识。
- Private 是当前 Vault 下的 `private/` 文件夹，不是独立存储空间；其内容不进入 Agent 上下文、不参与记忆、不参与共有知识索引。
- 任务以 Markdown checklist 表达，不作为独立一等业务对象。

## Ribbon 入口

Ribbon（工作域活动栏）展示以下入口：

| 入口     | 功能定位                                             |
| -------- | ---------------------------------------------------- |
| Daily    | 每日记录、当日回顾、轻量 checklist                   |
| Inbox    | 捕获、外部解析结果、审核候选与知识草稿的待处理集散地 |
| Private  | 私密内容编辑与隔离边界                               |
| Vault    | 共享知识库浏览、编辑、搜索、Frontmatter 索引         |
| Checklist | 各 Markdown 文件中 checklist 的聚合、定位和状态视图 |
| Graph    | 知识、资源、记忆和引用关系图                         |
| Agent    | 自定义 Agent 角色、Agent Session 与执行反馈          |
| Skills   | Skills 列表、详情和启用状态                          |
| Memory   | Agent 记忆查看、确认、修正、删除、知识引用           |
| MCP      | MCP Server / Tool 列表、连接状态和配置               |
| Session  | Agent Session 列表、历史与详情                       |
| Cron     | Cron Job 列表、状态、运行记录和配置                  |
| Settings | 工作区设置、Vault 路径、隐私边界、基础偏好           |

Checklist 是各个 Markdown 文件中 checklist 的聚合内容视图，不拥有独立任务真相源。Daily、Inbox、Private、Vault 默认使用 File Explorer Pane；Agent、Skills、Memory、MCP、Session、Cron 默认使用列表 Pane 查询相关内容。

## 文档导航

| 文档                                                     | Feature                          | 状态    |
| -------------------------------------------------------- | -------------------------------- | ------- |
| [01-workspace-shell.md](01-workspace-shell.md)           | 工作台壳层与全局交互             | `draft` |
| [02-agent-session.md](02-agent-session.md)               | Agent Session 工作域             | `draft` |
| [03-daily-inbox.md](03-daily-inbox.md)                   | Daily 与 Inbox 待处理闭环        | `draft` |
| [04-vault.md](04-vault.md)                               | Vault 共享知识空间               | `draft` |
| [05-private-boundary.md](05-private-boundary.md)         | Private 私密边界                 | `draft` |
| [06-agent-memory.md](06-agent-memory.md)                 | Agent 记忆管理                   | `draft` |
| [07-reflection-evolution.md](07-reflection-evolution.md) | 反思回顾、演化路径与经验教训候选 | `draft` |
| [08-checklist-tasks.md](08-checklist-tasks.md)           | Markdown checklist 任务          | `draft` |

## 范围界定

**In Scope**：

- 桌面端工作台基础结构与 Ribbon 入口。
- Markdown 笔记、Daily、Inbox、Vault、Private、Agent Session、Agent Memory 的用户可见行为。
- Agent 旁路观察、轻量回顾、Inbox 审核候选、演化记录、经验教训候选和知识沉淀的 Markdown 审阅流程。
- Markdown checklist 的轻量任务能力。

**Out of Scope**：

- 移动端应用：当前 PRD 聚焦桌面工作站，移动端输入通道另行定义。
- 云同步与多设备协作：本阶段以本地 Vault 为默认工作空间，云端能力需单独定义同步、冲突和权限策略。
- 多 Vault：单一 Vault 已覆盖 MVP 的知识空间闭环，多 Vault 会引入独立的切换和索引边界。
- 插件化 Ribbon 与 Pane 扩展：当前只定义固定入口和固定 Pane 组合。
- 业务 KPI：本 PRD 只定义可交付功能和验收标准。
