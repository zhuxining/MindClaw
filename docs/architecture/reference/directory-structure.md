# 目录结构

> 本文档记录当前代码目录结构。产品目标见 `docs/blueprint/00-overview.md`，架构设计见 `docs/architecture/`，迁移状态见 `docs/architecture/reference/migration.md`。

## 当前目录结构

标注说明：

- `[当前]` — 已存在且活跃在主代码路径中
- `[规划中]` — 架构设计中定义但尚未实现（状态见 migration.md）

### Rust 后端 (`src-tauri/src/`)

```text
src-tauri/src/
├── lib.rs                              # Tauri Builder：插件/命令注册、managed state 注入
├── main.rs                             # Rust 二进制入口
├── error.rs                            # AppError 定义（实现 Serialize）
├── commands/                           # [当前] Tauri Command 层（thin）
│   └── mod.rs                          #   Thin wrapper，调用 Services，返回 Result<T, AppError>
├── services/                           # [当前] Service 层（thick，业务逻辑；不 use tauri::*）
│   ├── mod.rs                          #   模块声明
│   ├── core/                           # [当前] 跨层共享核心类型
│   │   └── mod.rs                      #   ChannelMessage、AgentResponse、ResponseStatus
│   ├── gateway/                        # [当前] GatewaySupervisor 与运行时错误边界
│   │   ├── mod.rs
│   │   ├── supervisor.rs               #   App 内驻留业务主管组件（GatewaySupervisor）
│   │   ├── error.rs                    #   GatewayError
│   │   ├── api.rs                      #   [规划中] GatewayAPIAdapter — 统一入口适配（migration.md Phase 6）
│   │   └── health.rs                   #   [规划中] Health check / 运行状态查询（migration.md Phase 6）
│   ├── channels/                       # [当前] Channel Gateway 抽象层
│   │   ├── mod.rs                      #   Channel trait + CredentialsManager trait
│   │   ├── registry.rs                 #   ChannelRegistry — 渠道注册与查找
│   │   ├── inbound.rs                  #   InboundDriver + InboundChannel trait
│   │   ├── manager.rs                  #   [规划中] ChannelManager — 生命周期/健康/出站分发（migration.md Phase 7）
│   │   ├── feishu/                     # [当前] 飞书渠道实现
│   │   │   ├── mod.rs
│   │   │   ├── client.rs               #   FeishuClient（实现 Channel trait）
│   │   │   ├── converter.rs            #   Feishu payload → ChannelMessage
│   │   │   └── token.rs                #   Feishu 凭证管理
│   │   └── telegram/                   # [当前] Telegram 渠道实现
│   │       ├── mod.rs
│   │       ├── client.rs               #   TelegramClient（实现 Channel trait）
│   │       ├── converter.rs
│   │       └── token.rs                #   Telegram 凭证管理
│   ├── agent/                          # [当前] Agent Control Layer — 用户侧执行模型
│   │   ├── mod.rs
│   │   ├── types.rs                    #   Agent、Identity、Skill、SlashCommand、ExecutionContext
│   │   ├── store.rs                    #   AgentStore facade + AgentDataStore
│   │   ├── skill_store.rs              #   SkillStore（AgentSkillBinding 定义在 types.rs）
│   │   ├── command_store.rs            #   CommandStore
│   │   ├── state_store.rs              #   ConversationStateStore（SQLite + 内存 fallback）
│   │   ├── command_parser.rs           #   SlashCommandParser
│   │   └── resolver.rs                 #   AgentResolver
│   ├── agent_context/                  # [当前] Agent 上下文组装层
│   │   ├── mod.rs                      #   AgentContextBuilder / build_prompt
│   │   ├── memory.rs                   #   MemorySource trait + NoopMemory
│   │   ├── prompt_builder.rs           #   [规划中] PromptBuilder — 结构化 prompt 组装（migration.md Phase 5）
│   │   └── tool_registry.rs            #   [规划中] ToolRegistry — 本地工具元数据注册（migration.md Phase 5）
│   ├── acp_client/                     # [当前] ACP Execution Layer
│   │   ├── mod.rs
│   │   ├── client.rs                   #   AcpClient — ACP 调用入口
│   │   ├── server.rs                   #   AcpServer、AcpServerStatus
│   │   ├── registry.rs                 #   AcpServerRegistry
│   │   ├── transport.rs                #   Transport trait
│   │   ├── tool_executor.rs            #   ToolExecutor trait + NoopToolExecutor
│   │   ├── session.rs                  #   [规划中] SessionManager — ACP session 生命周期（migration.md Phase 4）
│   │   ├── protocol.rs                 #   [规划中] ACP 协议帧编解码（migration.md Phase 4）
│   │   └── content.rs                  #   [规划中] 请求/响应内容类型（migration.md Phase 4）
│   ├── session_dispatcher/             # [当前] 会话调度器
│   │   ├── mod.rs                      #   SessionDispatcher：per-session 队列 + lazy worker 启动
│   │   ├── types.rs                    #   DispatchCommand 类型
│   │   ├── worker.rs                   #   session_worker + process_message + 重试编排
│   │   └── retry.rs                    #   RetryPolicy
│   ├── event_bus/                      # [当前] Runtime 事件 Pub/Sub
│   │   ├── mod.rs                      #   EventBus（基于 tokio::sync::broadcast）
│   │   └── types.rs                    #   RuntimeEvent 枚举
├── storage/                            # [当前] Storage 层（thin）
│   └── mod.rs                          #   MessageStore（SQLite 去重 + 内存消息缓存）
└── config/                             # [当前] 配置管理
    └── mod.rs                          #   AppConfig
```

