> **Status**: `draft`

# 产品蓝图：App 内驻留消息网关、Agent 与 ACP Server 调度

## Part 1：产品定位

- **目标用户**：重度 LLM 用户（开发者、研究员、知识工作者），在 IM、Email、Webhook、MCP Event 和 CLI 场景中接收大量信息，需要本地 Agent 在无人值守状态下持续处理消息。
- **核心问题**：消息渠道分散在不同入口，敏感上下文不适合交给云端 AI 服务，桌面端、CLI、Web UI、移动伴随端缺少统一的本地 Agent 调度入口。
- **核心方式**：MindClaw 提供运行在 Tauri App 进程内的 GatewaySupervisor，托管渠道 inbound drivers、SessionDispatcher、EventBus、Agent 执行模型、ACP Server 调用链路和本地控制入口。
- **长期价值**：用户以 MindClaw 为本地 AI 消息中枢；当桌面窗口关闭到托盘时，已启用渠道仍持续连接到默认 Agent，用户也能通过 `/命令` 显式选择 Agent 和 Skill 完成特定任务。

## Part 2：核心场景

### 场景 1：窗口关闭到托盘后的消息处理

用户关闭 MindClaw 桌面窗口到托盘后，Tauri App 进程保持运行，GatewaySupervisor 继续在进程内处理已启用渠道。渠道 inbound driver 产生 `ChannelMessage`，SessionDispatcher 按会话顺序发送给默认 Agent，处理结果通过原渠道回写。

**数据流**：Channel inbound driver → ChannelManager → SessionDispatcher → AgentResolver → agent_context → acp_client → Agent 绑定的 ACP Server → SessionDispatcher → ChannelManager → Channel reply

### 场景 2：不同渠道使用不同接收方式

用户启用多个入口后，GatewaySupervisor 为每个渠道启动对应 inbound driver：Feishu 使用 polling，Telegram 使用 long polling，Email 使用 IMAP IDLE 或 polling，MCP Event 使用 stream，CLI Input 使用本地输入或 Local API。

### 场景 3：管理 Agent、Skill 与 ACP Server

用户在设置中管理多个 Agent。每个 Agent 默认拥有自己的 Identity，绑定一个默认 ACP Server，并关联一组可用 Skill。Skill 独立管理，同一个 Skill 能被多个 Agent 复用。

### 场景 4：通过 `/命令` 显式选择 Agent 或 Skill

用户在对话中输入 `/review`、`/reply` 或 `/research`，系统根据命令选择目标 Agent 和 Skill。该选择来自用户显式输入，不读取 legacy RouteRule，也不根据消息关键词自动切换 Agent。

### 场景 5：Desktop UI 作为控制台查看状态

用户重新打开 MindClaw 桌面端，Desktop UI 连接 Gateway API adapter，查看 GatewaySupervisor、渠道连接、inbound drivers、消息处理、EventBus 事件、当前会话 Agent、Skill 和 ACP Server 状态。Desktop UI 不直接轮询渠道，也不直接调用 ACP Server。

### 场景 6：Webhook 与公网入口边界

本地 webhook 和 CLI Input 通过本机 Local API 进入 GatewaySupervisor。外部 SaaS webhook 需要公网 HTTPS endpoint、tunnel 或 Cloud Relay；独立 daemon 只解决进程常驻，不解决公网可达。

## Part 3：产品原则

1. **App 内驻留优先**：v1 让 GatewaySupervisor 运行在 Tauri App 进程内，窗口关闭到托盘后继续处理消息。理由是该模型满足个人桌面后台场景，并避免独立 daemon 的安装、更新和权限成本。
2. **默认 Agent 优先**：无 `/命令` 的自动消息发送给默认 Agent。理由是 v1 保持单一默认执行目标，降低配置和调试成本。
3. **显式选择优先于自动路由**：用户通过 `/命令` 切换 Agent 或 Skill。理由是显式选择比关键词路由更可解释，不引入 RouteRule 优先级和冲突。
4. **Skill 独立复用**：Skill 独立管理，Agent 与 Skill 多对多关联。理由是任务能力需要跨 Agent 复用，Agent 负责组合身份、技能和执行后端。
5. **本地优先**：所有消息处理和 ACP Server 调用均在本地完成，不上传 MindClaw 云端。理由是用户的消息和上下文具有隐私敏感性。

## Part 4：价值边界

### 明确解决

