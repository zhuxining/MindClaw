# 迁移路线图

> 本文档记录从当前实现迁移到目标架构的状态和计划。迁移遵循“先文档对齐、再逐步重构、不破坏现有功能”的原则。产品目标见 `docs/blueprint/00-overview.md`，需求范围见 `docs/prd/`，目录结构见 `docs/architecture/reference/directory-structure.md`。

## 当前能力状态总览

| 能力 | 架构层 | 当前状态 | 说明 |
|------|--------|----------|------|
| 窗口关闭到托盘 | Channel Gateway Layer | 已实现 | 显式退出后停止，不承诺系统休眠或关机后继续运行。 |
| Feishu Channel 基础能力 | Channel Gateway Layer | 部分实现 | 已有 client、converter、credentials 基础；自动轮询、状态上报和内测体验待完善。 |
| Channel trait / ChannelRegistry | Channel Gateway Layer | 已实现基础 | trait 与 registry 已迁移到 `services/channels/`；具体渠道实现仍在 `services/im_channel/`。 |
| InboundDriver 抽象 | Channel Gateway Layer | 已实现基础 | 已有 trait / enum 骨架；Feishu 自动轮询任务待接入。 |
| ChannelManager | Channel Gateway Layer | 规划中 | 当前由 ChannelRegistry、渠道 client 与 GatewaySupervisor 分担职责。 |
| Gateway API adapter | Channel Gateway Layer | 规划中 | 当前 Desktop UI 主要通过 Tauri commands 访问 Services。 |
| Gateway health/status/stop | Channel Gateway Layer | 部分实现 | 基础启动与托盘驻留已有，完整 stop、health、status API 待补齐。 |
| SessionDispatcher | Agent Control Layer | 已实现基础 | 按会话串行处理，不同会话可并发。 |
| EventBus | Agent Control Layer | 部分实现 | 后端事件基础已存在，部分事件发布和前端订阅仍待补齐。 |
| Agent / Skill 基础模型 | Agent Control Layer | 部分实现 | 后端模型与持久化基础已存在，管理 UI 和完整验收仍待补齐。 |
| Agent-Skill 多对多 | Agent Control Layer | 已实现基础 | 仍需通过真实配置场景验证是否降低重复配置。 |
| ConversationExecutionState 持久化 | Agent Control Layer | 已实现基础 | 仍需验证多会话切换、恢复默认 Agent、Skill 状态一致性。 |
| SlashCommand parser | Agent Control Layer | 部分实现 | 解析层支持 `/help`、`/default`、`/use`、`/skill` 和任意 `/<name>` 执行入口；端到端执行依赖 ACP Execution / agent_context 后续阶段；无独立 `/agent` 命令。 |
| Agent Context 基础 prompt 组装 | ACP Execution Layer | 已实现基础 | 支持 Identity、Skill instruction、用户消息拼接。 |
| Agent Context 记忆 / 工具注入 | ACP Execution Layer | 规划中 | MemorySource 有基础 trait，ToolRegistry 待实现。 |
| ACP stdio client / Transport | ACP Execution Layer | 已实现基础 | 已有 ACP client、server registry、transport、tool executor 基础；SessionManager / 协议编解码待补齐。 |
| 执行元数据展示 | 跨层 | 规划中 | 需扩展 AgentResponse / DispatchResult 或引入 MessageExecution 记录。 |
| legacy MessageBus | legacy | 待删除 | RouteRule 退出主链路，`message_bus` 当前为空壳兼容模块。 |

## 迁移阶段

### Phase 0：文档重新对齐 ✅ 完成

目标：围绕“本地优先的 ACP-native Agent 控制平面”重写蓝图、PRD 和架构文档，明确文档边界。

| 任务 | 说明 | 状态 |
|------|------|------|
| P0.1 蓝图重写 | `docs/blueprint/00-overview.md` 聚焦产品定位、核心价值、演进方向 | ✅ |
| P0.2 PRD 简化 | `docs/prd/` 改为轻量产品需求文档 | ✅ |
| P0.3 架构重写 | `docs/architecture/` 按三层架构重新组织 | ✅ |
| P0.4 reference 清理 | directory-structure、migration、traceability 对齐新架构 | ✅ |

---

### Phase 1：类型与命名对齐 ✅ 完成

目标：消除命名不一致和核心类型寄生问题，不改动业务逻辑。