### 前端 (`src/`)

```text
src/                                     # [当前] React 19 + TypeScript + Vite 前端
├── App.tsx                              # 根组件 — Desktop UI 控制台
├── main.tsx                             # 前端入口
├── App.css
├── index.css
├── vite-env.d.ts
├── components/
│   ├── ChannelSettings.tsx              # 渠道设置组件
│   ├── FeishuSettings.tsx               # 飞书设置组件
│   ├── MessageList.tsx                  # 消息列表组件
│   ├── settings/                        # [规划中] 设置相关组件（后续按功能拆分）
│   ├── messages/                        # [规划中] 消息相关组件（后续按功能拆分）
│   ├── runtime/                         # [规划中] 运行时状态组件（后续按功能拆分）
│   └── ui/                              # [当前] shadcn/ui 生成组件（Base UI 基座，非 Radix）
│       ├── button.tsx
│       ├── card.tsx
│       ├── command.tsx
│       ├── context-menu.tsx
│       ├── dialog.tsx
│       ├── input-group.tsx
│       ├── input.tsx
│       ├── kbd.tsx
│       ├── label.tsx
│       ├── resizable.tsx
│       ├── scroll-area.tsx
│       ├── separator.tsx
│       ├── sheet.tsx
│       ├── sidebar.tsx
│       ├── skeleton.tsx
│       ├── textarea.tsx
│       ├── toggle-group.tsx
│       ├── toggle.tsx
│       └── tooltip.tsx
├── hooks/
│   └── use-mobile.ts
├── lib/
│   ├── types.ts
│   └── utils.ts
├── routes/                              # [规划中] TanStack Router 路由拆分
└── stores/
    └── message-store.ts                 # Zustand UI 状态
```

## 目录与架构层映射

| 架构层 | 当前目录 | 规划中模块 | 说明 |
|--------|----------|-----------|------|
| **Channel Gateway Layer** | `services/gateway/` | `api.rs`（GatewayAPIAdapter）、`health.rs` | GatewaySupervisor、运行时错误边界、统一入口适配 |
| | `services/channels/` | `manager.rs`（ChannelManager） | Channel trait、ChannelRegistry、InboundDriver 抽象 |
| | `services/channels/feishu`、`services/channels/telegram` | `manager.rs`（ChannelManager） | 具体渠道实现：Feishu、Telegram |
| **Agent Control Layer** | `services/agent/` | — | Agent、Skill、SlashCommand、AgentResolver、ConversationState |
| | `services/session_dispatcher/` | — | 会话调度、per-session 保序、ACP 编排 |
| | `services/event_bus/` | — | Runtime 事件 Pub/Sub |
| **ACP Execution Layer** | `services/agent_context/` | `prompt_builder.rs`、`tool_registry.rs` | prompt/context 组装、MemorySource、ToolRegistry |
| | `services/acp_client/` | `session.rs`、`protocol.rs`、`content.rs` | ACP 协议客户端、Transport、ToolExecutor |
| **Shared / Storage** | `services/core/`、`storage/`、`config/` | — | 共享类型、SQLite、配置 |
| **UI / IPC** | `src/`、`commands/`、`lib.rs` | `src/routes/`、`src/components/settings/`、`src/components/messages/`、`src/components/runtime/` | Desktop UI、Tauri commands |

