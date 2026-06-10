> **Status**: `draft`

# 架构子模块：Agent、Skill 与 SlashCommand

## § 职责定位

Agent 模块负责管理 MindClaw 中可被用户选择的执行者：Agent、Identity、Skill、Agent-Skill 关联、SlashCommand 和 ConversationExecutionState；不负责渠道协议、ACP 协议传输、消息队列调度、legacy RouteRule 或本地工具执行。

## § 核心原则

1. **Agent 是执行者**：用户选择 Agent 来决定“谁来执行”；理由是 Agent 比 ACP Server、Identity 和 Skill 更贴近用户心智。
2. **Identity 归属 Agent**：每个 Agent 默认拥有自己的 Identity；理由是首版需要降低身份复用带来的配置复杂度。
3. **Skill 独立复用**：Skill 独立管理，Agent 与 Skill 多对多关联；理由是任务能力需要跨 Agent 复用。
4. **SlashCommand 显式选择**：SlashCommand 只响应用户显式输入；理由是显式命令比自动规则路由更可解释。
5. **RouteRule 隔离**：Agent 选择不读取 legacy RouteRule；理由是规则路由已退出主链路。

## § 边界与实体

### 输入

- `save_agent(agent)`：创建或更新 Agent。
- `save_skill(skill)`：创建或更新 Skill。
- `bind_skill(agent_id, skill_id)`：将 Skill 关联到 Agent。
- `save_slash_command(command)`：配置 slash command 到 Agent 或 Agent + Skill 的映射。
- `parse_input(message)`：解析对话输入中的 slash command。
- `resolve_execution_context(message, conversation)`：解析当前消息使用的 Agent、Identity、Skill 和 ACP Server。
- `set_conversation_agent(conversation, agent_id)`：切换当前会话 Agent。
- `reset_conversation(conversation)`：恢复当前会话默认 Agent。

### 输出

- `ExecutionContext`：SessionDispatcher 调用 agent_context 前使用的执行上下文。
- `ConversationExecutionState`：当前会话选中的 Agent、Skill 和 ACP session 状态。
- `SlashCommandResult`：命令解析结果，包括执行、切换、恢复默认或错误。
- `RuntimeEvent`：Agent 选择、Skill 选择、命令调用和执行失败事件。

### 核心实体

- **Agent**：用户可选择的执行者，默认拥有 Identity，绑定默认 ACP Server，并关联多个 Skill。
- **Identity**：Agent 的身份、人设和行为约束。
- **Skill**：独立管理的任务能力模板，可被多个 Agent 复用。
- **AgentSkillBinding**：Agent 与 Skill 的多对多关联。
- **AcpServer**：Agent 默认绑定的 ACP 执行后端。
- **SlashCommand**：对话中的显式选择入口，映射到 Agent 或 Agent + Skill。
- **ConversationExecutionState**：按 `channel + conversation_id` 保存当前会话执行上下文。
- **ExecutionContext**：一次消息执行所需的 Agent、Identity、Skill、ACP Server 和会话元数据。

### 错误边界

- Agent、Skill、SlashCommand 不存在或被禁用时，Agent 模块返回执行上下文解析错误。
- Agent 绑定的 ACP Server 不可用时，Agent 模块返回目标不可用错误，不调用 `acp_client`。
- Agent 模块不暴露渠道原始 payload，不处理 ACP 协议错误，不读取 legacy RouteRule。

## § 关键流程

### SlashCommand one-shot 执行流程

```mermaid
sequenceDiagram
    participant SD as SessionDispatcher
    participant AG as AgentResolver
    participant CP as SlashCommandParser
    participant ACX as agent_context
    participant ACP as acp_client

    SD->>CP: parse(message.content)
    CP-->>SD: command + args
    SD->>AG: resolve(command, conversation)
    AG-->>SD: ExecutionContext
    SD->>ACX: build_request(context, message)
    ACX-->>SD: AcpRequest
    SD->>ACP: send_to_server(context.acp_server, request)
```

### 当前会话 Agent 切换流程

```mermaid
sequenceDiagram
    participant SD as SessionDispatcher
    participant CP as SlashCommandParser
    participant STORE as ConversationExecutionStateStore
    participant EB as EventBus

    SD->>CP: parse("/use reviewer")
    CP-->>SD: SwitchAgent(reviewer)
    SD->>STORE: set_agent(conversation, reviewer)
    STORE-->>SD: ConversationExecutionState
    SD->>EB: publish(AgentSelected)
```

### 实体关系

```mermaid
erDiagram
    Agent ||--|| Identity : "默认拥有"
    Agent }o--o{ Skill : "配置可用技能"
    AcpServer ||--o{ Agent : "作为默认执行后端"
    Agent ||--o{ SlashCommand : "作为命令目标"
    Skill ||--o{ SlashCommand : "作为命令技能"
    ConversationExecutionState }o--|| Agent : "当前选择"
    ConversationExecutionState }o--o| Skill : "当前技能"
    ConversationExecutionState }o--o| AcpServer : "临时后端覆盖"
```

## § 设计决策与权衡

| 决策问题 | 选择 | 放弃的替代方案 | 理由 |
|---------|------|--------------|------|
| 用户选择的主对象是什么？ | Agent | ACP Server 或 ExecutionRole | Agent 更符合“谁来执行”的用户心智，ACP Server 是执行后端 |
| Identity 如何建模？ | Agent 默认拥有 Identity | Identity 独立复用为主路径 | Agent 1:1 Identity 降低首版配置复杂度 |
| Skill 如何建模？ | Skill 独立管理，Agent-Skill 多对多 | Skill 内嵌在 Agent 中 | 独立 Skill 可跨 Agent 复用，减少重复配置 |
| SlashCommand 是否使用 RouteRule？ | 不使用 RouteRule | 复用 RouteRule 匹配逻辑 | SlashCommand 是显式命令，RouteRule 是自动规则路由 |
| SlashCommand 解析在哪里？ | 后端统一解析 | 前端解析后传结构化命令 | 后端解析保证 Desktop UI、CLI、Webhook 语义一致 |
| 会话切换状态如何保存？ | ConversationExecutionState 按 conversation 隔离 | 写入 ChannelMessage | 执行状态是会话状态，不属于消息内容 |
