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

- `reqwest` — HTTP 客户端（飞书 Open API 调用）
- `serde` / `serde_json` — JSON 序列化（已内置）
- `tokio` — 异步运行时（已内置）
- `chrono` — 时间处理
- `uuid` — 消息/请求 ID 生成

### 架构约定

- `src-tauri/src/services/im_channel/` — 渠道协议适配层
  - `im_channel/feishu/` — 飞书渠道的 `ChannelAdapter` 实现
  - `im_channel/telegram/` — Telegram Bot 渠道的 `ChannelAdapter` 实现
  - 新增渠道只需实现 `ChannelAdapter` + `CredentialsManager` trait
- `src-tauri/src/services/gateway/` — 网关层
  - `GatewayRegistry` — 渠道注册中心
  - `AuthFilter` — 身份鉴权
  - `RateLimiter` — 流量控制
  - `Transformer` — 消息标准化（`RawMessage` → `ChannelMessage`）
- `src-tauri/src/services/message_bus/` — 消息总线路由层
  - `Router` — 路由引擎
  - `Topic` / `SubscriptionMgr` — 消息分发与订阅管理
- `src-tauri/src/services/acp/` — Action Control Plane 执行层
  - `Router` — 意图识别
  - `Planner` — 任务规划
  - `Executor` — 动作执行
  - `Memory` — 记忆管理

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
