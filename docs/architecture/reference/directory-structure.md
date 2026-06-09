# 目录结构

> 本文档记录代码目录结构，代码重组时同步更新。

## 目标架构目录（Gateway Runtime + Active ACP Server）

```
src-tauri/src/
├── lib.rs                         # Tauri Builder：插件/命令注册
├── main.rs                        # Rust 二进制入口
├── error.rs                       # AppError 定义
├── commands/                      # Tauri Command 层（thin），只连接 Gateway API
│   └── mod.rs
├── services/                      # Service 层（thick，业务逻辑）
│   ├── mod.rs
│   ├── gateway/                   # Gateway Runtime：本地常驻运行时
│   │   ├── mod.rs
│   │   ├── runtime.rs             # GatewayRuntime：启动/停止/监督内部子模块
│   │   ├── api.rs                 # Gateway API：Desktop UI/CLI/Web UI/Webhook 入口
│   │   ├── active_acp.rs          # ActiveAcpServer：当前激活 ACP Server 选择与状态
│   │   ├── health.rs              # Health check 与运行状态
│   │   ├── supervisor.rs          # 后台进程与任务监督
│   │   └── error.rs               # GatewayError 错误类型
│   ├── channels/                  # ChannelManager 与具体渠道实现
│   │   ├── mod.rs                 # Channel trait + CredentialsManager trait
│   │   ├── manager.rs             # ChannelManager：渠道生命周期与出站分发
│   │   ├── registry.rs            # ChannelRegistry：渠道注册与查找
│   │   ├── feishu/                # 飞书渠道实现
│   │   │   ├── mod.rs
│   │   │   ├── client.rs          # FeishuChannel：飞书 Open API/Webhook 适配
│   │   │   ├── token.rs           # TokenManager：飞书 Token 管理
│   │   │   └── converter.rs       # 飞书消息 → ChannelMessage 转换
│   │   └── telegram/              # Telegram 渠道实现
│   │       ├── mod.rs
│   │       ├── client.rs          # TelegramChannel：Telegram Bot API 适配
│   │       ├── token.rs           # TelegramTokenManager：Bot Token 管理
│   │       └── converter.rs       # Telegram Update → ChannelMessage 转换
│   ├── message_bus/               # 消息总线层（无 RouteRule）
│   │   ├── mod.rs
│   │   ├── queue.rs               # inbound/outbound 消息队列
│   │   ├── topic.rs               # Topic 消息分发
│   │   ├── subscription.rs        # SubscriptionMgr 订阅管理
│   │   └── types.rs               # ChannelMessage, BusMessage 等类型
│   ├── agent_context/             # Agent 上下文组装层
│   │   ├── mod.rs
│   │   ├── identity.rs            # Agent 身份证管理
│   │   ├── memory.rs              # 记忆检索与注入
│   │   ├── prompt_builder.rs      # Prompt 组装器
│   │   └── tool_registry.rs       # 本地工具元数据注册表
│   └── acp_client/                # ACP 协议客户端
│       ├── mod.rs
│       ├── client.rs              # AcpClient：连接管理、生命周期
│       ├── transport.rs           # Transport trait + StdioTransport/HttpTransport
│       ├── session.rs             # SessionManager：会话管理
│       ├── protocol.rs            # ACP 协议数据结构
│       ├── tool_executor.rs       # ToolExecutor：本地工具执行
│       ├── tools/                 # 内置本地工具集
│       │   ├── mod.rs
│       │   ├── fs.rs              # File System 工具
│       │   └── terminal.rs        # Terminal 工具
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
├── lib/                           # 工具函数
├── routes/                        # TanStack Router 路由
│   ├── __root.tsx
│   ├── index.tsx                  # 首页
│   └── messages.tsx               # 消息流页面
└── stores/                        # Zustand stores
    └── message-store.ts           # 消息状态
```

## 当前实现目录（待迁移）

```
src-tauri/src/services/
├── gateway/                       # 当前仅包含 ChannelGateway、GatewayRegistry、GatewayError
├── im_channel/                    # 当前包含 feishu、telegram 具体渠道实现
├── message_bus/                   # 当前包含 MessageBus、RouteRule、ChannelMessage
└── acp_client/                    # 当前包含 AcpClient 与 ACP 协议类型
```

当前实现中的 `gateway/` 和 `im_channel/` 会迁移为目标架构中的 `gateway/`（Runtime）与 `channels/`（ChannelManager + Channel 实现）。当前实现中的 `RouteRule` 会移除，自动消息改为直接进入 Active ACP Server。
