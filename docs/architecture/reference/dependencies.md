# 依赖清单

> 本文档记录项目的依赖关系，在 Cargo.toml / package.json 变更时同步更新。

## Rust (src-tauri/Cargo.toml)

### 现有依赖

- `tauri` 2.x — 桌面应用框架
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

### 新增依赖（消息调度子系统）

- `reqwest` — HTTP 客户端（飞书 Open API、Telegram Bot API、Gateway API 调用）
- `serde` / `serde_json` — JSON 序列化（已内置）
- `tokio` — 异步运行时（已内置）
- `chrono` — 时间处理
- `uuid` — 消息/请求 ID 生成

### 架构约定

- `src-tauri/src/services/gateway/` — Gateway Runtime，本地常驻运行时
  - `GatewayRuntime` — 启动、停止和监督内部子模块
  - `GatewayAPI` — Desktop UI、CLI、Web UI、Mobile companion、Webhook 的统一入口
  - `Health` / `Supervisor` — 运行状态和后台任务监督
  - `ActiveAcpServer` — 当前激活 ACP Server 的选择和状态
  - `GatewayError` — 运行时错误边界
- `src-tauri/src/services/channels/` — 渠道管理与具体渠道实现
  - `ChannelManager` — 渠道生命周期、健康状态、入站接收和出站分发
  - `ChannelRegistry` — 渠道注册与查找
  - `Channel` — 渠道轮询、Webhook 处理、消息发送与凭证访问的统一 trait
  - `CredentialsManager` — 渠道凭证管理 trait
  - `channels/feishu/` — 飞书渠道实现，包含 `FeishuChannel`、`TokenManager`、飞书消息到 `ChannelMessage` 的转换器
  - `channels/telegram/` — Telegram 渠道实现，包含 `TelegramChannel`、`TelegramTokenManager`、Telegram Update 到 `ChannelMessage` 的转换器
- `src-tauri/src/services/message_bus/` — Gateway Runtime 内部消息总线层
  - `inbound` — 将渠道消息传递给 Active ACP Dispatch
  - `outbound` — 将 Agent 响应传递给 ChannelManager
  - `Topic` / `SubscriptionMgr` — 消息事件分发与订阅管理
- `src-tauri/src/services/agent_context/` — Agent 上下文组装层
  - `Identity` — Agent 身份证管理
  - `Memory` — 记忆检索与注入
  - `PromptBuilder` — Prompt 组装器
  - `ToolRegistry` — 本地工具元数据注册表
- `src-tauri/src/services/acp_client/` — ACP 协议客户端
  - `AcpClient` — ACP Agent 客户端
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

### 新增依赖（消息调度 UI）

- 无额外依赖 — 复用现有 shadcn/ui 组件
