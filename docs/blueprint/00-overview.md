> **Status**: `draft`

# 产品蓝图：本地常驻消息网关与 ACP Server 调度

## Part 1：产品定位

- **目标用户**：重度 LLM 用户（开发者、研究员、知识工作者），在飞书等 IM 平台接收大量信息，需要本地 ACP Server 在无人值守状态下持续处理消息
- **核心问题**：IM 渠道消息依赖人工打开桌面应用才能处理；敏感数据不适合交给云端 AI 服务；桌面端、CLI、Web UI、移动伴随端缺少统一的本地 ACP Server 调度入口
- **核心方式**：MindClaw 提供本地常驻 Gateway Runtime，托管消息渠道、MessageBus、当前激活 ACP Server 调用链路和本地控制 API，让各类客户端只作为控制台接入同一个后台运行时
- **长期价值**：用户以 MindClaw 为本地 AI 消息中枢，即使桌面窗口最小化或关闭，消息渠道仍持续连接到当前激活 ACP Server，实现"消息入 → Gateway Runtime → Active ACP Server 处理 → 结果回写"的闭环，数据永不离开设备

## Part 2：核心场景

### 场景 1：桌面关闭后的飞书消息自动处理

用户关闭 MindClaw 桌面窗口后，Gateway Runtime 继续在本地后台运行。飞书新消息进入 FeishuChannel，MessageBus 直接交给当前激活 ACP Server，处理结果通过飞书渠道回写。

**数据流**：飞书 API/Webhook → Gateway Runtime → ChannelManager → FeishuChannel → MessageBus → agent_context → acp_client → Active ACP Server → MessageBus → FeishuChannel → 飞书回复

### 场景 2：选择当前激活 ACP Server

用户在 Desktop UI 中选择一个 ACP Server 作为当前激活服务。选择生效后，后续自动进入的飞书消息直接发送给该 ACP Server。

### 场景 3：Desktop UI 作为控制台查看状态

用户重新打开 MindClaw 桌面端，Desktop UI 连接 Gateway API，查看 Gateway Runtime、渠道连接、消息处理和当前激活 ACP Server 状态。Desktop UI 不直接轮询飞书，也不直接调用 Agent。

### 场景 4：外部 Webhook 接入

当用户启用飞书 Webhook 后，飞书事件进入 Gateway API，Gateway Runtime 校验来源并交给 FeishuChannel 转换为 `ChannelMessage`，再进入 MessageBus 和当前激活 ACP Server 调用链路。

## Part 3：产品原则

1. **后台优先**：消息接入与 ACP Server 调度归 Gateway Runtime，不归 Desktop UI。理由是消息处理必须在桌面窗口关闭后继续运行。
2. **单一激活目标**：自动进入的渠道消息只发送给当前激活 ACP Server。理由是 v1 不提供复杂路由规则，用户心智更简单。
3. **本地优先**：所有消息处理和 ACP Server 调用均在本地完成，不上传 MindClaw 云端。理由是用户的 IM 消息和上下文具有隐私敏感性。
4. **客户端轻量**：Desktop UI、CLI、Web UI、Mobile companion 都是 Gateway Runtime 的客户端。理由是多客户端应共享同一个渠道连接和运行时状态。
5. **渠道无关**：ChannelManager 管理具体 Channel，MessageBus 只消费 `ChannelMessage`。理由是 ACP Server 调用链路不应依赖飞书、Telegram 等协议细节。

## Part 4：价值边界

### 明确解决

- 桌面窗口最小化或关闭后，飞书消息仍能连接到 MindClaw 并进入当前激活 ACP Server
- Gateway Runtime 统一托管渠道连接、MessageBus、Active ACP Dispatch 和本地控制 API
- 用户可选择当前激活 ACP Server，自动消息直接发送到该服务
- Desktop UI、CLI、Web UI、Mobile companion 共享同一个本地运行时状态
- 飞书消息自动流入当前激活 ACP Server 处理并回写飞书
- Agent 上下文组装（Agent 身份证 + 记忆 + 工具列表注入 prompt）
- 本地 ACP Server 通过 ACP 协议标准化调用
- 安全控制：凭证安全存储、Webhook 鉴权、本地数据隔离

### 明确不解决

- 不提供 MindClaw 云端消息处理服务
- 不做飞书消息的云端同步/备份
- 不做多 Agent 路由规则、关键词分发或负载均衡
- 不做团队协作和多用户权限系统
- 不实现 Agent 本身
- 不承诺桌面设备休眠或关机后继续处理消息

## Part 5：演进方向

### v1.0 — 本地常驻飞书闭环

- Gateway Runtime：后台常驻、启停控制、health 状态
- Gateway API：Desktop UI 连接、运行时状态、消息流订阅、Active ACP Server 选择
- ChannelManager：飞书渠道生命周期和出站分发
- FeishuChannel：飞书轮询接入、消息转换、回复发送
- MessageBus：入站传递、出站分发、事件订阅
- agent_context：Agent 身份证、记忆注入、Prompt 组装
- acp_client：ACP 协议客户端，调用当前激活 ACP Server

### v1.1 — 多客户端与 Webhook 增强

- CLI / Web UI 接入 Gateway API
- 飞书 Webhook 入口与签名校验
- ACP Server 管理增强：连接测试、状态展示、切换历史
- Agent 处理状态和日志可视化
- Gateway Runtime 自启动和托盘控制

### v2.0 — 多渠道扩展

- Telegram / 钉钉 / Slack Channel
- Mobile companion 接入
- 渠道优先级和分流规则
- 多 ACP Server 路由规则作为独立增强方向
- Gateway Runtime 增强：统一限流、渠道健康状态、富媒体能力声明

## Part 6：关键假设

| 假设 | 验证方式 | 失效后果 |
|------|---------|---------|
| 用户需要桌面 UI 关闭后继续处理 IM 消息 | 内测访谈与后台运行原型 | Gateway Runtime 优先级下降，退回桌面内嵌服务 |
| 用户在 v1 只需要一个当前激活 ACP Server | 内测访谈与消息处理原型 | 提前引入多 ACP Server 路由设计 |
| 本地常驻进程在目标平台可稳定运行 | macOS Launch Agent / 托盘原型验证 | 需要降级为仅桌面运行模式 |
| ACP 协议生态持续发展 | 跟踪社区活跃度 | 替换为自定义 Agent 协议 |
| 飞书开放平台 API 满足轮询和 Webhook 接入需求 | 原型验证 | 调整渠道接入策略 |
| 本地 ACP Server 处理延迟用户可接受 | 内测反馈 | 增加流式响应和进度反馈 |

## Part 7：非目标

- 不做 SaaS 化的云服务
- 不做飞书机器人商店/应用市场
- 不做团队协作功能（多用户、权限）
- 不在设备关机或系统休眠时处理消息
