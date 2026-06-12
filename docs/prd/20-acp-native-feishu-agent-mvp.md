> **Status**: `active`
> **Blueprint**: `docs/blueprint/00-overview.md`

# PRD: ACP-native Feishu Agent MVP

## 1. Objective

### Problem

用户已经有或愿意使用本地 ACP Server 作为 Agent 执行后端，但缺少一个本地控制平面把 Agent 角色、Skill、会话状态和真实 IM 消息流连接起来。如果 MindClaw 第一阶段同时追求多渠道、完整 Agent 管理和完整 Gateway API adapter，会拖慢内测验证。

### Target Users

- 已经使用 Claude Code、Gemini CLI、自研 Agent 或其他 ACP Server 的开发者。
- 希望让自己的 Agent 处理 Feishu 消息的重度 LLM 用户。
- 愿意参与早期内测、接受有限配置和有限渠道范围的高级用户。

### Desired Outcome

用户能在 MindClaw 中配置一个默认 ACP Server、一个默认 Agent、一个默认 Skill 和一个 Feishu 渠道。Feishu 文本消息进入 MindClaw 后，系统按会话顺序调用默认 Agent 绑定的 ACP Server，生成摘要、待办或建议回复；默认不自动发送真实 IM 回复，用户确认后才回写。

## 2. Success Criteria

- [ ] 首次内测用户能在 10 分钟内完成 ACP Server、默认 Agent、默认 Skill 和 Feishu 渠道配置。
- [ ] Feishu 文本消息能进入 MindClaw，并转换为 `ChannelMessage`。
- [ ] 同一 `channel + conversation_id` 内消息按进入顺序处理。
- [ ] 默认 Agent 能通过 ACP Server 返回处理结果。
- [ ] 处理结果包含 agent_id、skill_id、acp_server_id、status、error 信息。
- [ ] Agent 输出能形成可用的摘要、待办或建议回复。
- [ ] 默认不自动发送真实 IM 回复。
- [ ] 用户确认后可把建议回复发送回原 Feishu 会话。
- [ ] ACP Server、Feishu、Agent 执行失败均有可见错误。

## 3. Scope

### In Scope

- 一个默认 ACP Server 的注册、测试连接和默认绑定。
- 一个默认 Agent 的创建、编辑、启用状态和 Identity。
- 一个默认 Skill 或少量预设 Skill。
- Feishu 文本消息接入。
- Feishu 原始消息转换为 `ChannelMessage`。
- SessionDispatcher 按 `channel + conversation_id` 保序处理。
- AgentResolver 选择默认 Agent / Skill。
- agent_context 组装 Agent Identity、Skill instruction 和用户消息。
- acp_client 调用默认 Agent 绑定的 ACP Server。
- Agent 输出建议回复。
- 用户确认后发送 Feishu 回复。
- 测试会话中的受限自动回复。
- 基础执行元数据展示：Agent、Skill、ACP Server、状态、错误。
- 窗口关闭到托盘后的 App 内驻留。
- Tauri commands 作为 MVP UI 与后端交互入口。

### Out of Scope

- 自研基础 Agent Server：MindClaw 是 ACP-native 控制平面，不重复实现 Agent runtime。
- 多渠道平台：MVP 只用 Feishu 验证价值链。
- 完整 Gateway API adapter：MVP 使用 Tauri commands，Gateway API adapter 进入后续架构阶段。
- 完整多 Agent 管理 UI：MVP 聚焦默认 Agent。
- 复杂 Agent-Skill 多对多配置 UI：后端能力可保留，但不是 MVP 内测入口重点。
- Skill 参数 schema：参数化会增加 UI 与校验复杂度，后续再做。
- 自动 Agent 路由 / RouteRule：显式选择优先，避免不可解释路由。
- 多 Agent 并行处理同一消息：结果合并和冲突解释超出 MVP。
- 默认自动发送真实 IM 回复：误发会破坏信任。
- 富媒体消息：MVP 仅验证文本消息闭环。
- 公网 webhook relay：与 Feishu polling MVP 无关。
- 独立 daemon / sidecar：v1 采用 Tauri App 内驻留。
- 团队协作 / 多用户权限：MVP 面向个人开发者和重度 LLM 用户内测。

## 4. User Stories

### Story 1: Register Default ACP Server

**As a** 开发者用户，**I want** 注册一个本地 ACP Server，**so that** MindClaw 能复用我已有的 Agent 执行后端。

**Priority**: P0

**Acceptance Criteria**:

