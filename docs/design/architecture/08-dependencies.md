# MindClaw 技术架构设计

> 完整架构文档索引见 [README.md](./README.md)

## 九、技术依赖

### 层职责与 Tauri Plugin 使用原则

三层架构：**Web 层（遥控器）→ Tauri 层（连接线）→ Rust 层（主机）**

| 层 | 职责 | 原则 |
|---|------|------|
| Web (React) | UI 渲染、用户交互、状态展示 | 薄客户端，不持有业务逻辑 |
| Tauri (IPC + Plugins) | 窗口管理、系统能力桥接、权限管控 | 胶水层，不写业务逻辑 |
| Rust (Services) | 全部业务逻辑、数据持久化、LLM 通信 | 核心层，可脱离 Tauri 独立运行 |

#### Web 端使用边界

- ✅ 渲染 UI、收集用户输入、调 `invoke()` 拿数据
- ✅ Zustand 仅管 UI 状态（sidebar 展开、当前页面等）
- ✅ 可直接用 Tauri Plugin JS API 做系统交互（clipboard、dialog、notification、fs）
- ❌ 不直接发 HTTP 请求（不用 fetch 调外部 API）
- ❌ 不做数据持久化（不用 localStorage / plugin-store）
- ❌ 不处理业务逻辑（过滤、排序、转换都在 Rust 端完成后返回）

#### Tauri Plugin 分类

| 用法 | Plugin | 说明 |
|------|--------|------|
| 前端直接用 JS API | clipboard、dialog、notification、fs | 纯 UI 交互，不经过业务逻辑 |
| Rust 端后台用 | persisted-scope、autostart、window-state | 前端不感知 |
| 被 Service 包装 | stronghold | 密钥存取，通过 command 暴露 |

#### 不该用 Plugin 的场景

| 需求 | 正确方案 | 原因 |
|------|---------|------|
| HTTP 请求 | Rust `reqwest` | 统一出口，便于 retry/logging |
| WebSocket | Rust `tokio-tungstenite` | Channel 层长连接，生命周期由 Rust 管理 |
| KV 存储 | Rust SQLite / 文件 | 单一数据源 |
| Shell 执行 | Rust `tokio::process` | Agent tool，需权限管控 |

#### 逻辑归属决策

- 需要系统原生能力（剪贴板、通知、文件选择器）→ Tauri Plugin JS API
- 需要网络通信（API 调用、WebSocket）→ Rust 端
- 需要数据读写 → Rust Service → Storage
- 需要 UI 状态（展开/折叠、选中项）→ 前端 Zustand
- 其他一切业务逻辑 → Rust Service

### Rust 依赖（Cargo.toml）

#### Tauri Plugins（跨平台）

```toml
tauri = { version = "2", features = [] }
tauri-plugin-opener = "2"
tauri-plugin-clipboard-manager = "2"
tauri-plugin-dialog = "2"
tauri-plugin-fs = "2"
tauri-plugin-notification = "2"
tauri-plugin-os = "2"
tauri-plugin-persisted-scope = "2"
tauri-plugin-process = "2"
tauri-plugin-stronghold = "2"
```text

#### Tauri Plugins（仅桌面端）

```toml
tauri-plugin-autostart = "2"
tauri-plugin-cli = "2"
tauri-plugin-global-shortcut = "2"
tauri-plugin-updater = "2"
tauri-plugin-window-state = "2"
```text

#### 核心依赖

```toml
# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Async runtime（按需选 feature，不用 "full"）
tokio = { version = "1", default-features = false, features = ["rt-multi-thread", "macros", "time", "net", "io-util", "sync", "process", "io-std", "fs", "signal"] }
tokio-stream = { version = "0.1", default-features = false }

# Database
rusqlite = { version = "0.39", features = ["bundled"] }

# HTTP client（关闭默认 feature，用 rustls）
reqwest = { version = "0.13", default-features = false, features = ["json", "rustls"] }

# Configuration
directories = "6.0"
shellexpand = "3.1"
toml = "1.0"

# Utilities
base64 = "0.22"
image = { version = "0.25", default-features = false, features = ["jpeg", "png"] }
urlencoding = "2.1"
uuid = { version = "1.22", default-features = false, features = ["v4", "std"] }
glob = "0.3"
which = "8.0"
nanohtml2text = "0.2"
chrono = { version = "0.4", default-features = false, features = ["clock", "std", "serde"] }
regex = "1.10"
async-trait = "0.1"
futures-util = { version = "0.3", default-features = false, features = ["sink"] }

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["fmt", "ansi", "env-filter"] }

# Error handling
thiserror = "2.0"
anyhow = "1.0"
```text

### 前端依赖（package.json）

#### 框架与 UI

| 包 | 版本 | 用途 |
|---|------|------|
| react / react-dom | ^19 | 前端框架 |
| shadcn + tailwindcss + tw-animate-css | ^4 / ^4 / ^1 | UI 组件系统（基于 **Base UI**，按需`bunx shadcn@latest add <name>`） |
| @base-ui/react | ^1 | shadcn 底层无头组件库（非 Radix UI） |
| class-variance-authority + clsx + tailwind-merge | — | shadcn 配套样式工具 |
| lucide-react | ^1 | 图标库 |
| cmdk | ^1 | Command palette（⌘K） |
| react-resizable-panels | ^4 | 可拖拽分栏面板 |
| @fontsource-variable/inter | ^5 | 字体 |

**shadcn/ui 使用规范：**

- 添加组件：`bunx shadcn@latest add <component>`
- 组件生成到 `src/components/ui/`，可按需定制
- Base UI 组件规范（render prop、useRender、Field 等）详见 `shadcn` skill
- **反模式：**
  - 不要使用 `asChild` — Base UI 不支持，改用 `render` prop
  - 不要安装 `@radix-ui/*` — 本项目使用 Base UI 替代

#### 编辑器

| 包 | 版本 | 用途 |
|---|------|------|
| @milkdown/crepe | ^7 | Markdown 所见即所得编辑器（用于 Knowledge / Daily 编辑） |

#### 状态与路由

| 包 | 版本 | 用途 |
|---|------|------|
| zustand | ^5 | 轻量状态管理（UI 状态：sidebar、当前页面等） |
| @tanstack/react-router | ^1 | 类型安全路由 |
| @tanstack/react-query | ^5 | 服务端状态管理（invoke 请求缓存、自动重取） |

#### Tauri Plugin 前端包

```text
@tauri-apps/api, plugin-autostart, plugin-cli, plugin-clipboard-manager,
plugin-dialog, plugin-fs, plugin-global-shortcut, plugin-notification,
plugin-opener, plugin-os, plugin-process, plugin-stronghold,
plugin-updater, plugin-window-state
```text
