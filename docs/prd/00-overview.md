> **Status**: `draft`

# 功能需求规格：本地常驻 Gateway Runtime 与 Active ACP Server v1.0

## § 背景与目标

**背景**：MindClaw 需要在桌面窗口最小化或关闭后继续接收飞书消息并交由本地 ACP Server 处理。当前功能应从"桌面端主动拉取消息"升级为"本地常驻 Gateway Runtime 托管消息渠道，并将消息直接发送到当前激活 ACP Server"。

**目标**：实现 Gateway Runtime、FeishuChannel、MessageBus、agent_context、acp_client 和 Desktop UI 控制台的完整闭环，让用户在桌面窗口关闭后仍能持续接收、处理和回写飞书消息。

核心数据流： **`Gateway Runtime → ChannelManager → MessageBus → agent_context → acp_client → Active ACP Server`**

## § 功能描述

### Story 1：启动 Gateway Runtime

**作为** MindClaw 用户，**我希望** MindClaw 启动本地常驻 Gateway Runtime，**以便** 桌面窗口关闭后消息渠道仍能持续连接。

**优先级**：P0

**验收标准**：

- [ ] 用户启动 MindClaw 后，Gateway Runtime 进入运行状态
- [ ] 用户关闭桌面窗口后，Gateway Runtime 保持运行状态
- [ ] 用户显式退出 MindClaw 后，Gateway Runtime 停止运行
- [ ] Desktop UI 能显示 Gateway Runtime 的运行状态
- [ ] Gateway Runtime 异常停止后，Desktop UI 显示错误状态

---

### Story 2：配置飞书连接

**作为** MindClaw 用户，**我希望** 在设置中配置飞书 App ID 和 Secret，**以便** Gateway Runtime 能够连接我的飞书账号获取消息。

**优先级**：P0

**验收标准**：

- [ ] 用户在设置页面输入飞书 App ID 和 App Secret
- [ ] 密钥保存到 Stronghold 安全存储中
- [ ] 点击"测试连接"后，Gateway Runtime 验证飞书连接有效性
- [ ] 验证成功显示"连接成功"提示，失败显示具体错误原因
- [ ] 重新打开应用后，Gateway Runtime 能读取已保存凭证

---

### Story 3：选择当前激活 ACP Server

**作为** MindClaw 用户，**我希望** 选择一个 ACP Server 作为当前激活服务，**以便** 自动进入的飞书消息直接由该服务处理。

**优先级**：P0

**验收标准**：

- [ ] 用户可在设置中查看已注册 ACP Server 列表
- [ ] 用户可选择一个 ACP Server 设为当前激活服务
- [ ] Gateway Runtime 持久化当前激活 ACP Server
- [ ] Gateway Runtime 启动后恢复上次选择的当前激活 ACP Server
- [ ] 当前激活 ACP Server 不可用时，Gateway Runtime 记录错误状态并停止自动处理新消息
- [ ] Desktop UI 显示当前激活 ACP Server 的连接状态

---

### Story 4：后台拉取飞书消息

**作为** MindClaw 用户，**我希望** Gateway Runtime 自动拉取飞书中的新消息，**以便** 我不需要保持桌面窗口打开。

**优先级**：P0

**验收标准**：

- [ ] Gateway Runtime 启动已启用的 FeishuChannel
- [ ] FeishuChannel 每隔可配置的时间间隔（默认 30 秒）自动轮询飞书新消息
- [ ] 桌面窗口关闭后，FeishuChannel 继续轮询飞书新消息
- [ ] 轮询间隔可在设置中调整（最小 10 秒，最大 300 秒）
- [ ] 已拉取的消息不会重复出现（基于 message_id 去重）
- [ ] 连续轮询失败时，Gateway Runtime 记录渠道错误状态

---

### Story 5：渠道消息统一为 ChannelMessage

**作为** MindClaw 用户，**我希望** 飞书消息被统一为系统可处理的消息格式，**以便** 后续消息展示和 ACP Server 处理逻辑不依赖飞书协议。

**优先级**：P0

**验收标准**：

- [ ] FeishuChannel 注册到 ChannelManager
- [ ] FeishuChannel 将飞书新消息转换为标准 `ChannelMessage`
- [ ] 转换后的消息包含 `message_id`、`channel`、`conversation_id`、`sender_id`、`sender_name`、`content`、`timestamp`
- [ ] 消息保存前基于 `message_id` 去重
- [ ] MessageBus 只接收 `ChannelMessage`，不接收飞书原始响应

---

### Story 6：查看 Gateway Runtime 消息流

**作为** MindClaw 用户，**我希望** Desktop UI 连接 Gateway Runtime 查看消息流，**以便** 了解后台处理了哪些消息。

**优先级**：P1

**验收标准**：

- [ ] Desktop UI 通过 Gateway API 获取最近消息列表
- [ ] 消息以时间线方式展示，最新的在顶部
- [ ] 每条消息显示：发送者名称、消息内容、来源渠道、时间戳、处理状态
- [ ] Gateway Runtime 收到新消息后，Desktop UI 自动追加到消息列表
- [ ] Desktop UI 重新打开后能恢复显示最近消息

---

### Story 7：消息直接发送到 Active ACP Server

**作为** MindClaw 用户，**我希望** 飞书消息自动发送给当前激活 ACP Server，**以便** 用当前选中的 Agent 服务处理所有自动消息。

**优先级**：P0

**验收标准**：

