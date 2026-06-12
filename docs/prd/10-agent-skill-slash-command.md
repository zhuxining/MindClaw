> **Status**: `active`
> **Blueprint**: `docs/blueprint/00-overview.md`

# PRD: Agent / Skill / SlashCommand Control Plane

## 1. Objective

### Problem

用户可能已经拥有多个底层 ACP Server 或 Agent 执行后端，但这些后端各自有不同的角色、prompt、技能和会话管理方式。如果 MindClaw 只把消息转发给 ACP Server，就会退化成渠道转发器，无法形成自己的控制平面价值。

### Target Users

- 希望跨 ACP Server 复用 Agent 角色和 Skill 的开发者。
- 希望用显式命令选择任务能力的重度 LLM 用户。
- 希望让默认 Agent 进入 Feishu 等 IM 消息流的 MindClaw 内测用户。

### Desired Outcome

MindClaw 拥有用户侧 Agent 模型：它管理 Agent、Skill、默认 ACP Server、SlashCommand 和会话执行状态；ACP Server 只负责具体执行。

## 2. Success Criteria

- [ ] 用户可以配置默认 Agent，并将其绑定到默认 ACP Server。
- [ ] 用户可以配置 Skill instruction，并让 Agent 使用该 Skill。
- [ ] 无 slash command 的消息使用默认 Agent。
- [ ] SlashCommand 可以显式选择 Agent、Skill 或快捷任务。
- [ ] ConversationExecutionState 能按 `channel + conversation_id` 隔离当前 Agent / Skill 状态。
- [ ] 处理结果能记录 agent_id、skill_id、acp_server_id。
- [ ] SlashCommand 路径不读取 legacy RouteRule。

## 3. Scope

### In Scope

- ACP Server 作为 Agent 的执行后端绑定目标。
- Agent 名称、描述、Identity / system instruction、默认 ACP Server。
- Skill 名称、描述、instruction、启用状态。
- Agent 与 Skill 的绑定关系。
- 全局默认 Agent。
- SlashCommand parser 与显式执行语义。
- `/help`、`/default`、`/use`、`/skill`。
- 任意 `/<name>` 作为显式执行入口。
- ConversationExecutionState：按会话保存当前 Agent / Skill。
- 执行元数据：agent_id、skill_id、acp_server_id、status、error。

### Out of Scope

- 自动 Agent 路由、关键词分发、RouteRule 混合优先级。
- 多 Agent 并行处理同一消息。
- Agent marketplace。
- Skill 参数 schema。
- ACP Tool 管理 UI。
- 模型供应商抽象层。
- 底层 Agent runtime 实现。
- 团队共享 Agent / Skill 权限。

## 4. User Stories

### Story 1: Configure Default Agent

**As a** MindClaw 用户，**I want** 配置一个默认 Agent，**so that** 无 slash command 的 Feishu 消息拥有明确执行者。

**Priority**: P0

**Acceptance Criteria**:

- [ ] 用户可查看当前默认 Agent。
- [ ] 用户可设置一个启用状态的 Agent 为默认 Agent。
- [ ] 默认 Agent 必须绑定一个可用 ACP Server。
- [ ] 默认 Agent 包含 Identity / system instruction。
- [ ] 无 slash command 的消息使用默认 Agent。
- [ ] 默认 Agent 不可用时，不调用 ACP Server，并记录可展示错误。

### Story 2: Configure Agent Identity

**As a** 重度 LLM 用户，**I want** 编辑 Agent 的身份和行为约束，**so that** 同一 ACP Server 可以按不同角色处理任务。

**Priority**: P0

**Acceptance Criteria**:

- [ ] 用户可编辑 Agent 名称和描述。
- [ ] 用户可编辑 Identity / system instruction。
- [ ] Identity 会进入 agent_context 组装结果。
- [ ] Agent 保存后，后续消息使用更新后的 Identity。
- [ ] 旧消息保留原执行元数据，不被新配置覆盖。

### Story 3: Configure Skill Instruction

**As a** MindClaw 用户，**I want** 配置可复用 Skill，**so that** 同一个任务能力可以被多个 Agent 使用。

**Priority**: P1

**Acceptance Criteria**:

- [ ] 用户可创建 Skill。
- [ ] 用户可编辑 Skill 名称、描述和 instruction。
- [ ] 用户可启用或禁用 Skill。
- [ ] 禁用 Skill 后，该 Skill 不可用于新执行。
- [ ] Skill instruction 会进入 agent_context 组装结果。

### Story 4: Bind Skill to Agent

