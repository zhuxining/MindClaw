> **Status**: `draft`

# Agent Session 工作域

→ 架构关联：[03-agent-runtime.md](../architecture/03-agent-runtime.md)
→ 父文档：[00-overview.md](00-overview.md)

## 背景与目标

**背景**：Agent 是 MindClaw 中连接输入、整理、执行、反思和知识沉淀的协作能力。用户需要在工作台内与 Agent 对话、引用当前内容、查看执行过程，并把结果转化为草稿、记忆或知识。

**目标**：定义 Agent Session 的创建、消息发送、上下文引用、执行反馈、草稿生成和回顾触发行为。

## 功能描述

### US-01 打开 Agent Session

作为用户，我希望从 Ribbon 打开 Agent 工作域，以便在工作台内发起与当前知识空间相关的协作。

**验收标准**：

Given 用户点击 Ribbon 中的 Agent
When Agent 工作域加载完成
Then 左侧面板显示会话列表，中央内容区打开最近一次 Agent Session

Given 当前没有 Agent Session
When 用户进入 Agent 工作域
Then 中央内容区创建一个空 Session，输入框获得焦点

- [ ] New Session 全局动作创建新 Session Tab。
- [ ] Session 标题显示创建时间或用户设置的名称。
- [ ] Session 作为中央内容区 Tab 存在，不覆盖 Vault、Daily、Private 等工作域。

**优先级**：P0

---

### US-02 发送消息并查看回复

作为用户，我希望在 Session 中向 Agent 输入问题或指令，以便让 Agent 协助整理、分析、执行或生成内容。

**验收标准**：

Given Agent Session 已打开
When 用户输入文本并按 Enter
Then 用户消息显示在消息列表中，输入框清空，Agent 开始响应

Given Agent 正在响应
When 回复内容产生
Then Agent 消息以流式文本显示，并在状态区展示当前阶段

Given Agent 回复完成
When 用户查看消息
Then 消息支持 Markdown 渲染，代码块、列表、引用正常显示

- [ ] Shift+Enter 在输入框内换行。
- [ ] 输入框为空时发送按钮禁用。
- [ ] Agent 响应期间显示停止按钮。
- [ ] 用户停止响应后，当前 Agent 消息标记为已中断。

**优先级**：P0

---

### US-03 引用当前上下文

作为用户，我希望把当前打开的笔记、Inbox 条目或 Vault 文件作为上下文引用给 Agent，以便 Agent 基于明确材料协作。

**验收标准**：

Given 中央内容区激活一个非 Private 对象
When 用户在 Agent Session 中点击 Add Context
Then 当前对象出现在右侧上下文面板的引用列表中

Given 引用列表已有对象
When 用户发送消息
Then Agent 回复区域显示本次使用的引用对象名称

Given 当前对象属于 Private
When 用户尝试添加上下文
Then Add Context 操作禁用，并显示 Private 内容不可发送给 Agent 的说明

- [ ] 引用对象可从引用列表移除。
- [ ] 同一对象重复添加时只保留一条引用。
- [ ] 引用列表显示对象名称、来源工作域和最后更新时间。

**优先级**：P0

---

### US-04 查看 Agent 执行反馈

作为用户，我希望看到 Agent 当前正在做什么，以便判断执行是否符合预期。

**验收标准**：

Given Agent 正在处理用户请求
When Agent 进入不同阶段
Then Session 状态区显示阶段名称：理解请求、读取上下文、生成草稿、等待确认、完成

Given Agent 需要用户确认
When 中央内容区显示确认卡片
Then 用户可选择确认、拒绝或编辑后确认

Given 用户拒绝 Agent 建议
When 拒绝操作完成
Then Agent 停止执行该建议，并在 Session 中记录用户拒绝结果

- [ ] Agent 未经确认不得直接把草稿保存为共有知识。
- [ ] Agent 未经确认不得修改 Private 内容。
- [ ] 执行失败时显示失败原因和可重试入口。

**优先级**：P0

---

### US-05 生成并处理草稿

作为用户，我希望 Agent 能把对话结果生成可审阅草稿，以便把有价值内容沉淀到 Vault。

**验收标准**：

Given Agent 回复中包含可沉淀内容
When 用户点击 Create Draft
Then 中央内容区打开知识草稿 Tab，草稿包含标题、`tags`、`overview` 和正文

Given 草稿已打开
When 用户点击保存到 Vault
Then 草稿保存为 Vault 中的 Markdown 笔记，并关闭草稿状态

Given 用户关闭草稿 Tab
When 草稿未保存
Then 系统提示保存、放弃或返回编辑

- [ ] 草稿默认不进入共有知识，保存后才成为 Vault 内容。
- [ ] 草稿保存前可编辑标题、`tags`、`overview` 和正文。
- [ ] 草稿来源显示关联 Session。

**优先级**：P1

---

### US-06 触发 Session 轻量回顾

作为用户，我希望在一次协作结束后触发轻量回顾，以便确认本次过程是否产生观察候选、记忆建议或经验教训候选。

**验收标准**：

Given Agent Session 中存在执行结果、用户纠正或失败记录
When 用户点击 Review Session
Then 系统打开回顾视图，列出观察候选、记忆更新建议、经验教训候选和知识草稿入口

Given 回顾视图已打开
When 用户确认某条记忆更新建议
Then 该建议进入 Agent Memory，并生成演化记录

Given 用户确认某条经验教训候选建议
When 操作完成
Then 系统生成经验教训候选，并关联本次 Session 来源

- [ ] 经验教训候选不自动保存为 Vault 知识文档。
- [ ] 用户拒绝的观察候选不进入 Agent Memory。
- [ ] Private 来源内容不出现在 Review Session 候选中。

**优先级**：P1

## 范围界定

**In Scope**：

- Agent 工作域打开、Session 列表、新建 Session。
- 消息发送、流式回复、停止响应。
- 非 Private 内容的上下文引用。
- Agent 执行阶段展示与用户确认。
- 生成知识草稿、保存到 Vault。
- 从 Session 触发轻量回顾。

**Out of Scope**：

- 情绪承接类模式：新版产品定位聚焦知识共建与 Agent 演化。
- 匿名倾诉类模式：Private 提供隔离边界，Agent Session 不承载私密倾诉。
- 多 Agent 人格切换：MVP 聚焦单一 Agent 协作能力，避免引入额外行为差异。
- 语音、图片、文件上传：这些输入形态需要额外解析和权限设计。

## 非功能需求

- 打开 Agent Session 后输入框在 200ms 内获得焦点。
- 发送消息后用户消息在 100ms 内出现在消息列表。
- Agent 首个可见状态在 500ms 内显示。
