> **Status**: `draft`

# Vault 共享知识空间

→ 架构关联：[06-storage.md](../architecture/06-storage.md)
→ 父文档：[00-overview.md](00-overview.md)

## 背景与目标

**背景**：Vault 是 MindClaw 的共享知识空间。它承载人类笔记、共有知识、Agent 生成草稿和经确认的可复用经验，Markdown 文件是已确认知识内容的真相源，Frontmatter 是人类和 Agent 共同使用的轻量索引。

**目标**：定义 Vault 的浏览、编辑、搜索、Frontmatter 展示、`tags` 与 `overview` 使用规则，以及知识草稿和经验教训候选保存行为。

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
- [ ] Private 目录在 Vault 文件树中显示为隔离入口，点击后切换到 Private 工作域。

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

作为用户，我希望查看并维护笔记的 `tags` 和 `overview`，以便人类和 Agent 都能快速判断知识用途。

**验收标准**：

Given Vault 笔记已打开
When 右侧上下文面板展开
Then Frontmatter 区域显示 `tags` 和 `overview`

Given 用户编辑 `tags`
When 用户保存笔记
Then `tags` 以数组形式写入 Markdown Frontmatter

Given 用户编辑 `overview`
When 用户保存笔记
Then `overview` 写入 Markdown Frontmatter，并在 Vault 搜索结果中显示

- [ ] 新建知识笔记时默认包含 `title`、`tags`、`overview`。
- [ ] `tags` 允许为空数组。
- [ ] `overview` 允许为空字符串，但空值在右侧面板中显示待补充状态。

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

- [ ] 搜索结果显示标题、路径、`tags`、`overview`。
- [ ] 搜索结果最多显示 50 条。
- [ ] 无结果时显示空状态和清除搜索入口。

**优先级**：P1

---

### US-05 保存知识草稿和经验教训候选

作为用户，我希望把 Agent、Inbox 或经验教训候选保存到 Vault，以便把审核后的临时内容变成共享知识。

**验收标准**：

Given 知识草稿已打开
When 用户点击 Save to Vault
Then 系统要求标题不为空，并保存为 Markdown 笔记

Given 草稿缺少 `overview`
When 用户点击 Save to Vault
Then 系统允许保存，并在右侧面板显示待补充状态

Given 草稿保存完成
When 用户返回 Vault 工作域
Then 新笔记出现在 Vault 文件树和搜索结果中

Given 经验教训候选已打开
When 用户点击 Save as Knowledge
Then 系统保存为 Markdown 知识笔记，并保留来源记忆、演化记录或 Session 链接

- [ ] 草稿保存后保留来源信息链接。
- [ ] 保存到 Vault 的内容不自动变成 Agent 记忆正文。
- [ ] 若来源为经验教训候选，Agent Memory 只保留触发条件、短摘要和知识文档引用。
- [ ] 用户可在保存前编辑标题、`tags`、`overview` 和正文。

**优先级**：P1

## 范围界定

**In Scope**：

- Vault 文件树浏览、Markdown 打开、编辑、保存。
- `tags` 与 `overview` 的查看、编辑、搜索结果展示。
- 按关键词和 tag 搜索。
- 知识草稿和经验教训候选保存到 Vault。
- Private 作为隔离入口显示，不在 Vault 中展开私密内容。

**Out of Scope**：

- 图谱视图：关系图需要独立可视化规则，不进入 Vault MVP。
- 双向链接解析：Wikilink 和 backlink 会改变笔记导航模型，单独定义。
- 复杂 Frontmatter schema：只维护 `tags` 和 `overview`，避免分类维护成本。
- 版本历史：文件版本管理涉及回滚、冲突和存储策略，另行定义。
- 保存知识后自动改写 Agent 行为：行为变化需要通过 Agent Memory 引用和演化记录审核。

## 非功能需求

- Vault 文件树首屏在 500ms 内显示。
- 搜索结果在输入停止后 1 秒内显示。
- Markdown 笔记保存防抖时间为 1 秒。
