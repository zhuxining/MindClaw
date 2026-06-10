> **Status**: `draft`

# 功能需求规格：Agent、Skill 与 SlashCommand 显式执行选择

## § 背景与目标

**背景**：MindClaw 的默认消息闭环使用一个默认 Agent 处理无命令消息。用户在不同任务中需要使用不同身份、技能和 ACP Server 执行消息处理，例如代码审查、研究总结、消息回复和翻译。

**目标**：提供 Agent、Skill、ACP Server 和 SlashCommand 管理能力，让用户通过 `/命令` 显式选择 Agent 或 Skill 处理当前消息，并能为当前会话保存执行状态。

## § 功能描述

### Story 1：管理 ACP Server

**作为** MindClaw 用户，**我希望** 注册和测试 ACP Server，**以便** Agent 能绑定可用的执行后端。

**优先级**：P0

**验收标准**：

- [ ] 用户可新增 ACP Server。
- [ ] 用户可编辑 ACP Server 的名称、说明和连接配置。
- [ ] 用户可测试 ACP Server 连接。
- [ ] 测试成功时，ACP Server 状态显示为可用。
- [ ] 测试失败时，ACP Server 状态显示为不可用并展示错误原因。
- [ ] ACP Server 不可用时，绑定它的 Agent 显示不可用状态。
- [ ] 用户删除 ACP Server 前，系统展示受影响的 Agent 列表。

---

### Story 2：管理 Agent 与默认 Identity

**作为** MindClaw 用户，**我希望** 创建多个 Agent，并为每个 Agent 配置默认 Identity，**以便** 不同 Agent 以不同身份和行为约束执行任务。

**优先级**：P0

**验收标准**：

- [ ] 用户可创建 Agent。
- [ ] 每个 Agent 必须拥有一个默认 Identity。
- [ ] 用户可编辑 Agent 名称和描述。
- [ ] 用户可编辑 Identity 的 system prompt 和行为约束。
- [ ] 用户必须为 Agent 选择一个默认 ACP Server。
- [ ] 用户可启用或禁用 Agent。
- [ ] 禁用 Agent 后，该 Agent 不出现在 slash command 候选列表中。
- [ ] 禁用 Agent 后，已使用该 Agent 的会话显示 Agent 不可用状态。

---

### Story 3：管理 Skill

**作为** MindClaw 用户，**我希望** 独立管理 Skill，**以便** 多个 Agent 能复用同一任务能力。

**优先级**：P1

**验收标准**：

- [ ] 用户可创建 Skill。
- [ ] 用户可编辑 Skill 名称、描述和 instruction。
- [ ] 用户可启用或禁用 Skill。
- [ ] 禁用 Skill 后，该 Skill 不出现在新建 Agent 关联列表中。
- [ ] 禁用 Skill 后，指向该 Skill 的 SlashCommand 显示不可用状态。
- [ ] 同一个 Skill 可关联到多个 Agent。

---

### Story 4：将 Skill 关联到 Agent

**作为** MindClaw 用户，**我希望** 为 Agent 关联多个 Skill，**以便** 同一个 Agent 能处理多种任务。

**优先级**：P1

**验收标准**：

- [ ] 用户可为 Agent 选择多个 Skill。
- [ ] 用户可从 Agent 中移除已关联 Skill。
- [ ] 用户可为 Agent 设置一个默认 Skill。
- [ ] Agent 没有关联 Skill 时，系统允许 Agent 以普通对话模式执行。
- [ ] 同一 Agent 不允许重复关联同一个 Skill。
- [ ] 移除 Skill 关联后，对应 SlashCommand 不再可执行。

---

### Story 5：设置默认 Agent

**作为** MindClaw 用户，**我希望** 设置一个默认 Agent，**以便** 无 `/命令` 的自动消息拥有明确执行者。

**优先级**：P0

**验收标准**：

- [ ] 用户可设置一个全局默认 Agent。
- [ ] 全局默认 Agent 必须处于启用状态。
- [ ] 全局默认 Agent 绑定的 ACP Server 必须处于可用状态。
- [ ] 无 `/命令` 的消息使用全局默认 Agent。
- [ ] 全局默认 Agent 不可用时，SessionDispatcher 不调用 ACP Server，并记录错误状态。

---

### Story 6：通过 SlashCommand one-shot 执行 Agent + Skill

**作为** MindClaw 用户，**我希望** 在消息中输入 `/命令`，**以便** 当前消息由指定 Agent 和 Skill 执行。

**优先级**：P0

**验收标准**：

- [ ] 用户输入 `/` 后，Desktop UI 展示可用命令列表。
- [ ] 命令列表展示命令名、目标 Agent 和目标 Skill。
- [ ] 用户输入有效命令后，SessionDispatcher 解析命令。
- [ ] one-shot 命令只影响当前消息。
- [ ] one-shot 命令不修改当前会话默认 Agent。
- [ ] 命令目标 Agent 不可用时，SessionDispatcher 不调用 ACP Server。
- [ ] 命令目标 Skill 不可用时，SessionDispatcher 不调用 ACP Server。
- [ ] 命令不存在时，SessionDispatcher 不调用 ACP Server，并返回可展示错误。
- [ ] 消息处理结果展示使用的 Agent、Skill 和 ACP Server。

