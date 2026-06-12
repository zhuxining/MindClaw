# PRD 文档规范

> 本目录存放产品需求文档。PRD 回答“要交付什么、为什么、怎样算完成”；架构文档回答“系统如何实现”。

## 文档类型

| 类型 | 用途 | 命名 |
|------|------|------|
| Index | PRD 目录和当前需求入口 | `00-overview.md` |
| Product PRD | 跨版本产品能力需求 | `10-*.md` |
| MVP / Feature PRD | 当前阶段可验收功能需求 | `20-*.md`、`30-*.md` |

## 标准结构

每份 PRD 应尽量使用以下结构：

```md
> **Status**: `draft | active | superseded`
> **Blueprint**: `docs/blueprint/00-overview.md`

# PRD: [Feature / Product Name]

## 1. Objective

### Problem
[用户遇到什么问题。]

### Target Users
[具体用户是谁。]

### Desired Outcome
[完成后用户能做到什么。]

## 2. Success Criteria

- [ ] [可验证成功条件]

## 3. Scope

### In Scope
- [范围内事项]

### Out of Scope
- [范围外事项与原因]

## 4. User Stories

### Story 1: [Name]

**As a** [user], **I want** [capability], **so that** [outcome].

**Priority**: P0 | P1 | P2

**Acceptance Criteria**:

- [ ] [可验证验收条件]

## 5. Non-functional Requirements

- Security: [安全要求]
- Reliability: [可靠性要求]
- Performance: [性能要求]
- Observability: [可观测性要求]

## 6. Open Questions

- [ ] [未决问题]

## 7. Related Docs

- [相关文档]
```

## 编写规则

### PRD 应该包含

- 用户问题
- 目标用户
- 用户结果
- 成功标准
- In Scope / Out of Scope
- 用户故事和验收标准
- 产品级非功能需求
- 未决问题

### PRD 不应该包含

- 代码目录结构
- 模块内部设计
- 具体实现状态
- 迁移任务
- 工程命令清单
- Service / Command / Storage 详细约束

这些内容分别归属：

| 内容 | 应放位置 |
|------|----------|
| 产品定位、核心价值、长期演进 | `docs/blueprint/` |
| 用户故事、验收标准、产品范围 | `docs/prd/` |
| 模块职责、接口边界、依赖规则 | `docs/architecture/` |
| 当前实现状态、迁移路线 | `docs/architecture/reference/migration.md` |
| 目录结构 | `docs/architecture/reference/directory-structure.md` |
| 文档与模块追踪关系 | `docs/architecture/reference/traceability.md` |

## Success Criteria 写法

成功标准必须可验证。避免：

- “体验更好”
- “更稳定”
- “支持更多场景”

推荐：

- “首次内测用户能在 10 分钟内完成配置”
- “同一 `channel + conversation_id` 内消息按进入顺序处理”
- “默认配置下不自动发送真实 IM 回复”

## User Story 写法

推荐：

```md
**As a** 开发者用户，**I want** 注册一个本地 ACP Server，**so that** MindClaw 能复用我已有的 Agent 执行后端。
```

不推荐：

```md
实现 AcpServerRegistry CRUD。
```

## 维护规则

- 产品定位变化：先更新蓝图，再更新 PRD。
- 需求范围变化：先更新 PRD，再拆任务或实现。
- 架构边界变化：更新 architecture 文档，不塞进 PRD。
- 实现状态变化：只更新 migration 文档。
- 新增或删除 PRD 后，同步更新 `00-overview.md` 和 `docs/architecture/reference/traceability.md`。