- [ ] 用户可新增一个 ACP Server 配置。
- [ ] ACP Server 配置至少包含名称、启动方式或连接配置、说明。
- [ ] 用户可执行“测试连接”。
- [ ] 测试成功时，ACP Server 状态显示为可用。
- [ ] 测试失败时，展示可理解的错误原因。
- [ ] ACP Server secrets 必须进入安全存储，不得明文写入普通配置文件。
- [ ] MVP 只要求一个默认 ACP Server 可用。

### Story 2: Create Zero-config Default Agent

**As a** MindClaw 用户，**I want** 用最少配置创建默认 Agent，**so that** Feishu 消息拥有明确执行者。

**Priority**: P0

**Acceptance Criteria**:

- [ ] 首次启动或首次配置时，系统提供默认 Agent 模板。
- [ ] 默认 Agent 必须绑定一个可用 ACP Server。
- [ ] 默认 Agent 至少包含名称、描述、Identity / system instruction。
- [ ] 用户可编辑默认 Agent 的 Identity。
- [ ] 用户可恢复默认 Agent 模板。
- [ ] 默认 Agent 不可用时，SessionDispatcher 不调用 ACP Server，并记录错误状态。
- [ ] MVP 不要求完整多 Agent 管理 UI。

### Story 3: Configure Default Skill

**As a** MindClaw 用户，**I want** 使用默认消息助理 Skill，**so that** 默认 Agent 能对 Feishu 消息生成摘要、待办和建议回复。

**Priority**: P0

**Acceptance Criteria**:

- [ ] MVP 至少提供一个默认 Skill：消息助理。
- [ ] 默认 Skill instruction 覆盖摘要、待办提取、建议回复三个输出方向。
- [ ] 用户可查看默认 Skill instruction。
- [ ] 用户可编辑默认 Skill instruction。
- [ ] 默认 Agent 可绑定默认 Skill。
- [ ] Agent context 组装时包含 Agent Identity、当前 Skill instruction 和用户消息。
- [ ] MVP 不要求 Skill 参数 schema。

### Story 4: Configure Feishu Channel

**As a** MindClaw 用户，**I want** 配置 Feishu 渠道凭证，**so that** MindClaw 能接收 Feishu 文本消息。

**Priority**: P0

**Acceptance Criteria**:

- [ ] 用户可输入 Feishu 所需凭证。
- [ ] Feishu secrets 必须进入 Stronghold 或等价安全存储，不得明文保存。
- [ ] 用户可执行“测试连接”。
- [ ] 测试成功时显示连接成功。
- [ ] 测试失败时展示具体错误原因。
- [ ] 系统至少支持 Feishu 文本消息。
- [ ] MVP 不要求支持图片、文件、卡片等富媒体消息。
- [ ] MVP 不要求公网 webhook relay。

### Story 5: Convert Feishu Message to ChannelMessage

**As a** MindClaw 用户，**I want** Feishu 原始消息被转换为统一消息格式，**so that** 后续调度逻辑不依赖 Feishu 协议细节。

**Priority**: P0

**Acceptance Criteria**:

- [ ] Feishu 文本消息转换为 `ChannelMessage`。
- [ ] `ChannelMessage` 至少包含 `message_id`、`channel`、`conversation_id`、`sender_id`、`sender_name`、`content`、`timestamp`。
- [ ] 转换失败时记录错误，不调用 ACP Server。
- [ ] 同一 `message_id` 不重复触发 Agent 执行。
- [ ] 消息进入 SessionDispatcher 前不暴露 Feishu 原始响应给 Agent 调度层。

### Story 6: Dispatch Message to Default Agent

**As a** MindClaw 用户，**I want** Feishu 消息按会话顺序发送给默认 Agent，**so that** 同一对话不会乱序处理。

**Priority**: P0

**Acceptance Criteria**:

- [ ] Feishu 渠道、默认 Agent、默认 ACP Server 可用时，新消息自动进入 SessionDispatcher。
- [ ] 同一 `channel + conversation_id` 内的消息按进入顺序处理。
- [ ] 不同 `channel + conversation_id` 的消息可以并发处理。
- [ ] Agent 处理期间消息状态为“处理中”。
- [ ] Agent 处理完成后结果关联到对应消息。
- [ ] 处理失败时记录错误信息，并可在 UI 中展示。
- [ ] 无 slash command 的消息使用默认 Agent。
- [ ] MVP 不经过 legacy RouteRule、关键词路由或自动 Agent 路由。

### Story 7: Execute Through ACP Server

**As a** MindClaw 用户，**I want** MindClaw 通过 ACP 调用默认 Agent 绑定的 ACP Server，**so that** 复用我已有的 Agent 执行能力。

**Priority**: P0

**Acceptance Criteria**:

