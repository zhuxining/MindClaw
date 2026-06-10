> **Status**: `draft`

# 功能需求规格：App 内驻留 GatewaySupervisor 与默认 Agent v1.0

## § 背景与目标

**背景**：MindClaw 需要在桌面窗口关闭到托盘后继续接收渠道消息，并交由本地 Agent 处理。当前功能应从“桌面端主动拉取消息”升级为“App 内驻留 GatewaySupervisor 托管渠道 inbound drivers，并将消息发送到默认 Agent 绑定的 ACP Server”。

**目标**：实现 GatewaySupervisor、ChannelManager、SessionDispatcher、EventBus、agent_context、acp_client、默认 Agent 和 Desktop UI 控制台的完整闭环，让用户在 Tauri App 进程保持运行时持续接收、处理和回写渠道消息。

核心数据流：**`GatewaySupervisor → ChannelManager → SessionDispatcher → 默认 Agent → agent_context → acp_client → Agent 绑定的 ACP Server`**

## § 功能描述

### Story 1：启动 App 内驻留 GatewaySupervisor

**作为** MindClaw 用户，**我希望** MindClaw 启动运行在 Tauri App 进程内的 GatewaySupervisor，**以便** 桌面窗口关闭到托盘后消息渠道仍能持续连接。

**优先级**：P0

**验收标准**：

- [ ] 用户启动 MindClaw 后，GatewaySupervisor 进入运行状态。
- [ ] 用户关闭桌面窗口到托盘后，Tauri App 进程保持运行。
- [ ] 用户关闭桌面窗口到托盘后，GatewaySupervisor 保持运行状态。
- [ ] 用户显式退出 MindClaw 后，GatewaySupervisor 停止运行。
- [ ] GatewaySupervisor 不创建独立 OS daemon。
- [ ] GatewaySupervisor 不创建独立 `tokio::runtime::Runtime`。
- [ ] Desktop UI 能显示 GatewaySupervisor 的运行状态。
- [ ] GatewaySupervisor 异常停止后，Desktop UI 显示错误状态。

---

### Story 2：配置渠道连接

**作为** MindClaw 用户，**我希望** 在设置中配置渠道凭证，**以便** GatewaySupervisor 能够连接我的消息渠道获取消息。

**优先级**：P0

**验收标准**：

- [ ] 用户在设置页面输入渠道所需凭证。
- [ ] 密钥保存到 Stronghold 安全存储中。
- [ ] 点击“测试连接”后，GatewaySupervisor 验证渠道连接有效性。
- [ ] 验证成功显示“连接成功”提示，失败显示具体错误原因。
- [ ] 重新打开应用后，GatewaySupervisor 能读取已保存凭证。

---

### Story 3：配置默认 Agent

**作为** MindClaw 用户，**我希望** 配置一个默认 Agent，**以便** 自动进入且没有 slash command 的渠道消息由该 Agent 处理。

**优先级**：P0

**验收标准**：

- [ ] 用户可在设置中查看默认 Agent。
- [ ] 默认 Agent 拥有默认 Identity。
- [ ] 默认 Agent 绑定一个 ACP Server。
- [ ] GatewaySupervisor 持久化默认 Agent。
- [ ] GatewaySupervisor 启动后恢复默认 Agent。
- [ ] 默认 Agent 绑定的 ACP Server 不可用时，GatewaySupervisor 记录错误状态并停止自动处理新消息。
- [ ] Desktop UI 显示默认 Agent 和其绑定 ACP Server 的连接状态。

---

### Story 4：按渠道启动 inbound driver

**作为** MindClaw 用户，**我希望** GatewaySupervisor 为每个启用渠道启动合适的接收方式，**以便** 不同入口都能进入统一消息处理链路。

**优先级**：P0

**验收标准**：

- [ ] GatewaySupervisor 启动已启用渠道的 inbound driver。
- [ ] FeishuChannel 使用 polling 接收消息。
- [ ] TelegramChannel 使用 long polling 接收消息。
- [ ] EmailChannel 使用 IMAP IDLE 或 polling 接收消息。
- [ ] MCP Event 使用 stream 或 local connection 接收事件。
- [ ] CLI Input 使用 local API 或手动输入注入消息。
- [ ] 桌面窗口关闭到托盘后，已启动的 inbound driver 继续运行。
- [ ] 用户显式退出 MindClaw 后，已启动的 inbound driver 停止运行。
- [ ] 连续接收失败时，GatewaySupervisor 记录渠道错误状态。