**As a** MindClaw 用户，**I want** 将 Skill 关联到 Agent，**so that** Agent 能以特定任务能力处理消息。

**Priority**: P1

**Acceptance Criteria**:

- [ ] 用户可将一个或多个 Skill 关联到 Agent。
- [ ] 同一 Agent 不允许重复关联同一个 Skill。
- [ ] 用户可从 Agent 中移除 Skill。
- [ ] 当前 Agent 未关联某 Skill 时，`/skill` 不允许切换到该 Skill。
- [ ] 移除关联不修改历史消息的执行元数据。

### Story 5: One-shot SlashCommand Execution

**As a** MindClaw 用户，**I want** 输入 `/<name>` 显式执行某个 Agent 或快捷任务，**so that** 当前消息可以使用不同处理方式，但不影响后续会话状态。

**Priority**: P0

**Acceptance Criteria**:

- [ ] SlashCommand parser 能识别任意 `/<name>`。
- [ ] one-shot 命令只影响当前消息。
- [ ] one-shot 命令不修改当前会话默认 Agent / Skill。
- [ ] 命令不存在时，不调用 ACP Server，并返回可展示错误。
- [ ] 命令目标不可用时，不调用 ACP Server，并返回可展示错误。
- [ ] 处理结果展示实际使用的 Agent、Skill、ACP Server。

### Story 6: Sticky Session Agent Selection

**As a** MindClaw 用户，**I want** 使用 `/use` 切换当前会话 Agent，**so that** 后续消息由指定 Agent 处理。

**Priority**: P1

**Acceptance Criteria**:

- [ ] `/use <agent>` 更新当前 `channel + conversation_id` 的 Agent。
- [ ] sticky 切换只影响当前会话。
- [ ] 切换后后续普通消息使用新的会话 Agent。
- [ ] `/default` 将当前会话恢复到全局默认 Agent。
- [ ] UI 或消息详情可展示当前会话 Agent。
- [ ] 旧消息保留原 Agent 元数据。

### Story 7: Sticky Session Skill Selection

**As a** MindClaw 用户，**I want** 使用 `/skill` 选择当前会话 Skill，**so that** 后续消息使用指定任务能力处理。

**Priority**: P2

**Acceptance Criteria**:

- [ ] `/skill <skill>` 检查当前 Agent 是否关联该 Skill。
- [ ] 当前 Agent 关联该 Skill 时，更新当前会话 Skill。
- [ ] 当前 Agent 未关联该 Skill 时，不更新会话状态，并展示错误。
- [ ] 后续普通消息使用当前会话 Agent 和当前 Skill。
- [ ] `/default` 将当前会话 Skill 恢复为默认状态。

### Story 8: Isolate Legacy RouteRule

**As a** MindClaw 用户，**I want** SlashCommand 执行不受旧 RouteRule 影响，**so that** 显式命令结果可解释。

**Priority**: P0

**Acceptance Criteria**:

- [ ] SlashCommand 解析不读取 RouteRule。
- [ ] 无 slash command 的消息不经过 RouteRule 自动匹配。
- [ ] RouteRule 不改变当前会话 Agent。
- [ ] RouteRule 不改变当前会话 Skill。
- [ ] legacy RouteRule 入口如仍存在，必须标记为 legacy 或隐藏。

## 5. Non-functional Requirements

- **Security**：Agent / Skill 内容默认仅本地保存，不上传 MindClaw 云端。
- **Reliability**：会话状态必须按 `channel + conversation_id` 隔离。
- **Explainability**：每次执行都应能展示使用的 Agent、Skill 和 ACP Server。
- **Compatibility**：Desktop UI、CLI 和 webhook 等入口应共享同一命令语义。

## 6. Open Questions

- [ ] MVP 中是否需要暴露完整 Skill 创建 UI，还是只允许编辑默认 Skill instruction？
- [ ] `/<name>` 应优先匹配 Agent、Skill 还是 CommandStore 中的显式命令？
- [ ] `/default` 是否同时重置 Agent 和 Skill，还是只重置 Agent？
- [ ] 执行元数据应挂在 `AgentResponse`、`DispatchResult`，还是独立 `MessageExecution` 实体？

## 7. Related Docs

- `docs/blueprint/00-overview.md`
- `docs/prd/20-acp-native-feishu-agent-mvp.md`
- `docs/architecture/40-agent-skill-command.md`
- `docs/architecture/35-agent-context.md`
- `docs/architecture/30-acp-client.md`
- `docs/architecture/reference/traceability.md`
- `docs/architecture/reference/migration.md`