---

### Story 7：通过 SlashCommand sticky 切换当前会话 Agent

**作为** MindClaw 用户，**我希望** 使用 `/use agent` 切换当前会话默认 Agent，**以便** 后续消息由指定 Agent 处理。

**优先级**：P1

**验收标准**：

- [ ] 用户输入 `/use agent-name` 后，系统更新当前 `channel + conversation_id` 的 Agent。
- [ ] sticky 切换只影响当前会话。
- [ ] 切换后当前会话后续普通消息使用新的 Agent。
- [ ] 用户输入 `/default` 后，当前会话恢复全局默认 Agent。
- [ ] Desktop UI 展示当前会话 Agent。
- [ ] 旧消息保留原处理 Agent 信息。

---

### Story 8：选择当前会话 Skill

**作为** MindClaw 用户，**我希望** 使用 `/skill skill-name` 选择当前会话 Skill，**以便** 当前 Agent 使用指定任务能力处理后续消息。

**优先级**：P2

**验收标准**：

- [ ] 用户输入 `/skill skill-name` 后，系统检查当前 Agent 是否关联该 Skill。
- [ ] 当前 Agent 关联该 Skill 时，系统更新当前会话 Skill。
- [ ] 当前 Agent 未关联该 Skill 时，系统不更新会话状态并展示错误。
- [ ] 后续普通消息使用当前会话 Agent 和当前 Skill。
- [ ] 用户输入 `/default` 后，当前会话 Skill 恢复为 Agent 默认 Skill。

---

### Story 9：隔离 legacy RouteRule

**作为** MindClaw 用户，**我希望** SlashCommand 执行不受旧 RouteRule 影响，**以便** 显式命令结果可解释。

**优先级**：P0

**验收标准**：

- [ ] SlashCommand 解析不读取 RouteRule。
- [ ] 无 `/命令` 的消息不经过 RouteRule 自动匹配。
- [ ] legacy RouteRule 管理入口标记为 legacy 或隐藏。
- [ ] RouteRule 不改变当前会话 Agent。
- [ ] RouteRule 不改变当前会话 Skill。

## § 范围界定

### In Scope

- ACP Server 注册、编辑、测试连接和状态展示。
- Agent 创建、编辑、启用、禁用。
- Agent 默认 Identity 管理。
- Skill 创建、编辑、启用、禁用。
- Agent-Skill 多对多关联。
- 全局默认 Agent 设置。
- SlashCommand one-shot 执行。
- SlashCommand sticky 切换当前会话 Agent。
- 当前会话 Skill 选择。
- ConversationExecutionState 内存状态。
- 当前消息展示 Agent、Skill 和 ACP Server 元数据。
- legacy RouteRule 与 SlashCommand 隔离。

### Out of Scope

| 排除项 | 理由 |
|--------|------|
| 根据关键词自动选择 Agent | 本功能只处理用户显式选择，自动路由属于 v2.0 独立方向 |
| RouteRule 与 SlashCommand 混合优先级 | 混合规则会降低命令结果可解释性 |
| 多 Agent 并行处理同一消息 | 并行处理需要结果合并和冲突解释，超出显式选择范围 |
| Agent 复用 Identity | v1.1 采用 Agent 1:1 Identity，降低配置复杂度 |
| ACP Server override UI | v1.1 使用 Agent 默认 ACP Server，临时 override 放入 v1.2 |
| ConversationExecutionState 持久化 | v1.1 使用内存状态，重启后恢复全局默认 Agent |
| Skill 参数 schema | 参数 schema 属于 v1.2 命令增强 |
| ACP Tool 管理 | Skill 是任务模板，ACP Tool 执行由 `acp_client::ToolExecutor` 管理 |

## § 非功能需求

| 类别 | 约束 | 量化阈值 |
|------|------|---------|
| 性能 | SlashCommand 解析耗时 | 单条消息 ≤ 20ms |
| 性能 | AgentResolver 解析耗时 | 单条消息 ≤ 50ms |
| 可用性 | 命令错误反馈 | 无效命令在 1 秒内展示错误 |
| 可靠性 | RouteRule 隔离 | SlashCommand 路径 0 次读取 RouteRule |
| 可靠性 | 会话隔离 | 状态按 `channel + conversation_id` 隔离 |
| 安全 | ACP Server secrets | 必须使用 Stronghold，不得明文存储 |
| 安全 | Agent / Skill 内容 | 默认仅本地保存，不上传云端 |
| 兼容性 | 输入入口一致性 | Desktop UI、CLI 和 Webhook 使用同一后端命令解析语义 |
