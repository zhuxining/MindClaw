> **Status**: `draft`

# Daily 与 Inbox 待处理闭环

→ 架构关联：[08-desktop-frontend.md](../architecture/08-desktop-frontend.md)
→ 父文档：[00-overview.md](00-overview.md)

## 背景与目标

**背景**：MindClaw 的输入不只来自对话。用户每天的记录、临时想法、链接、片段、外部资料解析结果、知识草稿和 Agent 候选都需要先进入一个可集中处理的位置，再由用户确认去向。Inbox 是待整理、待审核、待沉淀 Markdown 产物的统一集散地。

**目标**：定义 Daily 与 Inbox 的打开、记录、捕获、待处理审核、分流和回顾入口，让输入从暂存内容转化为可审阅知识材料、Agent 记忆或共有知识。

## 功能描述

### US-01 打开今日 Daily Note

作为用户，我希望从 Ribbon 或 Open Today 打开今日 Daily Note，以便立即记录当天工作和想法。

**验收标准**：

Given 用户点击 Ribbon 中的 Daily 或全局动作 Open Today
When 今日 Daily Note 已存在
Then 中央内容区打开 `{vault}/daily/YYYY-MM-DD.md`

Given 今日 Daily Note 不存在
When 用户打开今日 Daily Note
Then 系统创建对应 Markdown 文件并在中央内容区打开

- [ ] 日期格式固定为 `YYYY-MM-DD`。
- [ ] Daily 工作域左侧面板显示日期列表，今天位于列表顶部。
- [ ] 今日 Daily Note 打开后，右侧面板显示大纲、Frontmatter、关联内容。

**优先级**：P0

---

### US-02 编辑 Daily Note

作为用户，我希望在 Daily Note 中直接编辑 Markdown，以便记录事实、想法、复盘和 checklist。

**验收标准**：

Given Daily Note 已打开
When 用户输入内容
Then 编辑器实时显示 Markdown 内容

Given 用户停止输入 1 秒
When 当前内容存在变更
Then 系统保存 Daily Note，并在状态栏显示已保存状态

Given 保存失败
When 编辑器收到失败结果
Then 状态栏显示保存失败，并保留用户未保存内容

- [ ] 编辑器支持标题、加粗、斜体、列表、引用、代码块、链接。
- [ ] Cmd+S 触发立即保存。
- [ ] Daily Note 中的 checklist 按 [08-checklist-tasks.md](08-checklist-tasks.md) 处理。

**优先级**：P0

---

### US-03 捕获和接收 Inbox 条目

作为用户，我希望把想法、链接、片段、外部资料解析结果或 Agent 候选统一放入 Inbox，以便先集中处理再决定去向。

**验收标准**：

Given 用户点击 Ribbon 中的 Inbox
When Inbox 工作域加载完成
Then 左侧面板显示待处理条目列表，中央内容区显示 Inbox 列表或最近选中条目

Given 用户点击 Add Link
When 用户输入 URL 和可选说明
Then 新条目出现在 Inbox 列表顶部，并标记为待处理

Given 用户在 Inbox 中点击 New Capture
When 用户输入文本并保存
Then 新条目出现在 Inbox 列表顶部，并标记为待处理

Given PDF、网页或文件完成解析
When 解析结果可供用户处理
Then 系统在 Inbox 中创建解析结果条目，并保留原始来源入口

Given Agent 生成观察候选、记忆更新建议或经验教训候选
When 候选需要用户判断
Then 系统在 Inbox 中创建审核条目，并标记候选类型

- [ ] Inbox 条目支持文本、链接、摘录、解析结果、知识草稿、观察候选、记忆建议、经验候选八种可见类型。
- [ ] Inbox 条目显示创建时间、来源类型、处理类型和处理状态。
- [ ] Inbox 条目可打开为中央内容区 Tab。
- [ ] Inbox 默认列表只显示待处理和处理中条目。

**优先级**：P0

---

### US-04 处理 Inbox 条目

