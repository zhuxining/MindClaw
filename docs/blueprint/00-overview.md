> **Status**: `active`
>
> **Related specs**: MVP 规格见 `docs/prd/20-acp-native-feishu-agent-mvp.md`；实现状态见 `docs/architecture/reference/migration.md`；场景追踪见 `docs/architecture/reference/traceability.md`。

# 产品蓝图：本地优先的 ACP-native Agent 控制平面

## 产品定位

MindClaw 为已经拥有或愿意搭建 ACP Server 的开发者和重度 LLM 用户，在真实 IM 消息进入个人工作流时解决 Agent 执行入口分散、角色配置重复和会话状态不可控的问题，通过本地控制平面统一管理 Agent、Skill、会话状态和消息渠道，形成可复用、可解释、由用户本机掌控的 Agent 工作空间。

MindClaw 的产品边界由三条判断定义：

1. **ACP-native**：MindClaw 不自研基础 Agent Server，而是通过 ACP 调用用户已有或自建的 Agent 执行后端。
2. **Control plane**：MindClaw 管理用户侧的 Agent 角色、Skill、会话状态、默认 Agent 和显式命令。
3. **Messaging-native**：MindClaw 把这些 Agent 接入 Feishu 等真实消息渠道，让 Agent 不只停留在 IDE、CLI 或聊天窗口里。

更短的对外表达：

> MindClaw 让你自己的 ACP Agent 进入 IM 消息流。

## 目标用户与场景

### Primary User

- **ACP-native 开发者**：已经使用 Claude Code、Gemini CLI、自研 Agent 或其他 ACP Server；核心问题是底层执行能力已有，但缺少统一控制平面；成功状态是默认 Agent 能通过已有 ACP Server 处理真实消息。
- **重度 LLM 用户**：需要在个人工作流中反复切换角色、任务模板和上下文；核心问题是 Agent、Skill 和会话状态散落在不同工具；成功状态是在 MindClaw 中复用同一组 Agent / Skill 配置。
- **早期 IM Agent 内测用户**：希望让自己的 Agent 进入 Feishu 消息流；核心问题是每个渠道都要重复搭建 bot、runtime 和回复策略；成功状态是 Feishu 文本消息能进入本地处理链路并生成可确认的建议回复。

### Secondary User

- **个人知识工作者和高级自动化用户**：长期需要多个消息入口和多个 Agent，但不驱动 MVP 取舍；原因是第一阶段必须先验证 ACP + Feishu 的最小闭环。

### Excluded User

- **大型企业采购与团队管理用户**：不服务其审批、组织权限和团队协作流程；理由是这些流程会把产品拉向 SaaS 管理后台，偏离本地优先的个人控制平面。
- **只需要云端托管 Bot 的用户**：不服务无本地运行时、无 ACP Server 配置意愿的纯 SaaS 场景；理由是 MindClaw 的核心价值来自用户本机控制和 ACP-native 执行边界。
- **需要多 Agent 自动路由平台的用户**：不服务关键词分发、负载均衡和自动 Agent 路由；理由是当前命题要求显式选择和可解释执行。

### Core Scenario

1. **把已有 ACP Server 接入 MindClaw**：用户注册一个本地 ACP Server，并将默认 Agent 绑定到该 Server。MindClaw 不关心 Server 内部使用哪个模型或工具，只通过 ACP 协议发送请求、接收响应和记录执行状态。
2. **在 MindClaw 中管理 Agent 和 Skill**：用户创建 Agent，配置名称、描述、Identity、默认 ACP Server，并关联一组 Skill。Skill 独立管理，可被多个 Agent 复用。
3. **通过 SlashCommand 显式选择执行方式**：用户在对话中输入 `/review`、`/reply` 或 `/research`，系统将任意 `/<name>` 解析为显式执行入口，并根据命令选择目标 Agent 或快捷任务。固定控制命令包括 `/help`、`/default`、`/use`、`/skill`。
4. **把 Agent 接入 Feishu 消息流**：用户启用 Feishu 渠道后，GatewaySupervisor 接收 Feishu 消息，SessionDispatcher 按 `channel + conversation_id` 保持会话内顺序，AgentResolver 选择默认 Agent 或会话当前 Agent，agent_context 组装 prompt，acp_client 调用 Agent 绑定的 ACP Server，处理结果按当前回写策略形成建议回复或受限自动回复。
5. **Desktop UI 作为本地控制台**：用户重新打开 MindClaw 桌面端，查看 ACP Server、Agent、Skill、渠道连接、消息处理、EventBus 事件和当前会话执行状态。窗口关闭到托盘后，Tauri App 进程保持运行；显式退出 App、系统休眠或关机后不承诺继续运行。

## 核心命题

本产品的核心命题是：用户应当在本机拥有一个 ACP-native Agent 控制平面，用显式、可解释、可复用的 Agent / Skill / 会话状态，把已有 ACP Server 的执行能力接入真实消息流，而不是重新托管一个云端 Agent 平台。

这个命题排除四类方向：自研基础 Agent Server、MindClaw 云端消息处理服务、legacy RouteRule 自动路由、多用户团队管理后台。

## 产品原则