| 任务 | 说明 | 优先级 | 状态 |
|------|------|--------|------|
| T1.1 提取核心共享类型 | 将 `ChannelMessage`、`AgentResponse`、`ResponseStatus` 从 `message_bus/types.rs` 迁移到 `services::core` | P0 | ✅ |
| T1.2 重命名 `ChannelGateway` → `Channel` | trait 更名为 `Channel`，与文档一致 | P1 | ✅ |
| T1.3 重命名 `GatewayRegistry` → `ChannelRegistry` | 同步迁移到 `channels/` 目录下 | P1 | ✅ |
| T1.4 统一 CredentialsManager 实现命名 | `TokenManager` → `FeishuCredentialsManager` / `TelegramCredentialsManager` 等 | P2 | ⏳ |

---

### Phase 2：Agent Control Layer 持久化 🔨 基础完成

目标：拆分 Agent / Skill / Command / ConversationState 存储，形成用户侧 Agent 控制平面基础。

| 任务 | 说明 | 优先级 | 状态 |
|------|------|--------|------|
| T2.1 拆分 AgentStore | 拆分为 `AgentStore` facade、`SkillStore`、`CommandStore`、`ConversationStateStore` | P1 | ✅ |
| T2.2 提取 ConversationStateStore | 独立管理 `ConversationExecutionState`，支持 SQLite 持久化 | P1 | ✅ |
| T2.3 持久化去重状态 | `MessageStore` 的 `check_and_mark_seen` 迁移到 SQLite | P2 | ✅ |
| T2.4 验证默认 Agent / Skill 恢复 | 应用重启后恢复默认 Agent、Skill 和会话状态 | P1 | ⏳ |

---

### Phase 3：SessionDispatcher 与 EventBus 🔨 基础完成

目标：实现 per-session 保序、背压、重试和基础运行时事件。

| 任务 | 说明 | 优先级 | 状态 |
|------|------|--------|------|
| T3.1 实现 DispatchKey 和 per-session 队列 | `channel + conversation_id` → session queue | P1 | ✅ |
| T3.2 实现 per-session worker | 每个 session 一个 async worker，保证 FIFO | P1 | ✅ |
| T3.3 接入 EventBus 基础事件 | 发布 `MessageReceived`、`DispatchSucceeded` / `DispatchFailed`、`RuntimeStarted`、`ReplySent` / `ReplyFailed` | P1 | ✅ |
| T3.4 实现重试策略 | 指数退避，默认 2 次重试 | P2 | ✅ |
| T3.5 补齐运行时事件 | `DispatchStarted`、`ChannelPoll*`、`MessageDeduplicated` 等事件 | P2 | ⏳ |
| T3.6 前端订阅事件 | Tauri event / Gateway API adapter 将 EventBus 暴露给 UI | P2 | ⏳ |

---

### Phase 4：ACP Execution Layer 完整化 🔨 基础完成

目标：从基础 ACP 调用演进为完整 ACP Execution Layer。

| 任务 | 说明 | 优先级 | 状态 |
|------|------|--------|------|
| T4.1 定义 Transport trait | `transport.rs` — 异步 dispatch + test_connection 接口 | P2 | ✅ |
| T4.2 实现 AcpServerRegistry | 注册、查询和测试 ACP Server | P1 | ✅ 基础完成 |
| T4.3 实现 ToolExecutor | 本地工具权限控制与执行接口 | P2 | ✅ 基础完成 |
| T4.4 实现 SessionManager | ACP session 生命周期管理 | P2 | ⏳ |
| T4.5 实现协议编解码 / frame handling | ACP 协议帧解析与序列化 | P2 | ⏳ |
| T4.6 清理 legacy send seam | 完全迁移到 `send_to_server` / ACP-native 调用路径 | P3 | ⏳ |

---

### Phase 5：Agent Context 完整化 🔨 基础完成

目标：从基础 prompt 拼接演进为完整的上下文组装层。

| 任务 | 说明 | 优先级 | 状态 |
|------|------|--------|------|
| T5.1 实现基础 PromptBuilder | 结构化组装 Identity、Skill instruction 和用户消息 | P1 | ✅ |
| T5.2 实现 MemorySource 接口 | 会话历史检索与长期记忆注入 trait + NoopMemory | P2 | ✅ 基础完成 |
| T5.3 实现 ToolRegistry | 本地工具元数据注册与筛选 | P2 | ⏳ |
| T5.4 注入执行元数据 | prompt / request 携带 channel、conversation、sender 等必要上下文 | P2 | ⏳ |