作为用户，我希望把 Inbox 条目处理到 Daily、Vault、Agent Memory、演化记录或 Agent Session，以便让临时内容进入明确归属。

**验收标准**：

Given Inbox 条目已打开
When 用户选择 Move to Daily
Then 条目内容追加到当前日期 Daily Note，并显示处理去向

Given Inbox 条目已打开
When 用户选择 Create Vault Draft
Then 中央内容区打开 Vault 草稿，草稿包含可编辑标题、`tags`、`overview`、`confidence` 和正文

Given 用户保存 Vault 草稿
When 用户选择主题位置或存在匹配用户规则
Then 内容保存到对应 Vault 位置，Inbox 条目显示目标链接

Given Inbox 条目处理完成但没有明确去向
When 用户选择 Archive 或关闭处理
Then 条目进入归档列表，并保留原始内容和来源引用

Given Inbox 条目是记忆更新建议
When 用户点击 Confirm Memory
Then 该建议写入 Agent Memory，并生成演化记录入口

Given Inbox 条目是经验教训候选
When 用户点击 Confirm Review
Then 该条目可以保存为 Agent Memory，或显示 Save to Vault 入口

Given Inbox 条目已打开
When 用户选择 Send to Agent
Then Agent Session 打开，并把该条目加入引用上下文

Given 用户拒绝 Inbox 条目
When 操作完成
Then 该条目标记为已拒绝，并从默认待处理列表移除

- [ ] Private 条目不提供 Send to Agent 操作。
- [ ] 有明确目标的已处理条目显示目标去向，并从默认待处理列表移除。
- [ ] 无明确目标、被拒绝或用户选择归档的条目进入归档列表。
- [ ] 用户可从归档列表恢复条目为待处理。
- [ ] 确认、拒绝、归档不会删除原始来源引用。

**优先级**：P1

---

### US-05 从 Daily 或 Inbox 进入轻量回顾

作为用户，我希望对当天记录和待处理材料进行轻量回顾，以便识别可审核的观察候选、记忆建议、经验教训候选或知识草稿。

**验收标准**：

Given 用户处于 Daily 工作域
When 用户点击 Review Today
Then 系统打开当日回顾视图，列出今日新增内容、待处理 Inbox 条目、checklist 变化和观察候选

Given 用户处于 Inbox 工作域
When 用户点击 Review Inbox
Then 系统打开 Inbox 回顾视图，列出待处理条目、审核候选、整理建议和可生成的知识草稿入口

Given 用户确认某条整理建议
When 操作完成
Then 条目进入对应去向：Daily、Vault 主题位置、Agent Memory、演化记录、Agent Session 或归档

- [ ] Daily 与 Inbox 回顾不直接写入共有知识。
- [ ] 观察候选必须经过用户确认或后续回顾才能进入 Agent Memory。
- [ ] Inbox 回顾可以按来源类型、处理类型和处理状态筛选。

**优先级**：P1

## 范围界定

**In Scope**：

- Daily Note 打开、创建、编辑、保存、日期列表。
- Inbox 条目捕获、接收解析结果、接收审核候选、查看、处理、分流、归档、恢复。
- Daily 与 Inbox 进入轻量回顾视图。
- 与 Agent Session、Vault 草稿、Checklist 任务的入口衔接。

**Out of Scope**：

- 日历月视图：日期列表已覆盖 MVP 的 Daily 导航。
- Daily 模板系统：模板会引入独立配置和变量规则，不进入本功能。
- Inbox 自动分类：MVP 由用户触发处理，自动分类需先验证整理结果质量。
- 外部浏览器插件捕获：桌面端内部 Add Link 已覆盖第一版捕获路径。
- Source 原始文件预览器：Inbox 只提供来源入口，复杂预览由 Source / Vault 能力单独定义。

## 非功能需求

- 打开今日 Daily Note 的可见响应时间不超过 500ms。
- Inbox 新条目保存后在 200ms 内出现在列表顶部。
- Daily 自动保存防抖时间为 1 秒。
