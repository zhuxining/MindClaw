> **Status**: `active`

# Runtime — 统一运行时

---

## § 职责定位

AppRuntime 负责统一初始化序列、依赖注入和三入口（Desktop、CLI、Gateway）的适配，不负责任何具体业务逻辑或数据处理。

---

## § 核心原则

**单一启动路径**：无论桌面应用、CLI 还是 Gateway 入口，均通过同一 `AppRuntimeBuilder` 初始化，确保组件版本、配置和状态一致。

**依赖方向单向**：入口层持有 AppRuntime，AppRuntime 持有 ServiceContainer，ServiceContainer 持有 Services，Services 持有 Storage。反向引用（Storage → Services）被禁止。

---

## § 边界与实体

**输入**：`AppConfig` — 从配置文件和环境变量加载的全局配置，包含数据库路径、vault 路径、通道配置、LLM 模型选择等。

**输出**：完整初始化的 AppRuntime 实例，暴露以下访问器：

- `services()` → `Arc<ServiceContainer>`（供命令层调用 Service）
- `agent_loop()` → `Arc<AgentLoop>`（供测试和监控）
- `bus()` → `Arc<MessageBus>`（供 Channel 发布消息）

**核心实体**：

**AppRuntime**：系统所有核心组件的所有者，持有组件的强引用，提供统一的启停接口。
关键属性：运行模式（Desktop / CLI / Gateway）、生命周期状态（Initializing / Running / Stopping）。
关系：由 AppRuntimeBuilder 构建，由各入口（`lib.rs`、`cli.rs`、`gateway`）持有。

**ServiceContainer**：业务服务的依赖注入容器，以 Arc 包装的无状态服务集合。
关键属性：TaskService、KnowledgeService、DailyService 各一个实例。
关系：由 AppRuntime 创建并持有；Tauri 命令、CLI 命令、Gateway API 通过 AppRuntime 访问，不直接持有 Container。

**AppConfig**：全局静态配置，在启动时加载，运行期间不可变。
关键属性：数据目录路径、vault 路径、最大并发请求数、已启用的通道列表。
关系：由 AppRuntimeBuilder 接收，分发给各组件初始化时使用。

---

## § 三入口适配

```
lib.rs (Tauri Desktop)          cli.rs (CLI Binary)          gateway/server.rs (HTTP)
        │                               │                              │
        └───────────────────────────────┼──────────────────────────────┘
                                        │
                              AppRuntimeBuilder::new(config)
                                        │
                                  runtime.start()
                                        │
                    ┌───────────────────┼───────────────────┐
                    ▼                   ▼                   ▼
              AgentLoop            ChannelManager       HTTP Server
             (后台任务)            (各通道监听器)       (REST + WS)
```

**Desktop 入口**：注册 Tauri 命令和插件，将 AppRuntime 以 `State<AppRuntime>` 注入 Tauri 应用，命令层通过 `state.services()` 访问 Service。

**CLI 入口**：解析命令行参数，直接调用对应 Service 方法或构建单次 AgentRunSpec 调用 AgentRunner，不启动 ChannelManager。

**Gateway 入口**：启动 HTTP 服务器，REST API 处理器通过 AppRuntime 访问 Service 和 AgentLoop，WebSocket 处理器直接向 MessageBus 发布消息。

---

## § 关键流程

**启动序列**：

1. 入口读取配置文件和环境变量，构建 `AppConfig`。
2. `AppRuntimeBuilder::new(config)` 初始化 Storage 层：执行 SQLite 迁移，初始化向量数据库。
3. Builder 初始化 ServiceContainer：创建 TaskService、KnowledgeService、DailyService，注入 Storage 引用。
4. Builder 初始化 ProviderRegistry：从 OS Keychain 读取 API Key，创建 Provider 实例。
5. Builder 初始化 ToolRegistry：注册内置工具，加载 MCP 配置（延迟连接）。
6. Builder 初始化 AgentLoop：注入 MessageBus、SessionManager、ContextPipeline、AgentRunner。
7. Builder 初始化 ChannelManager：按 `channels.yaml` 配置创建 Channel 实例。
8. `builder.build()` 返回 AppRuntime，所有组件引用已就绪。
9. 入口调用 `runtime.start()`：AgentLoop 后台任务启动，ChannelManager 各通道开始监听。

**停止序列**：Channel 停止接收 → AgentLoop 排空队列后停止 → Storage 层关闭连接。

---

## § 设计决策与权衡

| 决策问题 | 选择 | 放弃的替代方案 | 理由 |
|---------|------|--------------|------|
| 三个入口如何共享组件？ | ServiceContainer 以 Arc 共享，通过 AppRuntime 访问 | 每个入口独立构建服务实例 | 三个入口访问同一 SQLite 文件和 vault，共享实例避免重复初始化和状态不一致 |
| 初始化顺序如何保证？ | Builder 模式，手动按依赖顺序调用初始化方法 | 依赖注入框架自动解析（如 shaku） | 依赖顺序清晰可见，无运行时解析开销；MindClaw 依赖图稳定，无需框架的灵活性 |
| Tauri 命令如何访问 Runtime？ | `State<AppRuntime>` Tauri 依赖注入 | 全局静态 `OnceCell<AppRuntime>` | Tauri State 类型安全，自动处理多线程访问；静态变量难以在测试中替换和重置 |
| CLI 入口是否启动 AgentLoop？ | 不启动；直接调用 Service 或单次 AgentRunner | 与 Desktop 完全相同的启动序列 | CLI 不需要持续监听消息；轻量启动序列减少 CLI 命令的启动延迟 |
| Gateway 入口的认证在哪里处理？ | Gateway 层（Bearer Token 验证），不进入 AppRuntime | AppRuntime 统一认证 | Gateway 是可选组件；认证逻辑属于 Gateway 边界，不应污染核心 Runtime |
| 如何处理启动失败？ | Builder 返回 `Result<AppRuntime, AppError>` | panic 或日志后继续 | 显式错误使调用方（入口）可以决定如何处理（如显示错误界面或退出码）；panic 难以测试和恢复 |
| Runtime 的生命周期如何管理？ | AppRuntime 持有所有权，入口调用 `start()` / `shutdown()` | 全局静态实例 | 显式生命周期管理支持优雅关闭和资源清理；静态实例难以控制关闭顺序和测试替换 |
