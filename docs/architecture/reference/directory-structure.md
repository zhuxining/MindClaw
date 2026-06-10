# 目录结构

> 本文档记录代码目录结构，代码重组时同步更新。

## 目标架构目录（App 内驻留 GatewaySupervisor + Agent 调度）

```
src-tauri/src/
├── lib.rs                         # Tauri Builder：插件/命令注册、managed state 注入
├── main.rs                        # Rust 二进制入口
├── error.rs                       # AppError 定义
├── commands/                      # Tauri Command 层（thin），只连接 GatewaySupervisor API
│   └── mod.rs
├── services/                      # Service 层（thick，业务逻辑；不 use tauri::*）
│   ├── mod.rs
│   ├── gateway/                   # Gateway Runtime 业务边界
│   │   ├── mod.rs
│   │   ├── supervisor.rs          # GatewaySupervisor：App 内驻留业务主管组件
│   │   ├── api.rs                 # Gateway API adapter：commands/local API/webhook 边界
│   │   ├── health.rs              # Health check 与运行状态
│   │   └── error.rs               # GatewayError 错误类型
│   ├── agent/                     # Agent 执行模型
│   │   ├── mod.rs
│   │   ├── types.rs               # Agent、Identity、Skill、SlashCommand、ExecutionContext
│   │   ├── store.rs               # AgentStore、SkillStore、ConversationExecutionStateStore
│   │   ├── command_parser.rs      # SlashCommandParser
│   │   └── resolver.rs            # AgentResolver
│   ├── channels/                  # ChannelManager 与具体渠道实现
│   │   ├── mod.rs                 # Channel trait + CredentialsManager trait
│   │   ├── manager.rs             # ChannelManager：渠道生命周期、凭证代理、入站接收和出站分发
│   │   ├── registry.rs            # ChannelRegistry：渠道注册与查找
│   │   ├── inbound.rs             # InboundDriver：polling/long-polling/stream/webhook/manual
│   │   ├── feishu/                # 飞书渠道实现
│   │   └── telegram/              # Telegram 渠道实现
│   ├── session_dispatcher/        # 会话调度器
│   │   ├── mod.rs                 # SessionDispatcher 对外接口
│   │   ├── types.rs               # DispatchKey、DispatchCommand、RetryPolicy
│   │   └── worker.rs              # per-session worker
│   ├── event_bus/                 # Runtime 事件 Pub/Sub
│   │   ├── mod.rs                 # EventBus
│   │   └── types.rs               # RuntimeEvent
│   ├── message_bus/               # legacy RouteRule 兼容模块
│   │   ├── mod.rs
│   │   ├── router.rs              # MessageBus：legacy RouteRule 管理
│   │   └── types.rs               # ChannelMessage、AgentRequest、AgentResponse、RouteRule
│   ├── agent_context/             # Agent 上下文组装层
│   │   ├── mod.rs
│   │   ├── memory.rs              # 记忆检索与注入
│   │   ├── prompt_builder.rs      # Prompt 组装器
│   │   └── tool_registry.rs       # 本地工具元数据注册表
│   └── acp_client/                # ACP 协议客户端
│       ├── mod.rs
│       ├── client.rs              # AcpClient：协议调用入口
│       ├── server.rs              # AcpServer、AcpServerConfig、AcpServerStatus
│       ├── registry.rs            # AcpServerRegistry
│       ├── transport.rs           # Transport trait + StdioTransport/HttpTransport
│       ├── session.rs             # SessionManager：会话管理
│       ├── protocol.rs            # ACP 协议数据结构
│       ├── tool_executor.rs       # ToolExecutor：本地工具执行
│       └── content.rs             # ContentPart 多模态内容模型
├── storage/                       # Storage 层（thin）
│   └── mod.rs
└── config/                        # 配置管理
    └── mod.rs

src/
├── App.tsx                        # 根组件：Desktop UI 控制台
├── main.tsx                       # 入口
├── components/
│   └── ui/                        # shadcn/ui 组件
├── hooks/                         # 自定义 hooks
├── lib/                           # 工具函数与类型
├── routes/                        # TanStack Router 路由
└── stores/                        # Zustand stores
```

## 当前实现目录（迁移中）

```
src-tauri/src/services/
├── gateway/                       # 当前包含 ChannelGateway、GatewayRegistry、GatewayError；目标为 GatewaySupervisor 业务边界
├── im_channel/                    # 当前包含 feishu、telegram 具体渠道实现；目标迁移到 channels/
├── message_bus/                   # 当前包含 MessageBus、RouteRule、ChannelMessage；目标保留为 legacy 兼容模块
├── event_bus/                     # 当前已新增 RuntimeEvent 与 EventBus
└── acp_client/                    # 当前包含 AcpClient 与 ACP 协议类型
```

当前实现中的 `gateway/` 和 `im_channel/` 会迁移为目标架构中的 `gateway/`（GatewaySupervisor）与 `channels/`（ChannelManager + Channel 实现）。当前实现中的 `RouteRule` 保留为 legacy 兼容接口，自动消息调度改由 `SessionDispatcher` 和 `agent/` 模块解析默认 Agent 或 slash command 后进入对应 ACP Server。
