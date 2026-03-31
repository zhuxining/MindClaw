# MindClaw 技术架构设计 — Runtime 模块

> 完整架构文档索引见 [README.md](./README.md)

Runtime 模块提供**统一的、Tauri 无关的应用运行时**，供 Desktop、CLI、Gateway 三个入口共享。

## 模块结构

```
src-tauri/src/runtime/
├── mod.rs          # AppRuntime: 核心运行时 + 生命周期
├── builder.rs      # AppRuntimeBuilder: 11 步初始化
├── config.rs       # AppConfig: 配置集中管理
└── services.rs     # ServiceContainer: 服务统一封装
```

## 核心组件

### AppRuntime

持有所有共享基础设施，提供 `build → start → shutdown` 生命周期：

```
AppRuntime
├── db: Arc<Mutex<Connection>>          # SQLite
├── services: Arc<ServiceContainer>     # Knowledge/Daily/Task
├── bus: Arc<MessageBus>                # 消息总线
├── agent_loop: Arc<AgentLoop>          # AgentLoop（驱动器）
│     ├── agent: Agent                  #   Agent（大脑）
│     ├── bus (共享 Arc)                #   消息流
│     ├── session_mgr (共享 Arc)        #   会话编排
│     ├── commands                      #   命令拦截器
│     └── observer                      #   观测
├── session_mgr: Arc<SessionManager>    # 会话管理
├── config: Arc<AppConfig>              # 配置
└── shutdown: CancellationToken         # 关停信号
```

`bus` 和 `session_mgr` 在 AppRuntime 和 AgentLoop 中都有 Arc 引用，职责明确：
- **AppRuntime** 持有是因为外部入口（Tauri commands、CLI）需要直接访问
- **AgentLoop** 持有是因为内部消息驱动和会话编排需要

### AppRuntimeBuilder

Builder 模式，线性构建流程：

```rust
let rt = AppRuntime::builder()
    .provider("deepseek")
    .build().await?;
```

构建顺序：

1. 组装 `AppConfig`
2. 初始化 db
3. 创建 `ServiceContainer`（依赖 db）
4. 创建 `SessionManager`（依赖 db）
5. 创建 `MessageBus`
6. **AgentBuilder** 构建 `Agent`（只需 config，不需要 bus/session_mgr）
7. 组装 `AgentLoop`（组合 Agent + bus + session_mgr + commands + observer）
8. 组装 `AppRuntime`

Agent 和 AgentLoop 的构建职责分离：
- **AgentBuilder** 只构建大脑（Provider、ToolRegistry、ContextPipeline、Observer）
- **AppRuntimeBuilder** 构建基础设施 + 用 AgentBuilder 构建 Agent + 组装 AgentLoop

### ServiceContainer

统一封装业务服务，解决 Services 定义但未初始化的问题：

```rust
pub struct ServiceContainer {
    pub knowledge: Arc<KnowledgeService>,
    pub daily: Arc<DailyService>,
    pub task: Arc<TaskService>,
}
```

## 入口点集成

### Tauri Desktop

```rust
.setup(|app| {
    tauri::async_runtime::spawn(async move {
        let rt = Arc::new(AppRuntime::builder().build().await?);
        rt.start().await?;
        app.handle().manage(rt);
    });
})
```

Commands 通过 `State<Arc<AppRuntime>>` 访问服务。

### CLI Binary

直接使用，CLI 特有逻辑（tracing init、REPL）内联在入口：

```rust
let rt = Arc::new(AppRuntime::builder().build().await?);
rt.start().await?;
// ... CLI I/O ...
rt.shutdown().await;
```

### Gateway HTTP (计划)

```rust
let rt = Arc::new(AppRuntime::builder().build().await?);
rt.start().await?;
// Axum handler: State<Arc<AppRuntime>>
```

## 架构层级

```
入口点层 (Desktop/CLI/Gateway)
         │
         ▼
Runtime 层 (AppRuntime)
    ├── ServiceContainer (db)
    ├── SessionManager (db)
    ├── MessageBus
    └── AgentLoop（驱动器）
         ├── Agent（大脑）
         │    ├── ContextPipeline
         │    ├── Provider
         │    ├── ToolRegistry
         │    └── Observer
         ├── Commands
         └── Observer (共享)
         │
         ▼
基础设施层 (Storage/Provider/Tools)
```

### 依赖图

```
db ─────────┬── SessionManager
            └── ServiceContainer

AppConfig ──── AgentBuilder ──► Agent（无基础设施依赖）

Agent + bus + session_mgr + commands + observer ──► AgentLoop

db + services + bus + agent_loop + session_mgr + config ──► AppRuntime
```

## 关键设计

| 决策              | 说明                                                         |
| ----------------- | ------------------------------------------------------------ |
| Tauri 无关        | Runtime 不依赖 `tauri::*`，可被 CLI/Gateway 独立使用         |
| Agent/Loop 分离   | Agent 无状态可共享，AgentLoop 负责编排，职责不混              |
| 两级 Builder      | AgentBuilder 只建大脑，AppRuntimeBuilder 建基础设施 + 组装   |
| Observer 共享     | Agent 和 AgentLoop 持有同一个 Arc，各发射所属层事件          |
| Commands 归 Loop  | `/new`, `/stop` 等拦截器在 Context 构建前短路，属于编排层     |
| CancellationToken | tokio-util 实现优雅关停                                      |
| ServiceContainer  | 具体 struct 而非 trait，YAGNI                                |
