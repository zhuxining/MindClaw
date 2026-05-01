> **Status**: `draft`

# Vault 共享知识空间

→ 架构关联：[06-storage.md](../architecture/06-storage.md)
→ 父文档：[00-overview.md](00-overview.md)

## 背景与目标

**背景**：Vault 是 MindClaw 的共享知识空间。它承载人类笔记、共有知识、Agent 生成草稿、经确认的可复用经验，以及可审阅的 Agent 演化资产。Markdown 文件是内容真相源，Frontmatter 是人类和 Agent 共同使用的轻量索引。

**目标**：定义 Vault 的浏览、编辑、搜索、Frontmatter 展示、`tags`、`overview` 与 `confidence` 使用规则，以及知识草稿和经验教训候选保存行为。

Vault 共有知识可以按主题、项目或用户规则组织。保存入口需要支持用户选择目标位置；未选择时，系统可按既有规则、当前工作域或 `tags` 给出建议，但不要求新增复杂 Frontmatter 字段。

## 功能描述

### US-01 浏览 Vault 文件树

作为用户，我希望从 Ribbon 打开 Vault 并浏览知识库文件，以便找到和编辑已有知识。

**验收标准**：

Given 用户点击 Ribbon 中的 Vault
When Vault 工作域加载完成
Then 左侧面板显示 Vault 文件树，中央内容区打开最近访问的 Vault 笔记或 Vault 首页

Given 用户点击文件树中的 Markdown 文件
When 文件加载完成
Then 中央内容区打开该文件的编辑 Tab

- [ ] 文件树以 Vault 根目录为起点。
- [ ] 文件夹可展开和折叠。
- [ ] 当前打开文件在左侧文件树中显示选中态。
- [ ] Private 目录作为当前 Vault 下的 `private/` 文件夹显示，点击后切换到 Private 工作域。

**优先级**：P0

---

### US-02 编辑 Markdown 知识笔记

作为用户，我希望直接编辑 Vault 中的 Markdown 笔记，以便维护共享知识。

**验收标准**：

Given Vault 笔记已打开
When 用户编辑正文
Then 编辑器实时显示修改后的 Markdown 内容

Given 用户停止输入 1 秒
When 当前内容存在变更
Then 系统保存文件，并在状态栏显示已保存状态

Given 用户关闭含未保存内容的 Tab
When 保存未完成
Then 系统提示保存、放弃或返回编辑

- [ ] 编辑器支持标题、加粗、斜体、列表、引用、代码块、链接。
- [ ] Cmd+S 触发立即保存。
- [ ] 任务 checklist 仍保留为 Markdown 正文内容。

**优先级**：P0

---

### US-03 查看和编辑 Frontmatter 索引

作为用户，我希望查看并维护笔记的 `tags`、`overview` 和 `confidence`，以便人类和 Agent 都能快速判断知识用途与可信程度。

**验收标准**：

Given Vault 笔记已打开
When 右侧上下文面板展开
Then Frontmatter 区域显示 `tags`、`overview` 和 `confidence`

Given 用户编辑 `tags`
When 用户保存笔记
Then `tags` 以数组形式写入 Markdown Frontmatter

Given 用户编辑 `overview`
When 用户保存笔记
Then `overview` 写入 Markdown Frontmatter，并在 Vault 搜索结果中显示

Given 用户调整 `confidence`
When 用户保存笔记
Then `confidence` 以 0.0-1.0 数值写入 Markdown Frontmatter，并在后续 Agent 召回中参与排序

- [ ] 新建知识笔记时默认包含 `title`、`tags`、`overview`、`confidence`。
- [ ] `tags` 允许为空数组。
- [ ] `overview` 允许为空字符串，但空值在右侧面板中显示待补充状态。
- [ ] `confidence` 提供低 / 中 / 高的用户可读控件，并允许查看实际数值。
- [ ] 用户修改 `confidence` 后，ContextIndex 使用最新 Frontmatter 重建或增量同步。

**优先级**：P0

---

### US-04 搜索与筛选知识

作为用户，我希望按关键词、标签和概览搜索 Vault，以便快速定位相关知识。

**验收标准**：

Given 用户在 Vault 搜索框输入不少于 2 个字符
When 输入停止 300ms
Then 中央内容区显示搜索结果列表

Given 搜索结果列表已显示
When 用户点击结果项
Then 系统打开对应 Markdown 文件并高亮当前结果

Given 用户选择某个 tag 筛选
When 筛选生效
Then 文件列表只显示包含该 tag 的笔记

- [ ] 搜索结果显示标题、路径、`tags`、`overview` 和 `confidence`。
- [ ] 搜索结果最多显示 50 条。
- [ ] 无结果时显示空状态和清除搜索入口。

**优先级**：P1

---

### US-05 保存 Inbox 草稿和经验教训候选

作为用户，我希望把 Inbox 中的知识草稿、解析结果或经验教训候选保存到 Vault，以便把审核后的临时内容变成共享知识。

**验收标准**：

Given 知识草稿已打开
When 用户点击 Save to Vault
Then 系统要求标题不为空，并保存为 Markdown 笔记

Given 用户选择目标主题或存在匹配用户规则
When 草稿保存完成
Then 新笔记出现在对应 Vault 目录、文件树和搜索结果中

Given 草稿缺少 `overview`
When 用户点击 Save to Vault
Then 系统允许保存，并在右侧面板显示待补充状态

Given 草稿保存完成
When 用户返回 Vault 工作域
Then 新笔记出现在 Vault 文件树和搜索结果中

Given 经验教训候选已打开
When 用户点击 Save to Vault
Then 系统保存为 Markdown Vault 笔记，并保留来源记忆、演化记录、Session 或 resources 链接

- [ ] 草稿保存后保留来源信息链接。
- [ ] 若来源为 Inbox 条目，保存完成后 Inbox 条目链接到新知识笔记，并从默认待处理列表移除。
- [ ] 若没有目标主题、用户规则或显式保存位置，用户可把 Inbox 来源条目标记为归档，不把正文误放入共有知识区。
- [ ] 保存到 Vault 的内容不自动变成 Agent 记忆正文。
- [ ] 若来源为经验教训候选，Agent Memory 只保留触发条件、短摘要和知识文档引用。
- [ ] 用户可在保存前编辑标题、`tags`、`overview`、`confidence` 和正文。

**优先级**：P1

## 范围界定

**In Scope**：

- Vault 文件树浏览、Markdown 打开、编辑、保存。
- `tags`、`overview` 与 `confidence` 的查看、编辑、搜索结果展示。
- 按关键词和 tag 搜索。
- 知识草稿和经验教训候选保存到 Vault。
- Private 作为 `private/` 文件夹入口显示，不在共有知识浏览中展开私密内容。

**Out of Scope**：

- 图谱视图：关系图需要独立可视化规则，不进入 Vault MVP。
- 双向链接解析：Wikilink 和 backlink 会改变笔记导航模型，单独定义。
- 复杂 Frontmatter schema：MVP 只维护 `tags`、`overview` 和单一 `confidence`，避免分类和多维评分维护成本。
- 版本历史：文件版本管理涉及回滚、冲突和存储策略，另行定义。
- 保存知识后自动改写 Agent 行为：行为变化需要通过 Agent Memory 引用和演化记录审核。

## 非功能需求

- Vault 文件树首屏在 500ms 内显示。
- 搜索结果在输入停止后 1 秒内显示。
- Markdown 笔记保存防抖时间为 1 秒。
