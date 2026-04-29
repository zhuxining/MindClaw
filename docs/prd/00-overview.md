> **Status**: `draft`

# MindClaw 桌面端 PRD 总览

MindClaw 桌面端是一个**人机知识共建与 Agent 演化工作站**。用户在同一个 Markdown 知识空间中记录、整理、审阅、协作；Agent 围绕这些内容执行任务、形成记忆、留下演化记录，并把稳定经验沉淀为可共享知识。

本 PRD 集合只描述用户可见功能、交互行为、验收标准和范围边界。产品定位与长期方向见 [产品蓝图](../blueprint.md)，技术分层见 [架构总览](../architecture/00-overview.md)。

## 产品原则

- Markdown + Frontmatter 是知识内容、Inbox 待处理产物与 Agent 演化资产的真相源；索引层只作为可重建查询和运行时加速层。
- Frontmatter 的 `tags` 与 `overview` 是知识分层加载入口，服务人类检索与 Agent 预读。
- 共有知识可以按主题、项目或用户规则组织；Inbox 处理结果优先进入合适位置，Archive 只作为无明确去向时的兜底。
- Agent 记忆、旁路观察、记忆更新建议、演化记录和经验教训候选都必须可查看、可纠偏、可迁移。
- 旁路观察、轻量回顾、外部解析结果和经验教训候选必须先进入 Inbox 待处理流程，不能绕过审核直接写入共有知识。
- Private 内容不进入 Agent 上下文、不参与记忆、不参与共有知识索引。
- 任务以 Markdown checklist 表达，不作为独立一等业务对象。

## MVP Ribbon 入口

第一版 Ribbon（工作域活动栏）只展示以下入口：

| 入口 | 功能定位 |
|------|----------|
| Daily | 每日记录、当日回顾、轻量 checklist |
| Inbox | 捕获、外部解析结果、审核候选与知识草稿的待处理集散地 |
| Vault | 共享知识库浏览、编辑、搜索、Frontmatter 索引 |
| Private | 私密内容编辑与隔离边界 |
| Agent | Agent Session、上下文引用、执行反馈、草稿生成 |
| Memory | Agent 记忆查看、确认、修正、删除、知识引用 |
| Settings | 工作区设置、Vault 路径、隐私边界、基础偏好 |

Graph、Skills、MCP、Cron、Tasks 不作为 MVP Ribbon 主入口。Checklist 任务能力由 Daily、Inbox、Vault 与 Agent Session 承载。

## 文档导航

| 文档 | Feature | 状态 |
|------|---------|------|
| [01-workspace-shell.md](01-workspace-shell.md) | 工作台壳层与全局交互 | `draft` |
| [02-agent-session.md](02-agent-session.md) | Agent Session 工作域 | `draft` |
| [03-daily-inbox.md](03-daily-inbox.md) | Daily 与 Inbox 待处理闭环 | `draft` |
| [04-vault-knowledge.md](04-vault-knowledge.md) | Vault 共享知识空间 | `draft` |
| [05-private-boundary.md](05-private-boundary.md) | Private 私密边界 | `draft` |
| [06-agent-memory.md](06-agent-memory.md) | Agent 记忆管理 | `draft` |
| [07-reflection-evolution.md](07-reflection-evolution.md) | 反思回顾、演化路径与经验教训候选 | `draft` |
| [08-checklist-tasks.md](08-checklist-tasks.md) | Markdown checklist 任务 | `draft` |

## 范围界定

**In Scope**：

- 桌面端工作台基础结构与 MVP Ribbon 入口。
- Markdown 笔记、Daily、Inbox、Vault、Private、Agent Session、Agent Memory 的用户可见行为。
- Agent 旁路观察、轻量回顾、Inbox 审核候选、演化记录、经验教训候选和知识沉淀的 Markdown 审阅流程。
- Markdown checklist 的轻量任务能力。

**Out of Scope**：

- 移动端应用：当前 PRD 聚焦桌面工作站，移动端输入通道另行定义。
- 云同步与多设备协作：本阶段以本地 Vault 为默认工作空间，云端能力需单独定义同步、冲突和权限策略。
- 多 Vault：单一 Vault 已覆盖 MVP 的知识空间闭环，多 Vault 会引入独立的切换和索引边界。
- Graph、Skills、MCP、Cron 主入口：这些能力需要独立功能定义，纳入 MVP 会稀释第一版工作台主线。
- 业务 KPI：本 PRD 只定义可交付功能和验收标准。
