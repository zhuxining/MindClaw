> **Status**: `active`
> **Blueprint**: `docs/blueprint/00-overview.md`

# PRD Overview: MindClaw ACP-native Agent Control Plane

## 1. Objective

### Problem

MindClaw 的产品方向已经从“宽口径消息网关”收敛为“本地优先的 ACP-native Agent 控制平面”。旧 PRD 混合了产品需求、架构设计、实现状态和多渠道长期规划，导致 MVP 范围过大、验收口径不清晰。

### Target Users

- 已经在使用 Claude Code、Gemini CLI、自研 Agent 或其他 ACP Server 的开发者。
- 希望把自己的 Agent 接入 Feishu 等 IM 消息流的重度 LLM 用户。
- 需要在本地管理 Agent 角色、Skill、会话状态和渠道连接的高级用户。

### Desired Outcome

PRD 体系成为后续实现的需求入口：只描述用户目标、范围、验收标准和产品级非功能需求；架构设计、实现状态和迁移计划分别由 architecture 与 migration 文档承载。

## 2. Success Criteria

- [ ] 所有 PRD 遵循 `docs/prd/README.md` 中定义的轻量结构。
- [ ] 当前 MVP 需求以 `docs/prd/20-acp-native-feishu-agent-mvp.md` 为准。
- [ ] Agent / Skill / SlashCommand 的产品能力边界以 `docs/prd/10-agent-skill-slash-command.md` 为准。
- [ ] 旧的多渠道宽口径 Gateway PRD 不再作为当前实现依据。
- [ ] 实现状态只在 `docs/architecture/reference/migration.md` 维护。
- [ ] 场景追踪只在 `docs/architecture/reference/traceability.md` 维护。

## 3. Scope

### In Scope

- 定义 PRD 文档体系和优先级。
- 明确当前 MVP 的需求入口。
- 明确 Agent / Skill / SlashCommand 控制平面的产品边界。
- 保留与蓝图、架构和迁移文档的引用关系。

### Out of Scope

- 不在 overview 中写具体用户故事验收标准；具体 Story 写在对应 PRD。
- 不在 overview 中维护实现状态；实现状态写在 migration 文档。
- 不在 overview 中维护模块目录；目录结构写在 architecture reference。
- 不在 overview 中描述旧宽口径 GatewaySupervisor PRD。

## 4. PRD Index

| 文档 | 状态 | 用途 | 优先级 |
|------|------|------|--------|
| `docs/prd/README.md` | active | PRD 编写规范 | P0 |
| `docs/prd/00-overview.md` | active | PRD 目录和文档关系 | P0 |
| `docs/prd/10-agent-skill-slash-command.md` | active | Agent / Skill / SlashCommand 控制平面需求 | P0 |
| `docs/prd/20-acp-native-feishu-agent-mvp.md` | active | 当前 MVP 需求 | P0 |

## 5. Current Product Slice

当前实现和任务拆解应优先围绕：

> 一个默认 ACP Server + 一个默认 Agent + 一个默认 Skill + 一个 Feishu 渠道 + 建议回复 / 受限回写。

这不是最终产品边界，而是用最小闭环验证蓝图中的核心价值链：

> 用户是否愿意把自己的 Agent 角色和 Skill 配在 MindClaw 里，通过 ACP Server 执行，并让它进入一个真实 IM 消息流。

## 6. Non-functional Requirements

- **Clarity**：PRD 只承载需求，不混入实现状态和架构迁移计划。
- **Traceability**：每个核心 PRD 必须能在 `docs/architecture/reference/traceability.md` 中找到对应关系。
- **Maintainability**：新增或删除 PRD 后，需要同步更新本 overview。

## 7. Open Questions

- [ ] 是否需要为每个 v1.x 版本单独建立 PRD，还是先只维护 MVP 和核心控制平面 PRD？
- [ ] 是否需要将旧 PRD 历史移动到 `docs/prd/archive/`，还是直接以 git 历史保留？

## 8. Related Docs

- `docs/prd/README.md`
- `docs/blueprint/00-overview.md`
- `docs/prd/10-agent-skill-slash-command.md`
- `docs/prd/20-acp-native-feishu-agent-mvp.md`
- `docs/architecture/reference/migration.md`
- `docs/architecture/reference/traceability.md`