- 桌面窗口关闭到托盘后，已启用渠道仍能进入 GatewaySupervisor。
- GatewaySupervisor 统一托管渠道 inbound drivers、SessionDispatcher、EventBus、Agent 执行模型、ACP Server 调用链路和本地控制入口。
- 用户可管理多个 Agent，每个 Agent 默认拥有自己的 Identity。
- 用户可独立管理多个 Skill，并将 Skill 关联到多个 Agent。
- 用户可将 Agent 绑定到 ACP Server 作为默认执行后端。
- 用户可通过 `/命令` 显式选择 Agent 或 Agent + Skill 执行当前消息。
- 用户可将当前会话切换到指定 Agent，再用 `/default` 恢复默认 Agent。
- 渠道消息按 `channel + conversation_id` 保持会话内顺序，不同会话并发处理。
- Agent 上下文组装（Identity、Skill instruction、记忆、工具列表注入 prompt）。
- 本地 ACP Server 通过 ACP 协议标准化调用。
- 安全控制：凭证安全存储、本地 API 鉴权、Webhook 鉴权、本地数据隔离。

### 明确不解决

- 不提供 MindClaw 云端消息处理服务。
- 不承诺 Tauri App 完全退出后继续接收消息。
- 不在 v1 提供独立 OS daemon、sidecar 或系统服务。
- 不把独立 daemon 作为公网 webhook 的解决方案。
- 不提供 Feishu、WhatsApp、Telegram 等外部 SaaS webhook 的公网 relay，除非用户自建 endpoint 或启用 Cloud Relay。
- 不做 RouteRule、多 Agent 自动路由、关键词分发或负载均衡。
- 不做 RouteRule 与 SlashCommand 的混合优先级。
- 不做多个 Agent 并行处理同一消息。
- 不做团队协作和多用户权限系统。
- 不实现 ACP Server 内部 Agent 智能。
- 不承诺桌面设备休眠或关机后继续处理消息。

## Part 5：演进方向

### v1.0 — App 内驻留的本地消息闭环

- GatewaySupervisor：App 内驻留、启停控制、health 状态。
- Gateway API adapter：Desktop UI 连接、运行时状态、消息流订阅、默认 Agent 选择。
- ChannelManager：渠道生命周期、inbound drivers 和出站分发。
- FeishuChannel：polling 接入、消息转换、回复发送。
- SessionDispatcher：按 session 保序、并发处理、ACP 调用编排。
- EventBus：运行时事件订阅。
- agent_context：默认 Agent 的 Identity、记忆注入、Prompt 组装。
- acp_client：ACP 协议客户端，调用默认 Agent 绑定的 ACP Server。

### v1.1 — Agent / Skill / SlashCommand 显式选择

- ACP Server 管理：注册、编辑、测试连接、状态展示。
- Agent 管理：名称、描述、默认 Identity、默认 ACP Server。
- Skill 管理：名称、描述、instruction、输出约束。
- Agent-Skill 多对多关联。
- 默认 Agent 设置。
- SlashCommand：`/agent`、`/skill`、`/use`、`/default`、`/help`。
- ConversationExecutionState：按会话保存当前 Agent 和 Skill。
- 当前消息展示 Agent、Skill 和 ACP Server 执行元数据。

### v1.2 — 多入口与本地控制增强

- Telegram long polling。
- Email IMAP IDLE / polling。
- CLI Input 通过 Local API 注入消息。
- MCP Event stream 接入。
- 本地 webhook handler 与签名校验。
- Skill 参数 schema。
- SlashCommand 参数 schema。
- 命令 palette 搜索。
- ConversationExecutionState 持久化。
- 托盘控制和开机启动。

### v2.0 — 公网入口与后台形态增强

- 用户自建 HTTPS webhook endpoint 配置。
- Cloud Relay 作为可选产品方向。
- 独立 daemon / sidecar 作为可选产品方向。
- WhatsApp / Slack / 钉钉 Channel。
- Mobile companion 接入。
- 自动 Agent 路由作为独立产品方向。
- 统一限流、渠道健康状态和富媒体能力声明。

## Part 6：关键假设

| 假设 | 验证方式 | 失效后果 |
|------|---------|---------|
| 用户接受“窗口关闭到托盘后继续运行，显式退出后停止”的后台模型 | 内测访谈与托盘原型 | 提前评估独立 daemon 或弱化后台处理承诺 |
| 用户接受默认 Agent 处理无命令消息 | 内测访谈与消息处理原型 | v1.1 提前引入会话级 Agent 选择 |
| 用户理解 Agent 与 Skill 的差异 | 原型可用性测试 | 合并 Skill 到 Agent 配置，延后独立 Skill 管理 |
| Agent 与 Skill 多对多能减少重复配置 | 配置原型测试 | 将 Skill 降级为 Agent 内部 instruction |
| App 内驻留在目标平台可稳定运行 | macOS 托盘与后台任务原型验证 | 降级为仅窗口打开时运行，或推进 daemon 设计 |
| 本地 ACP Server 处理延迟用户可接受 | 内测反馈 | 增加流式响应和进度反馈 |

## Part 7：非目标

- 不做 SaaS 化的云服务。
- 不做飞书机器人商店或应用市场。
- 不做团队协作功能（多用户、权限）。
- 不在设备关机或系统休眠时处理消息。
- 不把公网 webhook 作为 v1 默认能力。
- 不把 legacy RouteRule 作为 Agent 选择机制。