- **ACP 优先，不自研 Agent Server**：MindClaw 专注控制平面和渠道接入，底层 Agent 执行交给用户配置的 ACP Server。理由：复用已有执行能力能让产品聚焦在用户侧控制与消息场景。
- **用户侧 Agent 模型优先**：Agent、Skill、默认 Agent、会话状态属于 MindClaw 的核心产品模型，不依赖某个具体 ACP Server 的内部配置。理由：用户需要跨 ACP Server 复用角色、任务模板和执行状态。
- **显式选择优先于自动路由**：用户通过 `/命令` 切换 Agent 或 Skill，不在 v1 引入关键词 RouteRule、自动 Agent 路由或复杂优先级。理由：真实 IM 场景要求结果可解释，显式命令比隐式规则更可靠。
- **少渠道先闭环**：第一阶段用 Feishu 验证 ACP Server → Agent → Skill → 消息回复的完整价值链，不同时推进多渠道平台。理由：单渠道闭环能暴露控制平面的真实缺口。
- **本地优先与可观测优先**：MindClaw 自身不向 MindClaw 云端上传消息或上下文，并让用户看清哪个 Agent、哪个 Skill、哪个 ACP Server 处理了消息。理由：本地控制和执行可追踪是用户信任真实 IM 接入的前提。

## 长期演进方向

### 方向一：ACP 执行后端复用能力增强

- **能力变化**：从一个默认 ACP Server 扩展到多个 ACP Server 的注册、状态检测、默认绑定和执行元数据展示。
- **价值理由**：用户可以保留已有 Agent 执行后端，同时在 MindClaw 中统一管理面向消息流的执行选择。
- **边界**：不抽象模型供应商，不实现 ACP Server 内部 Agent 智能，不替代底层 Agent runtime。

### 方向二：Agent / Skill 控制平面增强

- **能力变化**：从默认 Agent 和默认 Skill 扩展到多 Agent、多 Skill、Agent-Skill 多对多、SlashCommand 和会话级选择状态。
- **价值理由**：用户能用同一组角色和任务模板处理不同消息，而不在多个后端里重复配置。
- **边界**：不做 legacy RouteRule 混合优先级，不做多个 Agent 并行处理同一消息，不把自动路由作为 v1 主路径。

### 方向三：消息渠道接入增强

- **能力变化**：从 Feishu 文本消息闭环扩展到更多入站方式、出站回复策略、渠道健康状态和富媒体能力声明。
- **价值理由**：Agent 的价值来自进入真实消息流；渠道扩展应服务于同一套 Agent 控制平面。
- **边界**：不同时追求多渠道平台，不默认提供公网 webhook relay，不把渠道能力扩展成独立 SaaS bot 市场。

### 方向四：本地控制入口与可观测性增强

- **能力变化**：从 Desktop UI + Tauri commands 扩展到 Gateway API adapter、运行时状态、事件订阅、托盘控制和本地自动化入口。
- **价值理由**：用户需要理解系统是否正在工作、失败发生在哪里，以及如何从本机控制消息处理链路。
- **边界**：不提供团队协作权限系统，不承诺 App 完全退出、系统休眠或关机后继续处理消息。

## 非目标

- **不做 SaaS 化的云端消息处理服务**：理由是产品核心是本地优先控制平面；相关云端中继需求进入独立产品方向评估。
- **不做飞书机器人商店或应用市场**：理由是首要问题是用户本机 Agent 接入真实消息流；渠道分发和商业化市场不进入当前产品判断。
- **不做自研基础 Agent Server**：理由是 ACP-native 要求复用用户已有或自建执行后端；底层执行能力由 ACP Server 文档和配置承担。
- **不做模型供应商抽象层**：理由是模型选择属于 ACP Server 内部职责；MindClaw 只记录和调用 ACP Server。
- **不做团队协作功能（多用户、组织权限、审批）**：理由是这些能力会改变产品主语；相关需求需先进入新的产品域蓝图讨论。
- **不在设备关机、系统休眠或 App 完全退出时处理消息**：理由是 v1 后台形态是 Tauri App 内驻留；后台能力边界写入架构与 PRD。
- **不把 legacy RouteRule 作为 Agent 选择机制**：理由是 Agent 选择必须显式、可解释；旧路由规则只作为迁移背景保留。

## 关键假设

- 用户已经拥有或愿意搭建 ACP Server；验证方式：内测配置完成率和 ACP Server 连接成功率；失效后果：ACP-native 定位需要重新评估。
- Feishu 文本消息能代表第一阶段真实 IM 价值；验证方式：内测用户是否把建议回复用于实际会话；失效后果：消息渠道演进方向需要调整。
- 用户接受默认建议回复、确认后发送的安全路径；验证方式：建议回复采纳率和误发反馈；失效后果：回写策略需要更保守，自动回复继续保持非默认。
- 显式 SlashCommand 足以覆盖早期 Agent / Skill 选择需求；验证方式：命令使用频率和失败原因统计；失效后果：自动辅助选择需作为独立产品方向重新评估。

## 文档关联

- PRD 总览：`docs/prd/00-overview.md`
- 当前 MVP 需求：`docs/prd/20-acp-native-feishu-agent-mvp.md`
- Agent / Skill / SlashCommand 需求：`docs/prd/10-agent-skill-slash-command.md`
- 架构总览：`docs/architecture/00-overview.md`
- Channel Gateway 架构：`docs/architecture/10-channel-gateway.md`
- SessionDispatcher 与 EventBus 架构：`docs/architecture/20-message-bus.md`
- ACP Execution Layer 架构：`docs/architecture/30-acp-client.md`
- Agent Context 架构：`docs/architecture/35-agent-context.md`
- Agent / Skill / SlashCommand 架构：`docs/architecture/40-agent-skill-command.md`
- 迁移状态：`docs/architecture/reference/migration.md`
- 可追溯性矩阵：`docs/architecture/reference/traceability.md`