- [ ] agent_context 组装 Agent Identity、Skill instruction 和用户消息。
- [ ] acp_client 将组装后的请求发送给默认 Agent 绑定的 ACP Server。
- [ ] ACP Server 响应被解析为 `AgentResponse` 或等价内部结果类型。
- [ ] 响应至少包含处理状态、输出内容和错误信息。
- [ ] 结果记录 agent_id、skill_id、acp_server_id。
- [ ] ACP Server 调用超时时返回可展示错误。
- [ ] ACP Server 不可用时，不丢失原始消息记录。

### Story 8: Confirm Suggested Reply

**As a** MindClaw 用户，**I want** Agent 默认生成建议回复而不是直接发送，**so that** 真实 IM 场景中不会因为误发破坏信任。

**Priority**: P0

**Acceptance Criteria**:

- [ ] Agent 输出默认以“建议回复”形式展示。
- [ ] 用户可查看原始 Feishu 消息、Agent 输出和执行元数据。
- [ ] 用户可点击确认后将建议回复发送回原 Feishu 会话。
- [ ] 发送成功后消息状态更新为“已发送”。
- [ ] 发送失败后展示错误原因，并允许用户重试。
- [ ] 默认配置下不自动发送真实 IM 回复。
- [ ] 开发者可为测试会话开启受限自动回复。
- [ ] 受限自动回复必须明确标记，不能静默开启。

### Story 9: Show MVP Console Status

**As a** MindClaw 用户，**I want** 在 Desktop UI 中看到 ACP Server、默认 Agent、Feishu 渠道和消息处理状态，**so that** 我能理解系统是否正在工作以及失败发生在哪里。

**Priority**: P1

**Acceptance Criteria**:

- [ ] UI 显示默认 ACP Server 状态。
- [ ] UI 显示默认 Agent 和当前 Skill。
- [ ] UI 显示 Feishu 渠道连接状态。
- [ ] UI 显示最近消息列表。
- [ ] 每条消息显示来源渠道、发送者、内容、处理状态、Agent 输出。
- [ ] 每条已处理消息显示 agent_id、skill_id、acp_server_id。
- [ ] 处理失败的消息展示错误原因。
- [ ] MVP 可通过 Tauri commands 获取状态；不要求先完成 Gateway API adapter。

### Story 10: Stay Resident in Tray

**As a** MindClaw 用户，**I want** 关闭窗口后 MindClaw 仍在本机运行，**so that** Feishu 消息可以继续进入本地处理链路。

**Priority**: P1

**Acceptance Criteria**:

- [ ] 用户关闭桌面窗口到托盘后，Tauri App 进程保持运行。
- [ ] 用户关闭窗口后，已启用 Feishu 渠道继续按 MVP 配置接收消息。
- [ ] 用户显式退出 MindClaw 后，消息接收停止。
- [ ] 系统不创建独立 OS daemon。
- [ ] 系统不承诺设备休眠、关机或 App 完全退出后继续处理消息。

## 5. Non-functional Requirements

- **Security**：Feishu 和 ACP Server secrets 不得明文存储；MindClaw 自身不上传消息到 MindClaw 云端。
- **Reliability**：同一 `message_id` 只触发一次 Agent 执行；同一 `channel + conversation_id` 内消息按进入顺序处理。
- **Safety**：默认不自动发送真实 IM 回复；受限自动回复必须明确标记，不能静默开启。
- **Observability**：ACP Server、Feishu、Agent 执行失败必须可见；每次处理需关联 agent_id、skill_id、acp_server_id。
- **Performance**：首次配置目标 ≤ 10 分钟；ACP Server 调用超时默认 120 秒，可配置。
- **Compatibility**：MVP 优先 macOS。

## 6. Open Questions

- [ ] Feishu MVP 的接收方式是否只采用 polling，还是需要明确支持事件订阅前置接口？
- [ ] 建议回复的数据结构是直接使用 `AgentResponse.output`，还是引入 `SuggestedReply` 实体？
- [ ] 受限自动回复的最小安全边界是什么：按会话、按发送者、按测试开关，还是三者都需要？
- [ ] 执行元数据应扩展在 `AgentResponse`、`DispatchResult`，还是建立 `MessageExecution` 记录？
- [ ] 默认 Skill instruction 是否需要内置模板，还是首次配置时由用户输入？

## 7. Related Docs

- `docs/blueprint/00-overview.md`
- `docs/prd/README.md`
- `docs/prd/00-overview.md`
- `docs/prd/10-agent-skill-slash-command.md`
- `docs/architecture/00-overview.md`
- `docs/architecture/10-channel-gateway.md`
- `docs/architecture/30-acp-client.md`
- `docs/architecture/35-agent-context.md`
- `docs/architecture/40-agent-skill-command.md`
- `docs/architecture/reference/migration.md`
- `docs/architecture/reference/traceability.md`
