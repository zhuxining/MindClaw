# 目录结构

> 本文档记录当前代码目录、目标目录和迁移差异。产品目标见 `docs/blueprint/00-overview.md`，架构设计见 `docs/architecture/`，迁移状态见 `docs/architecture/reference/migration.md`。

## 当前目录结构（迁移中）

```text
src-tauri/src/
├── lib.rs                         # Tauri Builder：插件/命令注册、managed state 注入
├── main.rs                        # Rust 二进制入口
├── error.rs                       # AppError 定义
├── commands/                      # Tauri Command 层（thin），当前直接调用 Services / GatewaySupervisor
│   └── mod.rs
├── services/                      # Service 层（thick，业务逻辑；不 use tauri::*）
│   ├── mod.rs
│   ├── core/                      # 核心共享类型
│   │   └── mod.rs                 # ChannelMessage、AgentResponse、ResponseStatus
│   ├── gateway/                   # GatewaySupervisor 与运行时错误边界
│   │   ├── mod.rs
│   │   ├── supervisor.rs          # App 内驻留业务主管组件
│   │   └── error.rs               # GatewayError 错误类型
│   ├── channels/                  # Channel Gateway 抽象与注册
│   │   ├── mod.rs                 # Channel trait + CredentialsManager trait
│   │   ├── registry.rs            # ChannelRegistry：渠道注册与查找
│   │   └── inbound.rs             # InboundDriver + InboundChannel trait
│   ├── im_channel/                # 具体渠道实现（目标迁移到 channels/）
│   │   ├── mod.rs
│   │   ├── feishu/
│   │   │   ├── mod.rs
│   │   │   ├── client.rs          # FeishuClient（实现 Channel trait）
│   │   │   ├── converter.rs       # Feishu payload → ChannelMessage
│   │   │   └── token.rs           # Feishu 凭证管理
│   │   └── telegram/
│   │       ├── mod.rs
│   │       ├── client.rs          # TelegramClient（实现 Channel trait）
│   │       ├── converter.rs
│   │       └── token.rs           # Telegram 凭证管理
│   ├── agent/                     # Agent Control Layer
│   │   ├── mod.rs
│   │   ├── types.rs               # Agent、Identity、Skill、SlashCommand、ExecutionContext
│   │   ├── store.rs               # AgentStore facade + AgentDataStore
│   │   ├── skill_store.rs         # SkillStore（AgentSkillBinding 定义在 types.rs）
│   │   ├── command_store.rs       # CommandStore
│   │   ├── state_store.rs         # ConversationStateStore（SQLite + 内存 fallback）
│   │   ├── command_parser.rs      # SlashCommandParser
│   │   └── resolver.rs            # AgentResolver
│   ├── agent_context/             # Agent 上下文组装层
│   │   ├── mod.rs                 # AgentContextBuilder / prompt 组装
│   │   └── memory.rs              # MemorySource trait + NoopMemory
│   ├── acp_client/                # ACP Execution Layer
│   │   ├── mod.rs
│   │   ├── client.rs              # AcpClient：ACP 调用入口
│   │   ├── server.rs              # AcpServer、AcpServerStatus
│   │   ├── registry.rs            # AcpServerRegistry
│   │   ├── transport.rs           # Transport trait
│   │   └── tool_executor.rs       # ToolExecutor trait + NoopToolExecutor
│   ├── session_dispatcher/        # 会话调度器
│   │   └── mod.rs                 # per-session mpsc 队列 + FIFO worker + ACP 编排
│   ├── event_bus/                 # Runtime 事件 Pub/Sub
│   │   ├── mod.rs                 # EventBus
│   │   └── types.rs               # RuntimeEvent
│   └── message_bus/               # legacy RouteRule 兼容空壳，待删除
│       └── mod.rs
├── storage/                       # Storage 层（thin）
│   └── mod.rs                     # MessageStore（SQLite 去重 + 内存消息缓存）
└── config/                        # 配置管理
    └── mod.rs                     # AppConfig

src/
├── App.tsx                        # 根组件：Desktop UI 控制台
├── main.tsx                       # 前端入口
├── App.css
├── index.css
├── components/
│   ├── ChannelSettings.tsx
│   ├── FeishuSettings.tsx
│   └── MessageList.tsx
├── hooks/
│   └── use-mobile.ts
├── lib/
│   ├── types.ts
│   └── utils.ts
└── stores/
    └── message-store.ts
```

## 当前目录与三层架构的对应关系