## 目录用途说明

### Rust 后端

#### 入口与 IPC

| 目录 | 用途 |
|------|------|
| `lib.rs` | Tauri Builder：插件/command 注册、managed state 注入 |
| `main.rs` | Rust 二进制入口 |
| `error.rs` | `AppError` 定义，实现 `Serialize` |
| `commands/` | Tauri Command thin wrappers，调用 Services，返回 `Result<T, AppError>` |

#### Channel Gateway Layer

| 目录 | 用途 |
|------|------|
| `services/gateway/` | GatewaySupervisor：App 内驻留业务主管组件，组装/启动/停止子模块；GatewayError；（规划中）GatewayAPIAdapter、health check |
| `services/channels/` | Channel trait、CredentialsManager trait、ChannelRegistry、InboundDriver 抽象；feishu、telegram 渠道实现；（规划中）ChannelManager |
| `services/channels/feishu/` | 飞书渠道实现：FeishuClient（实现 Channel trait）、消息转换器、凭证管理 |
| `services/channels/telegram/` | Telegram 渠道实现：TelegramClient（实现 Channel trait）、消息转换器、凭证管理 |

#### Agent Control Layer

| 目录 | 用途 |
|------|------|
| `services/agent/` | Agent/Identity/Skill/SlashCommand 类型定义、AgentStore/SkillStore/CommandStore/ConversationStateStore 持久化、SlashCommandParser、AgentResolver |
| `services/session_dispatcher/` | per-session 队列 + FIFO worker、SlashCommand 入口、ACP 调用编排、重试策略（types/worker/retry） |
| `services/event_bus/` | 基于 `tokio::sync::broadcast` 的 EventBus、RuntimeEvent 枚举 |

#### ACP Execution Layer

| 目录 | 用途 |
|------|------|
| `services/agent_context/` | AgentContextBuilder：Identity/Skill instruction/用户消息组装；MemorySource trait；（规划中）PromptBuilder、ToolRegistry |
| `services/acp_client/` | AcpClient：ACP 协议调用入口；AcpServerRegistry；Transport trait；ToolExecutor；（规划中）SessionManager、protocol 编解码 |

#### Shared / Storage

| 目录 | 用途 |
|------|------|
| `services/core/` | 跨层共享类型：`ChannelMessage`、`AgentResponse`、`ResponseStatus` |
| `storage/` | MessageStore：SQLite 去重 + 内存消息缓存 |
| `config/` | AppConfig 配置管理 |

### 前端

| 目录 | 用途 |
|------|------|
| `src/App.tsx` | 根 React 组件 |
| `src/main.tsx` | 前端入口 |
| `src/components/` | 业务组件：ChannelSettings、FeishuSettings、MessageList；（规划中）按 settings/messages/runtime 拆分 |
| `src/components/ui/` | shadcn/ui 生成组件（Base UI 基座，非 Radix） |
| `src/hooks/` | 自定义 React hooks |
| `src/lib/` | 前端共享类型（`types.ts`）和工具函数（`utils.ts`） |
| `src/stores/` | Zustand UI 状态（如 `message-store.ts`） |
| `src/routes/` | （规划中）TanStack Router 路由拆分 |

## 维护规则

- 新增、移动、删除代码目录时同步更新本文档的树形图。
- 新增规划中的模块时，在树形图中以 `[规划中]` 标注，并在"目录用途说明"中添加条目。
- 模块从 `[规划中]` 变为已实现时，移除 `[规划中]` 标注，更新 migration.md 对应 Phase 状态。
- 架构文档（00-overview.md 及子模块文档）中的模块描述与本文档的目录职责描述保持一致。
- 前端组件拆分完成后，在树形图中展开 `src/components/` 的子目录。
