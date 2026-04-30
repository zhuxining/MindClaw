> **Status**: `draft`

# Agent 记忆管理

→ 架构关联：[03.10-memory.md](../architecture/03.10-memory.md)
→ 父文档：[00-overview.md](00-overview.md)

## 背景与目标

**背景**：Agent 记忆会影响后续判断和行动。用户需要查看 Agent 记住了什么、这些记忆来自哪里、是否仍然有效，并控制哪些记忆可以生成候选或引用共有知识。待确认记忆建议先进入 Inbox；确认后的记忆本身是可审阅资产，以 Markdown + Frontmatter 作为真相源。

**目标**：定义 Agent Memory 工作域的查看、分类、确认、修正、删除、降权、候选生成和知识引用行为。

## 功能描述

### US-01 打开 Agent Memory 工作域

作为用户，我希望从 Ribbon 打开 Memory 工作域，以便审阅 Agent 当前记忆。

**验收标准**：

Given 用户点击 Ribbon 中的 Memory
When Memory 工作域加载完成
Then 左侧面板显示记忆展示分类，中央内容区显示记忆列表

Given 记忆列表已显示
When 用户选择展示分类
Then 列表只显示该分类下的记忆

- [ ] MVP 展示分类为用户背景、用户偏好、观察假设、Agent 经验。
- [ ] 列表项显示标题、归属、类型、状态、置信度、最近更新时间。
- [ ] 右侧上下文面板显示选中记忆的来源、证据和可执行动作。
- [ ] 每条记忆提供打开源 Markdown 的入口。

**优先级**：P0

---

### US-02 查看二维分类

作为用户，我希望看到记忆的归属和类型，以便理解它如何影响 Agent。

**验收标准**：

Given 用户打开任一记忆详情
When 详情加载完成
Then 页面显示归属 `user`、`agent`、`shared` 之一

Given 用户打开任一记忆详情
When 详情加载完成
Then 页面显示类型 `profile`、`preference`、`entity`、`event`、`observation`、`case`、`pattern`、`procedure`、`constraint` 之一

Given 记忆归属为 `shared`
When 用户查看详情
Then 页面显示对应共有知识链接

- [ ] 用户无需手动填写二维分类。
- [ ] 分类用于解释记忆边界、候选生成和引用路径。
- [ ] 展示分类与内部类型映射关系在详情页可查看。
- [ ] `shared` 记忆只显示摘要、触发条件和共有知识引用，不复制知识正文。
- [ ] 归属、类型、状态、置信度和引用关系可在源 Markdown Frontmatter 中查看。

**优先级**：P0

---

### US-03 确认或修正记忆

作为用户，我希望确认或修正 Agent 记忆，以便让 Agent 后续行动基于准确上下文。

**验收标准**：

Given Inbox 中存在待确认记忆建议
When 用户点击 Confirm
Then 系统创建或更新 Agent 记忆，源 Inbox 条目归档并保留目标记忆引用

Given 用户点击 Edit
When 用户修改记忆内容并保存
Then 该记忆显示修改后内容，源 Markdown 记录用户修正来源

Given 用户调整记忆置信度
When 用户保存
Then `confidence` 写入记忆 Markdown Frontmatter，并影响后续召回排序

Given 用户取消编辑
When 操作完成
Then 记忆内容保持不变

- [ ] 已确认记忆仍可再次编辑。
- [ ] 用户修正后的记忆高于原 Agent 观察。
- [ ] 用户手动设置的置信度高于 Agent 自动估计值。
- [ ] 修正动作生成演化记录。
- [ ] 记忆列表在保存后使用更新后的 Markdown 索引。

**优先级**：P0

---

### US-04 删除或降权记忆

作为用户，我希望删除错误记忆或降低不可靠记忆权重，以便控制 Agent 后续行为。

**验收标准**：

Given 用户打开记忆详情
When 用户点击 Delete 并确认
Then 该记忆从默认列表中移除，并不再影响 Agent 行动

Given 用户打开记忆详情
When 用户点击 Downgrade
Then 该记忆状态变为低可信，并显示降权原因输入框

Given 用户提交降权原因
When 操作完成
Then 记忆详情显示降权原因和操作时间

- [ ] 删除操作需要二次确认。
- [ ] 删除和降权动作生成演化记录。
- [ ] 已删除记忆不出现在默认列表。

**优先级**：P0

---

### US-05 生成候选与维护知识引用

作为用户，我希望把稳定记忆生成经验教训候选或链接到共有知识，以便让可复用内容离开黑盒记忆。

**验收标准**：

Given 记忆状态为已确认
When 用户点击 Create Candidate
Then 系统展示可生成选项：经验教训候选、知识草稿

Given 用户选择经验教训候选
When 操作完成
Then 系统在 Inbox 中生成经验教训候选，并关联原记忆和来源证据

Given 用户选择共有知识草稿
When 操作完成
Then 中央内容区打开知识草稿，包含标题、`tags`、`overview`、`confidence` 和正文

Given 用户把已确认知识链接到记忆
When 操作完成
Then 该记忆显示触发条件、短摘要和知识文档引用路径

- [ ] 候选生成动作生成演化记录。
- [ ] 生成的候选先出现在 Inbox，不直接写入 Vault 或 Agent 长期资产。
- [ ] 经验教训正文不复制到 Agent Memory 文档。
- [ ] 未确认记忆不能生成共有知识草稿。

**优先级**：P1

## 范围界定

**In Scope**：

- Agent Memory 工作域入口和记忆列表。
- MVP 展示分类与内部二维分类展示。
- 记忆确认、编辑、删除、降权。
- 记忆置信度查看和人工调整。
- 源 Markdown 入口和 Frontmatter 状态展示。
- 记忆生成经验教训候选或知识草稿。
- 已确认知识与 Agent 记忆的引用关系。
- 与演化记录的关联入口。

**Out of Scope**：

- 用户手工维护完整分类体系：二维分类由系统呈现，MVP 不要求用户填写。
- 自动把所有对话变成记忆：记忆需要观察、确认或回顾流程。
- Private 内容生成记忆：该行为违反 Private 隔离边界。
- 跨设备记忆同步：同步会引入身份、冲突和权限问题。

## 非功能需求

- Memory 列表首屏在 500ms 内显示。
- 记忆确认、删除、降权动作完成后在 200ms 内更新列表状态。
- 记忆详情必须显示来源信息或无来源状态。
