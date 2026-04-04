> **Status**: `active`
>
> 本文档描述当前使用的关键依赖，随 Cargo.toml 和 package.json 变化同步更新。

---

## Rust 后端依赖

### 框架与运行时

| crate | 用途 |
|-------|------|
| `tauri` (v2) | 桌面应用框架，IPC 和窗口管理 |
| `tokio` | 异步运行时 |
| `serde` / `serde_json` | 序列化/反序列化 |

### Tauri 插件

| 插件 | 用途 | 使用边界 |
|------|------|---------|
| `tauri-plugin-shell` | OS 命令执行 | 仅 Tauri Plugin 层，Services 不直接使用 |
| `tauri-plugin-dialog` | 文件对话框 | 仅 Tauri Plugin 层 |
| `tauri-plugin-clipboard-manager` | 剪贴板 | 仅 Tauri Plugin 层 |
| `tauri-plugin-notification` | 系统通知 | 仅 Tauri Plugin 层 |

> **使用边界**：Tauri Plugin 仅在命令层（`src/commands/`）通过 Plugin JS API 使用。Services 层不得 `use tauri::*`。

### 数据存储

| crate | 用途 |
|-------|------|
| `sqlx` (SQLite) | 数据库访问 |
| `keyring` | OS Keychain 访问（macOS / Windows / Linux） |

### AI / LLM

| crate | 用途 |
|-------|------|
| `rmcp` | MCP 协议官方 Rust SDK |
| `reqwest` | HTTP 客户端（调用 LLM API） |
| `eventsource-stream` | SSE 流式响应解析 |

### 并发与工具

| crate | 用途 |
|-------|------|
| `dashmap` | 并发 HashMap（Session Lock 存储） |
| `tokio::sync::Semaphore` | Concurrency Gate 实现 |
| `async-trait` | 异步 trait 支持 |
| `glob` | 文件 Glob 匹配（find_files 工具） |

---

## 前端依赖

### 框架与构建

| 包 | 用途 |
|----|------|
| `react` (v19) | UI 框架 |
| `typescript` | 类型系统 |
| `vite` | 构建工具 |
| `@tauri-apps/api` | Tauri IPC 调用 |

### UI 组件

| 包 | 用途 | 备注 |
|----|------|------|
| `shadcn/ui` | UI 组件库 | 基于 Base UI，不是 Radix |
| `@base-ui-components/react` | Base UI 基础组件 | shadcn 底层 |
| `@milkdown/crepe` | Markdown WYSIWYG 编辑器 | |

> **反模式**：不使用 `@radix-ui/*`，不使用 `asChild` prop（Base UI 使用 `render` prop）。

### 状态管理与路由

| 包 | 用途 |
|----|------|
| `zustand` | UI 状态管理 |
| `@tanstack/react-query` | 服务端状态（invoke 结果缓存） |
| `@tanstack/react-router` | 客户端路由 |

### 开发工具

| 包 | 用途 |
|----|------|
| `@biomejs/biome` | Lint + Format（替代 ESLint + Prettier） |
| `bun` | 包管理器 + 运行时 |

---

## 依赖使用原则

**Tauri Plugin 边界**：Tauri Plugin 仅在命令层使用，Services 层不得 `use tauri::*`。

**前端 API 调用**：所有后端调用通过 `@tauri-apps/api` 的 `invoke()`，不直接发起 HTTP 请求。

**密钥存储**：API Key 仅存储在 OS Keychain，不以任何形式落入前端或配置文件。
