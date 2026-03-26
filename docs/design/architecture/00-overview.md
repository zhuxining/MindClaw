# MindClaw 技术架构设计

> 拆分自 architecture.md，完整索引见 [README.md](./README.md)

## 一、系统总览

### 技术栈

| 层 | 技术 | 版本 |
|---|------|------|
| 桌面框架 | Tauri | 2.x |
| 前端 | React + TypeScript | 19.x / 6.x |
| 后端 | Rust | 2021 Edition |
| 构建 | Vite + Bun | 8.x / latest |
| 存储 | SQLite + Markdown + 本地文件系统 | — |
| LLM | Claude API (BYOK) | — |

### 架构分层

```
┌─────────────────────────────────────────────────────────┐
│                React Frontend (UI)                      │
│  Pages · Components · Hooks · Zustand Store             │
├──────────────┬───────────────┬──────────────────────────┤
│ Web Commands │ Agent Cmds    │  CLI Binary              │
│ invoke()     │ /new /stop    │  mindclaw <sub>       │
│ → Services   │ /restart      │  clap → CliRuntime       │
│ (~28 个 IPC) │ /status       │  → Services              │
├──────────────┴───────────────┴──────────────────────────┤
│   Channel Layer   │         Gateway Layer               │
│   Desktop         │  HTTP Server (PWA/Webhook)          │
│   Telegram        │  WebSocket (实时通信)                │
│   Feishu          │                                     │
├───────────────────┴─────────────────────────────────────┤
│          MessageBus（双向异步消息队列）                   │
│   inbound: Channel → Agent  │  outbound: Agent → Channel│
├─────────────────────────────┴───────────────────────────┤
│             Core Agent Service (编排器)                  │
│  AgentLoop · Context · Session · SubAgent               │
├──────────────────┬──────────────────────────────────────┤
│  Provider Layer  │          Tool Layer                  │
│  Claude API      │  基础能力: fs · shell · mcp_client   │
│  Haiku / Sonnet  │  元工具: operations (按需调用)        │
├──────────────────┼──────────────────────────────────────┤
│  Memory Layer    │        Services Layer                │
│  Agent 私有记忆   │  核心业务逻辑 (前端/Agent 共用)       │
│  观察·偏好·模式   │  Knowledge · Daily · Task · Capture │
│  (SQLite)        │  (操作 Markdown + SQLite)            │
├──────────────────┴──────────────────────────────────────┤
│           Infrastructure Layer (基础设施)               │
│  Cron (定时任务) · Heartbeat (健康检测) · Logging       │
├──────────────────┬──────────────┬───────────────────────┤
│   SQLite         │  Markdown FS │  OS Keychain          │
│  结构+索引+记忆   │  内容真相     │  API Key              │
└──────────────────┴──────────────┴───────────────────────┘

调用关系：
  前端: Commands → Services → Storage
  Agent: AgentLoop → Tools → Services → Storage
                   → Memory → Storage
                   → Provider (LLM)
  记忆是 Agent 的 (Memory/SQLite)，知识是共同的 (Knowledge/Markdown)
```

### 桌面端即服务器

桌面端是数据和 Agent 的唯一运行环境。移动端（Phase 2）通过本地 WiFi 或 Tailscale 接入桌面端的 Web Server，作为薄客户端。MVP 阶段移动对话通过 Telegram/Feishu Bot webhook 实现。

---