---

### Phase 6：Gateway API Adapter 与 Runtime Status 🔨 基础完成

目标：从 Tauri commands 直连 Services 迁移到统一入口适配层，并完善运行时状态。

| 任务 | 说明 | 优先级 | 状态 |
|------|------|--------|------|
| T6.1 提取 GatewayAPIAdapter | Tauri Command 不再直接调用 GatewaySupervisor 内部结构 | P2 | ⏳ |
| T6.2 完善 Tauri Tray | 窗口关闭到托盘，托盘右键菜单（显示 / 退出） | P2 | ✅ 基础完成 |
| T6.3 启动时恢复状态 | 从 SQLite 恢复 Channel、Agent、Conversation 状态 | P2 | ✅ 基础完成 |
| T6.4 Runtime health/status API | GatewaySupervisor、Channel、Dispatcher、ACP Server 状态查询 | P1 | ⏳ |
| T6.5 UI 运行时状态展示 | Desktop UI 显示 ACP Server、Agent、Feishu、消息处理状态 | P1 | ⏳ |

---

### Phase 7：Feishu-first 自动接入 ⏳ 待开始

目标：完成 MVP 所需 Feishu 文本消息闭环。

| 任务 | 说明 | 优先级 | 状态 |
|------|------|--------|------|
| T7.1 InboundDriver trait | 支持 polling / long-polling / stream / webhook / manual | P2 | ✅ |
| T7.2 Feishu 自动轮询 | 基于 poll interval 的后台任务 | P1 | ⏳ |
| T7.3 Feishu 文本消息内测路径 | 文本消息 → ChannelMessage → Dispatcher → ACP → 建议回复 | P0 | ⏳ |
| T7.4 用户确认发送 | Agent 输出作为建议回复，用户确认后回写 Feishu | P0 | ⏳ |
| T7.5 受限自动回复 | 仅测试会话、明确开关、默认关闭 | P1 | ⏳ |

---

### Phase 8：目录整理与 legacy 删除 ⏳ 待开始

目标：在功能稳定后清理过渡目录和 legacy 空壳。

| 任务 | 说明 | 优先级 | 状态 |
|------|------|--------|------|
| T8.1 迁移 Feishu 实现 | `im_channel/feishu` → `channels/feishu` | P3 | ⏳ |
| T8.2 迁移 Telegram 实现 | `im_channel/telegram` → `channels/telegram` | P3 | ⏳ |
| T8.3 拆分 SessionDispatcher | `mod.rs` 拆分为 `types.rs`、`worker.rs`、`retry.rs` | P3 | ⏳ |
| T8.4 删除 legacy message_bus | 删除空壳兼容模块 | P3 | ⏳ |
| T8.5 清理旧命名 | TokenManager 等旧命名统一到 CredentialsManager 语义 | P3 | ⏳ |
| T8.6 重命名 SessionDispatcher 架构文档 | `docs/architecture/20-message-bus.md` → `20-session-dispatcher.md`，删除 MessageBus legacy 命名 | P3 | ⏳ |

## 废弃清单

| 模块 / 文件 | 状态 | 处理方式 |
|-------------|------|----------|
| `src-tauri/src/services/message_bus/router.rs` | 已删除 | 无需恢复 |
| `src-tauri/src/services/message_bus/types.rs` | 已删除 | 核心类型已迁移到 `services::core` |
| `src-tauri/src/services/acp_client/protocol.rs` | 已删除 | 后续如需协议结构，重新设计为 ACP-native protocol 模块 |
| `src-tauri/src/services/message_bus/mod.rs` | 待删除 | 当前为空壳兼容模块，待 legacy RouteRule 完全退场后删除 |
| `src-tauri/src/services/im_channel/` | 待迁移 | 具体渠道实现后续整体迁移到 `channels/feishu` 与 `channels/telegram` |

## 维护规则

- 新功能进入实现前，先确认对应 PRD 和架构文档已更新。
- 实现状态只在本文档维护，不在蓝图或 PRD 中重复写状态表。
- 目录迁移同步更新 `directory-structure.md`。
- 产品场景与模块映射同步更新 `traceability.md`。
