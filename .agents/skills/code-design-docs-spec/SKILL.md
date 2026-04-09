---
name: code-design-docs-spec
description: 软件设计文档规范集合，包含架构设计文档(arch-spec)、功能需求规格(prd-spec)和测试规格(test-spec)的完整规范。当需要编写或维护设计文档时，根据文档类型引用对应规范，按规范中的 Agent 执行工作流操作。
---

# 设计文档规范

本技能提供三套规范，覆盖软件设计的主要文档类型：

1. **架构设计文档** (`arch-spec.md`) — 系统架构、模块边界、接口契约、技术决策
2. **功能需求规格** (`prd-spec.md`) — 用户可见功能、验收标准、范围界定、非功能需求
3. **测试规格** (`test-spec.md`) — 测试策略、测试边界、测试用例

---

## 使用流程

**步骤 1：判断文档类型**

```
变更涉及...
├── 架构/接口/技术决策 → arch-spec
├── 用户可见功能/需求范围 → prd-spec
└── 测试策略/测试边界/测试用例 → test-spec
```

**步骤 2：读取对应规范文件**

- 架构设计 → `references/arch-spec.md`
- 功能需求 → `references/prd-spec.md`
- 测试规格 → `references/test-spec.md`

**步骤 3：按规范中的 Agent 执行工作流操作**

每套规范均包含：触发条件判断 → 文档结构 → 写作规则 → 自检清单。

---

## 共同约定

**规则强度标记**（三套规范统一）：

- **REQUIRED** — 必须遵守
- **PROHIBITED** — 明确禁止
- **OPTIONAL** — 视情况使用

**文档状态标注**（所有设计文档统一格式）：

```markdown
> **Status**: `draft` | `active`
```

废弃文档移入 `deprecated/` 文件夹，文件头部添加 `> **Superseded by**: [链接]`。

状态转换规则：

- `draft → active`：对应规范的自检清单全部通过后标记
- 废弃文档：有替代文档则移入 `deprecated/` 文件夹；无替代文档则直接删除

**目录结构**：

```
docs/design/
├── arch/
│   ├── 00-overview.md
│   ├── NN-module.md
│   ├── NN.MM-sub-module.md
│   ├── deprecated/
│   └── reference/
├── prd/
│   ├── 00-overview.md
│   ├── NN-feature.md
│   ├── NN.MM-sub-feature.md
│   ├── deprecated/
│   └── reference/
└── test/
    ├── 00-overview.md
    ├── 10-plan-system.md
    ├── 20-cases-module.md
    ├── deprecated/
    └── reference/
```

**PROHIBITED** 以下做法：

- 跳过触发条件判断（为不需要文档的变更编写文档）
- 忽略自检清单（未检查直接交付）
- 使用模糊语言（"可能"、"大概"、"视情况而定"）
- 复制其他文档内容（应使用链接引用）
