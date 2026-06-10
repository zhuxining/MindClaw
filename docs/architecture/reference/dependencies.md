# 依赖清单

> 本文档记录项目的依赖关系，在 Cargo.toml / package.json 变更时同步更新。

## Rust (src-tauri/Cargo.toml)

### 现有依赖

- `tauri` 2.x — 桌面应用框架，提供 app/window/webview runtime、IPC、managed State、AppHandle、plugin lifecycle 和 async runtime
- `tauri-plugin-stronghold` 2 — 密钥存储
- `tauri-plugin-fs` 2 — 文件系统访问
- `tauri-plugin-dialog` 2 — 系统对话框
- `tauri-plugin-notification` 2 — 系统通知
- `tauri-plugin-clipboard-manager` 2 — 剪贴板
- `tauri-plugin-os` 2 — OS 信息
- `tauri-plugin-process` 2 — 进程管理
- `tauri-plugin-opener` 2 — 打开文件/URL
- `tauri-plugin-autostart` 2 — 开机启动
- `tauri-plugin-cli` 2 — CLI 参数
- `tauri-plugin-global-shortcut` 2 — 全局快捷键
- `tauri-plugin-persisted-scope` 2 — 持久化权限
- `serde` / `serde_json` — JSON 序列化
- `tokio` — 异步运行时依赖；`sync` feature 支持 mpsc、oneshot 和 broadcast
- `tokio-util` — CancellationToken，用于 GatewaySupervisor lifecycle 和 worker 停止
- `chrono` — 时间处理
- `uuid` — 消息、请求和 session/run ID 生成
- `reqwest` — HTTP 客户端
- `rusqlite` — SQLite 存储
- `async-trait` — dyn async trait 兼容
- `thiserror` — 错误类型定义

### 运行时依赖约定

- GatewaySupervisor 复用 Tauri App 进程和 Tauri async runtime，不创建独立 `tokio::runtime::Runtime`。
- `services/**` 不依赖 `tauri::*`；Tauri AppHandle、event emit、capability 和 plugin 生命周期由 `lib.rs`、`commands` 或 Gateway API adapter 处理。
- 后台任务使用 cancellation token 管理停止；显式退出应用时停止 inbound drivers 和 dispatcher workers。
- v1 不引入 daemon、sidecar、systemd、launchd 或 Windows service 依赖。

## 架构约定

- `src-tauri/src/services/gateway/` — Gateway Runtime 业务边界
  - `GatewaySupervisor` — App 内驻留业务主管组件，组装、启动、停止和监督内部子模块
  - `GatewayAPIAdapter` — Desktop UI、CLI、Web UI、Mobile companion、Webhook 的统一入口适配
  - `Health` — 运行状态
  - `GatewayError` — 运行时错误边界
- `src-tauri/src/services/agent/` — Agent 执行模型
  - `Agent` — 用户可选择的执行者，默认拥有 Identity，绑定默认 ACP Server
  - `Identity` — Agent 的身份、人设和行为约束
  - `Skill` — 独立管理的任务能力模板
  - `AgentSkillBinding` — Agent 与 Skill 的多对多关联
  - `SlashCommandParser` — slash command 解析
  - `AgentResolver` — 根据消息和 ConversationExecutionState 解析 ExecutionContext
  - `ConversationExecutionStateStore` — 按 conversation 保存当前 Agent 和 Skill
- `src-tauri/src/services/channels/` — 渠道管理与具体渠道实现
  - `ChannelManager` — 渠道生命周期、健康状态、入站接收和出站分发
  - `ChannelRegistry` — 渠道注册、查找和列表顺序
  - `InboundDriver` — polling、long-polling、stream、webhook handler 和 manual input 的接收方式抽象
  - `Channel` — 渠道轮询、Webhook 处理、消息发送与凭证访问的统一 trait
  - `CredentialsManager` — 渠道凭证管理 trait
- `src-tauri/src/services/session_dispatcher/` — 会话调度器
  - `DispatchKey` — `channel + conversation_id` 的 session 标识
  - `SessionDispatcher` — per-session 队列、worker、去重、slash command 入口、ACP 调用和回复编排
  - `RetryPolicy` — 调度层重试策略
- `src-tauri/src/services/event_bus/` — Runtime 事件 Pub/Sub
  - `EventBus` — 基于 `tokio::sync::broadcast` 的事件总线
  - `RuntimeEvent` — runtime/channel/message/dispatch/reply/agent/skill/slash command 事件
- `src-tauri/src/services/message_bus/` — legacy RouteRule 兼容模块
  - `MessageBus` — legacy RouteRule 管理
  - `RouteRule` — legacy 路由规则，不参与主调度链路
  - `ChannelMessage` / `AgentRequest` / `AgentResponse` — 迁移期间复用的跨模块类型
- `src-tauri/src/services/agent_context/` — Agent 上下文组装层
  - `Memory` — 记忆检索与注入
  - `PromptBuilder` — 将 Agent Identity、Skill instruction、记忆和工具元数据合并为请求
  - `ToolRegistry` — 本地工具元数据注册表
- `src-tauri/src/services/acp_client/` — ACP 协议客户端
  - `AcpClient` — ACP 协议客户端
  - `AcpServerRegistry` — ACP Server 注册与状态查询
  - `Transport` / `SessionManager` / `Protocol` / `ToolExecutor` — ACP 协议能力适配

## Frontend (package.json)

### 现有依赖

- `react` 19 / `react-dom` 19
- `@base-ui/react` — UI 基座（shadcn/ui 基于此）
- `@milkdown/crepe` / `@milkdown/kit` — Markdown 编辑器
- `@tanstack/react-query` — 服务端状态
- `@tanstack/react-router` — 路由
- `zustand` — UI 状态
- `effect` — Effect-TS
- `tailwindcss` 4 — CSS 框架
- `lucide-react` — 图标库

### 消息调度 UI 依赖

- 无额外依赖 — 复用现有 shadcn/ui 组件
