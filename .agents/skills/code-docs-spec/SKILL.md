---
name: code-docs-spec
description: 软件设计文档规范集合，包含产品蓝图(blueprint-spec)、架构设计文档(architecture-spec)、功能需求规格(prd-spec)、测试规格(test-spec)和文档审校与润色(docs-review-spec)的完整规范。当需要编写、维护、检查或润色设计文档时，根据文档类型引用对应规范，并在首次成稿后按 docs-review-spec 执行交付前质量门。
---

# 设计文档规范

本技能提供四套规范，覆盖软件设计的主要文档类型：

1. **产品蓝图** (`blueprint-spec.md`) — 产品存在理由、核心命题、价值边界、长期演进方向
2. **架构设计文档** (`architecture-spec.md`) — 系统架构、模块边界、接口契约、技术决策
3. **功能需求规格** (`prd-spec.md`) — 用户可见功能、验收标准、范围界定、非功能需求
4. **测试规格** (`test-spec.md`) — 测试策略、测试边界、测试用例

另提供一套通用交付前质量门：

- **文档审校与润色** (`docs-review-spec.md`) — 首次成稿后的合规检查、结构优化、语言润色与交付前确认

---

## 使用流程

**步骤 1：判断文档类型**

```
变更涉及...
├── 文档审校/润色/合规检查 → docs-review-spec + 对应文档规范
├── 产品定位/核心命题/长期方向 → blueprint-spec
├── 架构/接口/技术决策 → architecture-spec
├── 用户可见功能/需求范围 → prd-spec
└── 测试策略/测试边界/测试用例 → test-spec
```

**步骤 2：读取对应规范文件**

- 产品蓝图 → `references/blueprint-spec.md`
- 架构设计 → `references/architecture-spec.md`
- 功能需求 → `references/prd-spec.md`
- 测试规格 → `references/test-spec.md`
- 文档审校与润色 → `references/docs-review-spec.md`

**步骤 3：按规范中的 Agent 执行工作流操作**

每套规范均包含：触发条件判断 → 文档结构 → 写作规则 → 自检清单。

**步骤 4：首次成稿后执行交付前质量门**

- Agent 首次写完任一设计文档后，**REQUIRED** 读取 `references/docs-review-spec.md`
- 按 `docs-review-spec` 完成合规检查与润色优化
- 通过原文档类型自检清单和 `docs-review-spec` 后，才能交付或将 `draft` 标记为 `active`

---

## 共同约定

**规则强度标记**（四套规范统一）：

- **REQUIRED** — 必须遵守
- **PROHIBITED** — 明确禁止
- **OPTIONAL** — 视情况使用

**文档状态标注**（所有设计文档统一格式）：

```markdown
> **Status**: `draft` | `active`
```

废弃文档移入 `deprecated/` 文件夹，文件头部添加 `> **Superseded by**: [链接]`。

状态转换规则：

- `draft → active`：对应规范的自检清单与 `docs-review-spec` 全部通过后标记
- 废弃文档：有替代文档则移入 `deprecated/` 文件夹；无替代文档则直接删除

**目录结构**：

```
docs/
├── blueprint/
│   ├── 00-overview.md
│   ├── NN-product-area.md
│   ├── deprecated/
│   └── reference/
├── architecture/
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