---

### Story 5：渠道消息统一为 ChannelMessage

**作为** MindClaw 用户，**我希望** 渠道消息被统一为系统可处理的消息格式，**以便** 后续消息展示和 ACP Server 处理逻辑不依赖具体渠道协议。

**优先级**：P0

**验收标准**：

- [ ] Channel 注册到 ChannelManager。
- [ ] Channel 将外部原始消息转换为标准 `ChannelMessage`。
- [ ] 转换后的消息包含 `message_id`、`channel`、`conversation_id`、`sender_id`、`sender_name`、`content`、`timestamp`。
- [ ] 消息进入 SessionDispatcher 前基于 `message_id` 去重。
- [ ] SessionDispatcher 只接收 `ChannelMessage`，不接收渠道原始响应。

---

### Story 6：查看 GatewaySupervisor 消息流与事件

**作为** MindClaw 用户，**我希望** Desktop UI 连接 GatewaySupervisor 查看消息流和运行时事件，**以便** 了解后台处理了哪些消息。

**优先级**：P1

**验收标准**：

- [ ] Desktop UI 通过 Gateway API adapter 获取最近消息列表。
- [ ] 消息以时间线方式展示，最新消息显示在顶部。
- [ ] 每条消息显示发送者名称、消息内容、来源渠道、时间戳、处理状态。
- [ ] GatewaySupervisor 收到新消息后，Desktop UI 自动追加到消息列表。
- [ ] EventBus 发布 message received、dispatch started、dispatch completed、reply sent 和 error 事件。
- [ ] Desktop UI 重新打开后能恢复显示最近消息。

---

### Story 7：消息按 session 发送到默认 Agent

**作为** MindClaw 用户，**我希望** 渠道消息按会话顺序自动发送给默认 Agent，**以便** 同一对话不会出现乱序处理。

**优先级**：P0

**验收标准**：

- Given GatewaySupervisor 运行、渠道连接已配置且默认 Agent 可用，When 新消息进入 SessionDispatcher，Then SessionDispatcher 将消息发送到默认 Agent 绑定的 ACP Server。
- [ ] 消息到达后自动触发默认 Agent 处理，无需用户手动操作。
- [ ] 同一 `channel + conversation_id` 内的消息按进入顺序处理。
- [ ] 不同 `channel + conversation_id` 的消息可以并发处理。
- [ ] Agent 处理期间消息状态为“处理中”。
- [ ] Agent 处理完成后结果关联到对应消息。
- [ ] 处理失败时记录错误信息，Desktop UI 可展示错误状态。
- [ ] 系统不要求用户配置 RouteRule 或关键词规则。

---

### Story 8：ACP Server 执行

**作为** MindClaw 用户，**我希望** GatewaySupervisor 通过 `acp_client` 调用默认 Agent 绑定的 ACP Server 并返回结果，**以便** 获得智能处理输出。

**优先级**：P0

**验收标准**：

- [ ] `agent_context` 组装上下文：Agent 身份、会话记忆、可用工具元数据和用户消息。
- [ ] `acp_client` 向默认 Agent 绑定的 ACP Server 发送请求。
- [ ] 默认 Agent 返回响应，`acp_client` 解析为 `AgentResponse`。
- [ ] 若默认 Agent 发起 Tool Call 请求，`acp_client::ToolExecutor` 执行本地工具并返回结果。
- [ ] `acp_client` 返回 `AgentResponse`，包含处理状态、输出内容和执行元数据。
- [ ] 处理超时时返回超时错误并支持取消。

---

### Story 9：Agent 处理结果回写原渠道

**作为** MindClaw 用户，**我希望** 默认 Agent 处理完消息后自动将结果回复到原渠道会话中，**以便** 对话者能看到 Agent 的回复。

**优先级**：P1

**验收标准**：

- Given 默认 Agent 处理完成并产生回复内容，When 回复内容非空，Then ChannelManager 找到原 Channel 并发送回复到原始会话。
- [ ] 回复消息包含原始消息的引用。
- [ ] 回复失败时 GatewaySupervisor 记录错误，Desktop UI 可展示错误状态。
- [ ] 用户可在设置中关闭自动回复开关。

