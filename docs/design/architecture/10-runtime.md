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
├── agent: Arc<AgentLoop>               # Agent 循环
├── session_mgr: Arc<SessionManager>    # 会话管理
├── config: Arc<AppConfig>              # 配置
└── shutdown: CancellationToken         # 关停信号
```

### AppRuntimeBuilder

Builder 模式支持入口点特定配置：

```rust
let rt = AppRuntime::builder()
    .provider("deepseek")
    .observer(Arc::new(MyObserver))
    .build().await?;
```

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
    ├── ServiceContainer
    ├── AgentLoop
    ├── MessageBus
    └── SessionManager
         │
         ▼
基础设施层 (Storage/Provider/Tools)
```

## 关键设计

| 决策              | 说明                                                 |
| ----------------- | ---------------------------------------------------- |
| Tauri 无关        | Runtime 不依赖 `tauri::*`，可被 CLI/Gateway 独立使用 |
| Builder 模式      | 支持入口点特定配置（provider、observer 等）          |
| CancellationToken | tokio-util 实现优雅关停                              |
| ServiceContainer  | 具体 struct 而非 trait，YAGNI                        |

## 状态迁移

**重构前**：

- `CliRuntime` 包含 80+ 行初始化逻辑，Tauri 无运行时注入

**重构后**：

- 初始化逻辑提取到 `AppRuntimeBuilder`
- CLI 直接使用 `AppRuntime`
- Tauri 通过 State 注入
- Gateway 复用同一套代码
