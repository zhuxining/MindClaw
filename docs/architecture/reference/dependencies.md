# 依赖清单与架构约定

> 本文档记录项目依赖和跨模块架构约定。Cargo.toml / package.json 变更、架构边界变化时同步更新。当前实现状态见 `docs/architecture/reference/migration.md`。

## Rust (src-tauri/Cargo.toml)

### 现有依赖

- `tauri` 2.x — 桌面应用框架，提供 app/window/webview runtime、IPC、managed State、AppHandle、plugin lifecycle 和 async runtime。
- `tauri-plugin-stronghold` 2 — 密钥存储。
- `tauri-plugin-fs` 2 — 文件系统访问。
- `tauri-plugin-dialog` 2 — 系统对话框。
- `tauri-plugin-notification` 2 — 系统通知。
- `tauri-plugin-clipboard-manager` 2 — 剪贴板。
- `tauri-plugin-os` 2 — OS 信息。
- `tauri-plugin-process` 2 — 进程管理。
- `tauri-plugin-opener` 2 — 打开文件 / URL。
- `tauri-plugin-autostart` 2 — 开机启动。
- `tauri-plugin-cli` 2 — CLI 参数。
- `tauri-plugin-global-shortcut` 2 — 全局快捷键。
- `tauri-plugin-persisted-scope` 2 — 持久化权限。
- `agent-client-protocol` — ACP 协议类型与客户端能力。
- `serde` / `serde_json` — JSON 序列化。
- `tokio` — 异步运行时依赖；`sync` feature 支持 mpsc、oneshot 和 broadcast。
- `tokio-util` — CancellationToken，用于 GatewaySupervisor lifecycle 和 worker 停止。
- `chrono` — 时间处理。
- `uuid` — 消息、请求和 session / run ID 生成。
- `reqwest` — HTTP 客户端。
- `rusqlite` — SQLite 存储。
- `async-trait` — dyn async trait 兼容。
- `thiserror` — 错误类型定义。

### 运行时依赖约定

- GatewaySupervisor 复用 Tauri App 进程和 Tauri async runtime，不创建独立 `tokio::runtime::Runtime`。
- `services/**` 不依赖 `tauri::*`；Tauri AppHandle、event emit、capability 和 plugin 生命周期由 `lib.rs`、`commands` 或 Gateway API adapter 处理。
- 后台任务使用 cancellation token 管理停止；显式退出应用时停止 inbound drivers 和 dispatcher workers。
- v1 不引入 daemon、sidecar、systemd、launchd 或 Windows service 依赖。
- Service 层 async 方法中避免使用 `std::sync::Mutex`，优先使用 `tokio::sync::Mutex`，防止阻塞 async executor。

## 架构层约定

### Web / Desktop UI

- React 只做 UI 渲染、输入收集和调用 Tauri commands。
- 不直接访问 Feishu、Telegram、ACP Server 或本地 SQLite。
- 不实现业务调度、Agent 选择或渠道轮询策略。
- 服务端状态使用 TanStack Query，UI 状态使用 Zustand。

### Tauri Commands / Gateway API Adapter

- 当前 MVP 可由 Tauri commands 直接调用 Services。
- 后续 Gateway API adapter 作为 Desktop UI、CLI、Local API、Webhook 的统一入口。
- Command 层保持 thin wrapper：参数转换、调用 Service、返回 `Result<T, AppError>`。
- Command 层可以依赖 `tauri::*`；Service 层不可以。

### Services

- `src-tauri/src/services/core/` — 核心共享类型。
  - `ChannelMessage` / `AgentResponse` / `ResponseStatus` — 跨渠道、调度和 ACP 调用复用的核心类型。
- `src-tauri/src/services/gateway/` — Gateway Runtime 业务边界。
  - `GatewaySupervisor` — App 内驻留业务主管组件，组装、启动、停止和监督内部子模块。
  - `GatewayAPIAdapter`（后续）— Desktop UI、CLI、Web UI、Webhook 的统一入口适配。
  - `GatewayError` — 运行时错误边界。