## § 范围界定

### In Scope

- Tauri App 内驻留的 GatewaySupervisor 运行与启停控制。
- 窗口关闭到托盘后继续运行，显式退出应用后停止。
- Desktop UI 作为 GatewaySupervisor 控制台。
- Gateway API adapter：运行时状态、消息流、配置读写、默认 Agent 选择。
- ChannelManager：渠道启动、停止、健康状态、inbound driver 管理和出站分发。
- FeishuChannel：polling 接入、消息转换、回复发送。
- TelegramChannel：long polling 接入边界。
- CLI Input：local API 或手动输入注入边界。
- 渠道消息转换为 `ChannelMessage`。
- SessionDispatcher：去重、按 session 保序、ACP 调用编排。
- EventBus：运行时事件订阅。
- 默认 Agent 选择与持久化。
- `agent_context` Agent 上下文组装。
- `acp_client` ACP 协议客户端。
- 密钥 Stronghold 安全存储。

### Out of Scope

| 排除项 | 理由 |
|--------|------|
| 独立 OS daemon / sidecar / system service | v1 采用 App 内驻留，降低安装、更新和本地 IPC 成本 |
| Tauri App 完全退出后继续处理消息 | 该能力需要独立 daemon 或 sidecar，不属于 v1 后台模型 |
| 设备关机或系统休眠后继续处理消息 | 本地运行时依赖设备处于可执行状态 |
| 公网 SaaS webhook relay | 公网 webhook 需要 HTTPS endpoint、tunnel 或 Cloud Relay，不由 App 内驻留单独解决 |
| WhatsApp 正式接入 | WhatsApp 依赖公网 webhook 或平台服务，超出 v1 本地闭环 |
| 图片/文件等富媒体消息 | v1 聚焦文本消息，富媒体需要独立的内容模型和渲染路径 |
| RouteRule、多 Agent 路由和关键词分发 | v1 只使用默认 Agent 绑定的 ACP Server，降低配置与调试复杂度 |
| 多 Agent 并行处理 | v1 使用单个默认 Agent 处理自动消息 |
| Agent 本身的实现 | 架构边界：`acp_client` 只做协议通信，`agent_context` 只做上下文组装 |
| 多 Transport 支持 | v1 使用现有 ACP 调用路径验证本地 ACP Server 调用闭环 |
| 向量检索记忆 | v1 使用短期会话记忆验证上下文注入闭环 |

## § 非功能需求

| 类别 | 约束 | 量化阈值 |
|------|------|---------|
| 性能 | 单次渠道轮询响应时间 | ≤ 5 秒 |
| 性能 | 渠道消息转换 | 单条消息 ≤ 100ms |
| 性能 | ACP Server 调用超时 | 默认 120 秒，可配置 |
| 可用性 | App 内驻留 | 窗口关闭到托盘后 GatewaySupervisor 保持运行 |
| 可用性 | 显式退出 | 用户显式退出后 5 秒内停止 inbound drivers |
| 可观测性 | 运行状态刷新 | Desktop UI 中运行状态刷新延迟 ≤ 2 秒 |
| 安全 | 密钥存储 | 必须使用 Stronghold，不得明文存储 |
| 安全 | 网络请求范围 | Channel 和 Gateway API adapter 可接入外部网络 |
| 安全 | 本地 API 访问 | Gateway API 必须校验本地访问 token 或系统权限 |
| 安全 | 自动消息处理目标 | 只有默认 Agent 绑定的 ACP Server 接收自动渠道消息 |
| 安全 | 工具执行 | `acp_client::ToolExecutor` 拒绝 `vault/private/` 路径，Terminal 禁止危险命令 |
| 可靠性 | 渠道接收失败恢复 | 连续失败 3 次后进入退避状态，最大间隔 300 秒 |
| 可靠性 | 消息不重复处理 | 同一 `message_id` 只触发一次 ACP 调用 |
| 可靠性 | 会话内顺序 | 同一 `channel + conversation_id` 内消息按进入顺序处理 |
| 兼容性 | 平台支持 | macOS（v1），Windows/Linux（v1.1） |