- Given Gateway Runtime 运行、飞书连接已配置且 Active ACP Server 可用，When 飞书新消息进入 MessageBus，Then MessageBus 将消息发送到 Active ACP Dispatch
- [ ] 消息到达后自动触发 Active ACP Server 处理，无需用户手动操作
- [ ] Agent 处理期间消息状态为"处理中"
- [ ] Agent 处理完成后结果关联到对应消息
- [ ] 处理失败时记录错误信息，Desktop UI 可展示错误状态
- [ ] 系统不要求用户配置 RouteRule 或关键词规则

---

### Story 8：ACP Server 执行

**作为** MindClaw 用户，**我希望** Gateway Runtime 通过 `acp_client` 调用当前激活 ACP Server 并返回结果，**以便** 获得智能处理输出。

**优先级**：P0

**验收标准**：

- [ ] `acp_client` 与当前激活 ACP Server 完成初始化握手（Initialization + Authentication）
- [ ] `acp_client` 创建并管理 ACP Session
- [ ] `agent_context` 组装完整上下文：Agent 身份证（Identity）→ system prompt、记忆（Memory）→ context、可用工具列表 → available_tools
- [ ] `acp_client` 发送 Prompt Turn 请求，携带组装后的完整 ACP 请求
- [ ] Active ACP Server 返回响应，`acp_client` 解析为 `AgentResponse`
- [ ] 若 Active ACP Server 发起 Tool Call 请求，`acp_client::ToolExecutor` 执行本地工具并返回结果
- [ ] `acp_client` 返回 `AgentResponse`，包含处理状态、输出内容、执行元数据
- [ ] 处理超时（默认 120s）时返回超时错误，支持取消

---

### Story 9：Agent 处理结果回写飞书

**作为** MindClaw 用户，**我希望** Active ACP Server 处理完消息后自动将结果回复到飞书会话中，**以便** 飞书中的对话者能看到 Agent 的回复。

**优先级**：P1

**验收标准**：

- Given Active ACP Server 处理完成并产生回复内容，When 回复内容非空，Then ChannelManager 找到 FeishuChannel 并发送回复到原始会话
- [ ] 回复消息包含原始消息的引用（回复模式）
- [ ] 回复失败时 Gateway Runtime 记录错误，Desktop UI 可展示错误状态
- [ ] 用户可在设置中关闭"自动回复飞书"开关

## § 范围界定

### In Scope

- Gateway Runtime 本地常驻运行与启停控制
- Desktop UI 作为 Gateway Runtime 控制台
- Gateway API：运行时状态、消息流、配置读写、Active ACP Server 选择
- FeishuChannel：飞书单渠道消息接入（轮询 + 发送）
- ChannelManager：渠道启动、停止、健康状态和出站分发
- 飞书消息转换为 `ChannelMessage`
- MessageBus 消息传递与事件订阅
- Active ACP Server 选择与持久化
- `agent_context` Agent 上下文组装（Identity + Memory + PromptBuilder + ToolRegistry）
- `acp_client` ACP 协议客户端（Transport + Session + Protocol + ToolExecutor）
- 密钥 Stronghold 安全存储

### Out of Scope

| 排除项 | 理由 |
|--------|------|
| 设备关机或系统休眠后继续处理消息 | 本地运行时依赖设备处于可执行状态 |
| 飞书以外的消息渠道 | v1 先验证飞书单渠道，降低接入和测试成本 |
| 图片/文件等富媒体消息 | v1 聚焦文本消息，富媒体需要独立的内容模型和渲染路径 |
| 外部 Webhook 正式接入 | v1 预留 Gateway API 边界，先用轮询验证闭环 |
| RouteRule、多 Agent 路由和关键词分发 | v1 只使用当前激活 ACP Server，降低配置与调试复杂度 |
| 多 Agent 并行处理 | v1 单 Active ACP Server 串行处理自动消息 |
| Agent 本身的实现 | 架构边界：`acp_client` 只做协议通信，`agent_context` 只做上下文组装 |
| 多 Transport 支持 | v1 使用 stdio 验证本地 ACP Server 调用闭环 |
| 向量检索记忆 | v1 使用短期会话记忆验证上下文注入闭环 |

## § 非功能需求

| 类别 | 约束 | 量化阈值 |
|------|------|---------|
| 性能 | 消息轮询响应时间 | 单次轮询 ≤ 5 秒 |
| 性能 | 渠道消息转换 | 单条消息 ≤ 100ms |
| 性能 | ACP Server 调用超时 | 可配置，默认 120 秒 |
| 可用性 | 后台运行 | 桌面窗口关闭后 Gateway Runtime 保持运行 |
| 可观测性 | 运行状态刷新 | Desktop UI 中运行状态刷新延迟 ≤ 2 秒 |
| 安全 | 密钥存储 | 必须使用 Stronghold，不得明文存储 |
| 安全 | 网络请求范围 | 仅 Channel 和 Gateway API 可接入外部网络 |
| 安全 | 本地 API 访问 | Gateway API 必须校验本地访问 token 或系统权限 |
| 安全 | 自动消息处理目标 | 只有当前激活 ACP Server 接收自动渠道消息 |
| 安全 | 工具执行 | `acp_client::ToolExecutor` 拒绝 `vault/private/` 路径，Terminal 禁止危险命令 |
| 可靠性 | 轮询失败恢复 | 连续失败 3 次后指数退避，最大间隔 300 秒 |
| 可靠性 | 消息不丢失 | MessageBus 保证至少一次投递 |
| 兼容性 | 平台支持 | macOS（v1），Windows/Linux（v1.1） |