| 架构层 | 当前目录 | 说明 |
|--------|----------|------|
| Channel Gateway Layer | `services/gateway/`、`services/channels/`、`services/im_channel/` | GatewaySupervisor、Channel trait / registry、Feishu / Telegram 具体实现 |
| Agent Control Layer | `services/agent/`、`services/session_dispatcher/`、`services/event_bus/` | Agent / Skill / SlashCommand、ConversationExecutionState、调度和事件 |
| ACP Execution Layer | `services/agent_context/`、`services/acp_client/` | prompt / context 组装、ACP Server 调用、ToolExecutor 边界 |
| Shared / Storage | `services/core/`、`storage/`、`config/` | 共享类型、SQLite / Stronghold / 配置 |
| UI / IPC | `src/`、`commands/`、`lib.rs` | Desktop UI、Tauri command、managed state 注入 |

## 当前与目标架构的差异

| 当前 | 目标 | 状态来源 |
|------|------|----------|
| 具体渠道实现仍在 `im_channel/` | 迁移到 `channels/feishu`、`channels/telegram` | `migration.md` Phase 7 / 废弃清单 |
| `ChannelRegistry` 承担注册和查找 | `ChannelManager` 统一生命周期、健康状态、inbound driver 和出站分发 | `migration.md` Phase 7 |
| Tauri Command 直接调用 Services / GatewaySupervisor | `GatewayAPIAdapter` 作为统一入口适配层 | `migration.md` Phase 6 |
| `agent_context` 主要承担基础 prompt 组装 | `PromptBuilder` + `MemorySource` + `ToolRegistry` | `migration.md` Phase 5 |
| `acp_client` 已有 transport / registry / tool executor 基础 | 完整 SessionManager、协议编解码和 legacy seam 清理 | `migration.md` Phase 4 |
| `session_dispatcher/` 仍为单文件 | 按 types / worker / retry 等职责拆分 | 后续代码整理 |
| `message_bus/` 为空壳兼容模块 | 删除 legacy message_bus 模块 | `migration.md` 废弃清单 |
| 前端仍为少量组件 | 后续按设置、消息、运行时状态等页面拆分 | UI 设计文档 / 后续 PRD |

## 目标目录结构

```text
src-tauri/src/
├── lib.rs
├── main.rs
├── error.rs
├── commands/
│   └── mod.rs                     # Tauri Command thin wrapper
├── services/
│   ├── mod.rs
│   ├── core/
│   │   └── mod.rs                 # ChannelMessage、AgentResponse、ResponseStatus、执行元数据
│   ├── gateway/
│   │   ├── mod.rs
│   │   ├── supervisor.rs          # GatewaySupervisor：App 内驻留业务主管组件
│   │   ├── api.rs                 # GatewayAPIAdapter：commands/local API/webhook 边界
│   │   ├── health.rs              # Health check 与运行状态
│   │   └── error.rs
│   ├── channels/
│   │   ├── mod.rs                 # Channel trait + CredentialsManager trait
│   │   ├── manager.rs             # ChannelManager：生命周期、健康状态、入站接收和出站分发
│   │   ├── registry.rs            # ChannelRegistry
│   │   ├── inbound.rs             # InboundDriver：polling/long-polling/stream/webhook/manual
│   │   ├── feishu/
│   │   │   ├── mod.rs
│   │   │   ├── client.rs
│   │   │   ├── converter.rs
│   │   │   └── credentials.rs
│   │   └── telegram/
│   │       ├── mod.rs
│   │       ├── client.rs
│   │       ├── converter.rs
│   │       └── credentials.rs
│   ├── agent/
│   │   ├── mod.rs
│   │   ├── types.rs
│   │   ├── store.rs
│   │   ├── skill_store.rs
│   │   ├── command_store.rs
│   │   ├── state_store.rs
│   │   ├── command_parser.rs
│   │   └── resolver.rs
│   ├── agent_context/
│   │   ├── mod.rs
│   │   ├── memory.rs
│   │   ├── prompt_builder.rs
│   │   └── tool_registry.rs
│   ├── acp_client/
│   │   ├── mod.rs
│   │   ├── client.rs
│   │   ├── server.rs
│   │   ├── registry.rs
│   │   ├── transport.rs
│   │   ├── session.rs
│   │   ├── protocol.rs
│   │   ├── tool_executor.rs
│   │   └── content.rs
│   ├── session_dispatcher/
│   │   ├── mod.rs
│   │   ├── types.rs
│   │   ├── worker.rs
│   │   └── retry.rs
│   └── event_bus/
│       ├── mod.rs
│       └── types.rs
├── storage/
│   └── mod.rs
└── config/
    └── mod.rs

src/
├── App.tsx
├── main.tsx
├── components/
│   ├── settings/
│   ├── messages/
│   ├── runtime/
│   └── ui/
├── hooks/
├── lib/
├── routes/
└── stores/
```

## 维护规则

- 新增、移动、删除代码目录时同步更新本文档。
- 只记录目录职责，不记录用户故事或产品目标。
- 当前实现状态写在 `migration.md`，本文档只引用状态来源。
- 目标目录是演进方向，不代表当前已实现。
