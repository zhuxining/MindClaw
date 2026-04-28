> **Status**: `draft`

# Markdown checklist 任务

→ 架构关联：[08-desktop-frontend.md](../architecture/08-desktop-frontend.md)
→ 父文档：[00-overview.md](00-overview.md)

## 背景与目标

**背景**：MindClaw 中的任务是 Markdown 内容的一种表达形式。用户可以在 Daily、Inbox、Vault 笔记中写入 checklist，Agent 可帮助识别、定位、补充提醒和更新状态，但任务不独立成为工作台主入口。

**目标**：定义 checklist 的识别、创建、勾选、定位和轻量提醒行为。

## 功能描述

### US-01 识别 checklist

作为用户，我希望系统识别 Markdown 中的 checklist，以便在 Daily、Vault 和 Agent Session 中引用这些待办内容。

**验收标准**：

Given Markdown 笔记中存在 `- [ ]` 或 `- [x]` 行
When 笔记保存完成
Then 系统把这些行识别为 checklist 项

Given checklist 行包含 `@YYYY-MM-DD`
When 系统展示 checklist 项
Then 展示该日期为截止日期

Given checklist 行包含 `!high`、`!medium` 或 `!low`
When 系统展示 checklist 项
Then 展示对应优先级

- [ ] checklist 真相保留在 Markdown 正文中。
- [ ] 识别结果只作为视图和 Agent 协作辅助。
- [ ] 普通列表不显示为 checklist。

**优先级**：P1

---

### US-02 创建 checklist

作为用户，我希望通过直接编辑或 Agent Session 创建 checklist，以便把待办保留在相关笔记中。

**验收标准**：

Given 用户在 Markdown 编辑器中输入 `- [ ] 完成报告`
When 文件保存完成
Then 该行显示为未完成 checklist 项

Given 用户在 Agent Session 中要求记录待办
When 用户确认 Agent 建议
Then checklist 项追加到用户指定的 Daily、Inbox 或 Vault 笔记中

Given 用户未指定目标笔记
When 用户确认 Agent 建议
Then checklist 项追加到今日 Daily Note

**优先级**：P1

---

### US-03 更新 checklist 状态

作为用户，我希望通过勾选或 Agent Session 更新 checklist 状态，以便让 Markdown 正文反映任务进展。

**验收标准**：

Given Markdown 编辑器中存在未完成 checklist
When 用户点击复选框
Then 原文从 `- [ ]` 更新为 `- [x]`

Given Markdown 编辑器中存在已完成 checklist
When 用户取消勾选
Then 原文从 `- [x]` 更新为 `- [ ]`

Given 用户在 Agent Session 中要求标记某项完成
When 用户确认 Agent 匹配结果
Then 对应 checklist 行更新为已完成

- [ ] Agent 匹配到多条候选时必须让用户选择。
- [ ] 更新状态后，中央内容区显示最新 Markdown 内容。
- [ ] 状态更新失败时保留原文。

**优先级**：P1

---

### US-04 定位 checklist

作为用户，我希望从 Daily、回顾视图或 Agent Session 定位 checklist，以便回到原始上下文处理它。

**验收标准**：

Given checklist 项显示在 Daily 摘要、回顾视图或 Agent Session 中
When 用户点击该项
Then 中央内容区打开来源笔记，并滚动到 checklist 所在位置

Given checklist 来源笔记不存在
When 用户点击该项
Then 系统显示来源不可用状态，并提供清除该引用的入口

**优先级**：P1

---

### US-05 轻量提醒 checklist

作为用户，我希望 Agent 在合适的回顾场景中提示临近截止或长期未处理的 checklist，以便减少遗漏。

**验收标准**：

Given 存在截止日期为明天的未完成 checklist
When 用户打开 Review Today
Then 回顾视图显示该 checklist 为临近截止

Given 存在超过 3 天未处理且无截止日期的 checklist
When 用户打开 Review Today
Then 回顾视图显示该 checklist 为待确认

Given 用户选择忽略某条提醒
When 操作完成
Then 该提醒在当天回顾中不再显示

- [ ] checklist 提醒只出现在回顾视图或 Agent Session 中。
- [ ] checklist 不触发系统级推送。
- [ ] Tasks 不作为 MVP Ribbon 主入口。

**优先级**：P2

## 范围界定

**In Scope**：

- Markdown checklist 识别。
- 直接编辑或 Agent Session 创建 checklist。
- 勾选、取消勾选和 Agent 确认后更新状态。
- 从摘要、回顾、Session 定位到原始笔记。
- 回顾场景中的轻量提醒。

**Out of Scope**：

- 独立任务对象：任务真相在 Markdown 正文中，独立对象会偏离产品定位。
- 甘特图、依赖关系、子任务树：这些能力属于重度项目管理工具范畴。
- 系统级通知：MVP 通过回顾和 Session 提示处理 checklist。
- Ribbon Tasks 入口：第一版主导航聚焦知识共建和 Agent 演化。

## 非功能需求

- 保存笔记后 checklist 识别结果在 1 秒内更新。
- 从 checklist 跳转到来源位置的响应时间不超过 500ms。
