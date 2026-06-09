# 目录结构

> 本文档记录代码目录结构，代码重组时同步更新。

```
src-tauri/src/
├── lib.rs                         # Tauri Builder：插件/命令注册
├── main.rs                        # Rust 二进制入口
├── error.rs                       # AppError 定义
├── commands/                      # Tauri Command 层（thin）
│   └── mod.rs
├── services/                      # Service 层（thick，业务逻辑）
│   ├── mod.rs
│   ├── im_channel/                # IM 渠道协议适配层（新增）
│   │   ├── mod.rs                 # ChannelAdapter + CredentialsManager trait
│   │   ├── feishu/                # 飞书（ChannelAdapter 实现）
│   │   │   ├── mod.rs
│   │   │   ├── client.rs          # HTTP 客户端（飞书 Open API）
│   │   │   ├── token.rs           # Token 管理（实现 CredentialsManager）
│   │   │   └── converter.rs       # 飞书消息 → RawMessage 转换
│   │   └── telegram/              # Telegram（ChannelAdapter 实现）
│   │       ├── mod.rs
│   │       ├── client.rs          # HTTP 客户端（Telegram Bot API）
│   │       ├── token.rs           # Bot Token 管理（实现 CredentialsManager）
│   │       └── converter.rs       # Telegram Update → RawMessage 转换
│   ├── gateway/                   # 网关层（新增）
│   │   ├── mod.rs                 # Gateway trait + GatewayRegistry
│   │   ├── registry.rs            # GatewayRegistry 渠道注册中心
│   │   ├── auth.rs                # AuthFilter 身份鉴权
│   │   ├── rate_limiter.rs        # RateLimiter 流量控制
│   │   ├── transformer.rs         # Transformer 消息标准化
│   │   └── error.rs               # GatewayError 错误类型
│   ├── message_bus/               # 消息总线路由层
│   │   ├── mod.rs
│   │   ├── router.rs              # 路由引擎（RouteRule 匹配）
│   │   ├── topic.rs               # Topic 消息分发
│   │   ├── subscription.rs        # SubscriptionMgr 订阅管理
│   │   └── types.rs               # ChannelMessage, RouteRule, BusMessage 等类型
│   └── acp/                       # Action Control Plane 执行层（新增）
│       ├── mod.rs
│       ├── router.rs              # 意图识别路由
│       ├── planner.rs             # 任务规划
│       ├── executor.rs            # 动作执行
│       ├── memory.rs              # 记忆管理
│       ├── protocol.rs            # ACP 协议实现
│       └── types.rs               # AgentRequest, AgentResponse 等类型
├── storage/                       # Storage 层（thin）
│   └── mod.rs
└── config/                        # 配置管理
    └── mod.rs

src/
├── App.tsx                        # 根组件
├── main.tsx                       # 入口
├── components/
│   └── ui/                        # shadcn/ui 组件
├── hooks/                         # 自定义 hooks
├── lib/                           # 工具函数
├── routes/                        # TanStack Router 路由
│   ├── __root.tsx
│   ├── index.tsx                  # 首页
│   └── messages.tsx               # 消息流页面（新增）
└── stores/                        # Zustand stores
    └── message-store.ts           # 消息状态（新增）
```
