> **Status**: `draft`
> **定位变更**: 任务不再是系统核心一等公民，而是可选功能 —— 通过 Markdown checklist 实现，Agent 驱动管理

# Markdown Checklist 任务管理

→ 架构关联：[08-desktop-frontend.md](../architecture/08-desktop-frontend.md)  
→ 父文档：[00-overview.md](00-overview.md)

---

## 背景与目标

**背景**：任务在 MindClaw 中**不再是独立数据结构**。任务以 Markdown checklist（`- [ ] 任务内容`）形式存在于 Daily Note 或任意笔记中。Agent 解析这些 checklist 并提供管理支持。

**核心转变**：
- 从「任务是一等公民」→「任务是 Markdown 内容，Agent 辅助管理」
- 从「独立数据模型」→「结构化 Markdown + 可选索引」
- 从「强制 Tasks 面板」→「可选视图，Agent 驱动呈现」

**目标**：定义 Agent 如何识别、追踪、提醒 Markdown 中的 checklist 任务，以及用户如何通过对话管理它们。

---

## 功能描述

### US-01 任务识别与索引

作为 Agent，我希望自动识别 Daily Note 中的 checklist 项，以便为用户提供任务管理服务。

**验收标准**：

Given 用户 Daily Note 中有 Markdown checklist  
When Agent 解析笔记内容  
Then 识别以下格式的任务：
- `- [ ] 待办任务`（未完成）
- `- [x] 已完成任务`（已完成）
- `- [ ] 高优先级任务 !high`（带优先级标签）
- `- [ ] 截止日任务 @2026-05-01`（带日期标签）

Given 任务被识别  
When Agent 索引到 SQLite  
Then 仅存储派生索引（path, line_number, content, status, tags），**真相始终在 Markdown**

**优先级**：P1

---

### US-02 通过对话创建任务

作为用户，我希望在 Chat 中说"帮我记个任务"，Agent 自动在 Daily Note 中创建 checklist 项。

**验收标准**：

Given 用户在 Chat 中发送"帮我记个任务：明天和产品开会"  
When Agent 识别意图  
Then 在当前 Daily Note 底部追加：
```markdown
- [ ] 明天和产品开会
```

Given 用户指定优先级或截止日  
When 说"记个高优先级任务，周五前完成报告"  
Then 创建：
```markdown
- [ ] 完成报告 !high @2026-05-02
```

- [ ] 任务创建后立即出现在 Daily Note 原文中
- [ ] Agent 回复确认，包含任务内容和位置

**优先级**：P1

---

### US-03 任务状态更新

作为用户，我希望在对话中更新任务状态，或直接在 Daily Note 中勾选。

**验收标准**：

Given 用户在 Chat 中说"产品会议任务完成了"  
When Agent 找到匹配任务  
Then 将该 checklist 项从 `- [ ]` 更新为 `- [x]`，并追加完成时间：
```markdown
- [x] 明天和产品开会 ✅ 2026-04-28
```

Given 用户在 Daily Note 中手动勾选任务  
When 文件保存  
Then Agent 在下次交互时感知状态变化（通过文件监听或索引更新）

**优先级**：P1

---

### US-04 任务提醒（Agent 驱动）

作为用户，我希望 Agent 主动提醒我即将到期或遗漏的任务。

**验收标准**：

Given 有一个带截止日 `@2026-04-30` 的未完成任务  
When 截止日临近（如提前 1 天）  
Then Agent 在对话中自然带出：
> "我注意到你有个任务'完成报告'截止明天，进展如何？"

Given 有未设置截止日的任务存在超过 3 天  
When Agent 评估当前对话上下文合适  
Then 温和询问："关于'调研竞品'这个任务，需要我帮你设置一个目标时间吗？"

- [ ] 提醒不是系统推送，而是 Agent 在对话中自然带出
- [ ] 用户可以配置提醒偏好（频率、时机）

**优先级**：P2

---

### US-05 可选 Tasks 面板（前端）

作为用户，我希望在右侧可选面板中看到从 Daily Note 提取的任务列表。

**验收标准**：

Given 用户打开 Tasks 面板  
When 面板加载  
Then 从 SQLite 索引查询并显示：
1. 今日相关（截止日为今日）
2. 高优先级（标记 !high）
3. 待办清单（其他未完成任务）

Given 用户在 Tasks 面板中点击任务  
When 点击操作触发  
Then 打开对应 Daily Note 并定位到 checklist 行

Given 用户在 Tasks 面板中勾选任务  
When 勾选操作触发  
Then 更新 Markdown 中的 `- [ ]` → `- [x]`，面板同步刷新

- [ ] Tasks 面板是**可选视图**，不是核心界面
- [ ] 面板关闭时系统正常运行

**优先级**：P2

---

## 范围界定

**In Scope**：

- Markdown checklist 解析与识别
- Agent 驱动的任务创建/更新/提醒
- 可选的 Tasks 面板（基于索引的视图）
- SQLite 派生索引（非真相源）

**Out of Scope**：

- 独立任务数据结构（已废弃）
- 复杂任务属性（子任务、标签系统、依赖关系）
- 日历集成（保持轻量，Agent 驱动提醒）
- 任务统计/甘特图等重度功能

## 技术实现要点

### Checklist 语法约定

```markdown
- [ ] 普通任务
- [ ] 优先级任务 !high !medium !low
- [ ] 截止日任务 @YYYY-MM-DD
- [ ] 综合任务 !high @2026-05-01
- [x] 已完成任务 ✅ 2026-04-28
```

### Agent 任务管理工具

```rust
// 工具定义示例
tool create_task {
    params: {
        content: String,
        priority: Option<"high" | "medium" | "low">,
        due_date: Option<String>,
    }
    // 在当前 Daily Note 底部追加 checklist 项
}

tool update_task_status {
    params: {
        task_reference: String,  // 通过内容模糊匹配
        new_status: "todo" | "done",
    }
    // 更新 Markdown 中的 checkbox 状态
}

tool list_tasks {
    params: {
        filter: Option<"today" | "overdue" | "high_priority">,
    }
    // 查询索引返回任务列表
}
```

### 索引策略

```sql
-- 可选：任务索引（从 Markdown checklist 派生）
checklist_tasks (
    id INTEGER PRIMARY KEY,
    note_path TEXT NOT NULL,      -- 所属笔记路径
    line_number INTEGER,          -- 行号，用于定位
    content TEXT NOT NULL,        -- 任务内容（去除标记）
    raw_line TEXT,                -- 原始行内容
    status TEXT,                  -- "todo" | "done"
    priority TEXT,                -- "high" | "medium" | "low" | null
    due_date TEXT,                -- ISO 日期或 null
    completed_at TEXT,            -- 完成时间
    last_indexed TEXT
)

-- 索引可完全重建：扫描所有 .md 文件，提取 checklist 项
```

## 非功能需求

- 任务索引重建时间 < 5 秒（针对一般用户 vault 大小）
- Agent 任务工具调用延迟 < 300ms（文件写入）
- Tasks 面板加载时间 < 500ms（从 SQLite 查询）

---

## 变更记录

| 日期 | 变更 | 说明 |
|------|------|------|
| 2026-04-28 | 重构 | 任务从一等公民降维为 Markdown checklist，Agent 驱动管理 |