- `src-tauri/src/services/channels/` — 渠道抽象与注册。
  - `Channel` — 渠道轮询、Webhook 处理、消息发送与凭证访问的统一 trait。
  - `CredentialsManager` — 渠道凭证管理 trait。
  - `ChannelRegistry` — 渠道注册、查找和列表顺序。
  - `InboundDriver` — polling / long-polling / stream / webhook / manual input 的统一接收方式。
  - `FeishuClient` / `TelegramClient` — 实现 `channels::Channel` trait；分别位于 `channels/feishu/`、`channels/telegram/`。
  - Feishu `TokenManager` / `TelegramTokenManager` — 实现 `channels::CredentialsManager` trait。
- `src-tauri/src/services/agent/` — Agent Control Layer。
  - `Agent` — 用户可选择的执行者，默认拥有 Identity，绑定默认 ACP Server。
  - `Identity` — Agent 的身份、人设和行为约束。
  - `Skill` — 独立管理的任务能力模板。
  - `AgentSkillBinding` — Agent 与 Skill 的多对多关联。
  - `SlashCommandParser` — SlashCommand 解析。
  - `AgentResolver` — 根据消息和 ConversationExecutionState 解析 ExecutionContext。
  - `AgentStore` — Agent / Skill / Command / ConversationState 的 facade。
  - `SkillStore` / `CommandStore` / `ConversationStateStore` — AgentStore 下的具体子存储。
- `src-tauri/src/services/session_dispatcher/` — 会话调度器。
  - `SessionDispatcher` — per-session 队列、FIFO worker、SlashCommand 入口、ACP 调用和回复编排。
  - `types.rs` — `DispatchCommand` 类型。
  - `worker.rs` — `session_worker`、`process_message` 和重试编排。
  - `retry.rs` — `RetryPolicy` 重试策略。
- `src-tauri/src/services/event_bus/` — Runtime 事件 Pub/Sub。
  - `EventBus` — 基于 `tokio::sync::broadcast` 的事件总线。
  - `RuntimeEvent` — runtime / channel / message / dispatch / reply / agent / skill / slash command 事件。
- `src-tauri/src/services/agent_context/` — Agent 上下文组装层。
  - `AgentContextBuilder` / `PromptBuilder` — Identity、Skill、Memory、Tool metadata、用户消息组装。
  - `MemorySource` — 会话历史和长期记忆来源。
  - `ToolRegistry` — 可暴露给 ACP Server 的本地工具元数据。
- `src-tauri/src/services/acp_client/` — ACP Execution Layer。
  - `AcpClient` — ACP 协议调用入口。
  - `AcpServerRegistry` — ACP Server 注册与状态查询。
  - `Transport` — stdio / HTTP / 其他传输抽象。
  - `SessionManager` — ACP 会话管理。
  - `ToolExecutor` — 本地工具执行与权限控制。

### Storage

- 当前实体：`MessageStore`，负责 SQLite 去重和内存消息缓存。
- Storage 层保持 thin，负责 SQLite / Stronghold / 文件系统等持久化细节。
- 业务决策不写入 Storage 层。
- Secrets 必须进入 Stronghold 或等价安全存储。

## Frontend (package.json)

### 现有依赖

- `react` 19 / `react-dom` 19。
- `@base-ui/react` — UI 基座（shadcn/ui 基于此）。
- `@milkdown/crepe` / `@milkdown/kit` — Markdown 编辑器。
- `@tanstack/react-query` — 服务端状态。
- `@tanstack/react-router` — 路由。
- `zustand` — UI 状态。
- `effect` — Effect-TS。
- `tailwindcss` 4 — CSS 框架。
- `lucide-react` — 图标库。

### UI 依赖约定

- 不引入 `@radix-ui/*`，shadcn/ui 基于 Base UI。
- Base UI 使用 `render` prop，不使用 Radix 风格 `asChild`。
- 新 UI 组件通过 `bunx shadcn@latest add <name>` 添加。
- 前端不直接发起业务 HTTP 请求；业务调用通过 Tauri commands / Gateway API adapter。
