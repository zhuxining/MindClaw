> **Status**: `active`

# 代码目录结构

> 本文档描述当前代码库的目录现状，随代码变化同步更新，不记录设计意图。

---

## Rust 后端（`src-tauri/src/`）

### Agent 模块当前收敛范围

`agent/` 当前采用“**单文件优先，tools 保持目录**”的组织方式。

当前已经收敛为单文件的边界：

- `agent.rs`：AgentProfile / AgentRegistry / ModelRouter
- `loop_.rs`：AgentLoop 编排层，同时内聚 `/new /stop /restart /status` 等 loop 控制命令
- `runner.rs`：AgentRunner 执行层
- `hooks.rs`：RunHooks 与交互式 Hook 实现
- `spawn.rs`：派生执行与后台代理调度
- `session.rs`：会话管理
- `context.rs`：上下文装配
- `events.rs`：运行事件
- `spec.rs`：run 契约
- `memory.rs`：记忆数据结构与召回入口
- `skills.rs`：技能清单、元数据与注册表
- `observability.rs`：观测接口与 tracing 实现

当前**仍保留目录**的范围：

- `tools/`：按规划继续保留目录

因此，这里的目录结构已经是最新现状。

---

```
src/
├── lib.rs                  Tauri App 入口，注册插件和命令
├── main.rs                 桌面应用二进制入口
├── error.rs                统一错误类型 AppError（实现 Serialize）
│
├── runtime/                统一运行时
│   ├── mod.rs              AppRuntime 核心结构（含 AgentLoop / AgentRegistry / ModelRouter 注入结果）
│   ├── builder.rs          AppRuntimeBuilder
│   ├── config.rs           AppConfig
│   └── services.rs         ServiceContainer
│
├── agent/                  Agent 核心系统
│   ├── mod.rs              模块导出
│   ├── agent.rs            AgentProfile / AgentKind / AgentRegistry / ModelRouter
│   ├── loop_.rs            AgentLoop（消息编排、命令路由、session 串行化、/new /stop /restart /status）
│   ├── runner.rs           AgentRunner（LLM 迭代循环与工具执行驱动）
│   ├── spec.rs             AgentRunSpec / AgentRunResult / StopReason / TokenUsage
│   ├── context.rs          ContextPipeline / ContextSource / BuiltContext
│   ├── session.rs          SessionManager / AgentSession / TurnRecord
│   ├── events.rs           ProviderEvent / AgentEvent / UsageStats
│   ├── hooks.rs            RunHooks / InteractiveRunHooks / NoopRunHooks / RecordingRunHooks
│   ├── spawn.rs            AgentSpawnDispatcher / SubAgent 定义 / 后台派发
│   ├── memory.rs           Memory / MemoryCategory / recall
│   ├── skills.rs           SkillManifest / SkillMetadata / SkillsRegistry
│   ├── observability.rs    AgentObserver / TracingObserver / CompositeObserver
│   ├── tools/              工具系统
│   │   ├── mod.rs          ToolRegistry
│   │   ├── traits.rs       Tool trait
│   │   ├── path_guard.rs   PathGuard 路径沙箱
│   │   ├── shell.rs        Shell 工具
│   │   ├── file_ops.rs     文件读取 + 写入 + 编辑
│   │   ├── find_files.rs   文件搜索（Glob）
│   │   ├── search_content.rs 内容搜索（正则）
│   │   ├── agent_spawn.rs  同步子代理入口 + 后台代理入口
│   │   └── mcp.rs          MCP external tool bridge（当前仍注册为 Tool）
│   │
│
├── bus/                    消息总线
│   ├── mod.rs              MessageBus
│   └── events.rs           InboundMessage / OutboundMessage
│
├── channels/               通道层
│   ├── mod.rs
│   ├── traits.rs           Channel trait
│   ├── desktop.rs          Tauri Desktop
│   ├── telegram.rs         Telegram Bot
│   └── feishu.rs           飞书 Bot
│
├── providers/              LLM 适配层
│   ├── mod.rs
│   ├── traits.rs           Provider trait / ChatMessage / ProviderEvent
│   ├── config.rs           ModelConfig / ProviderConfig
│   ├── registry.rs         ProviderRegistry
│   ├── claude.rs           Anthropic Claude
│   └── openai_compat.rs    OpenAI 兼容适配器
│
├── models/                 数据模型（跨层共享）
│   ├── mod.rs
│   ├── task.rs             Task / TaskStatus
│   ├── note.rs             Note / DailyNote / KnowledgeEntry
│   ├── conversation.rs     ConversationMode
│   └── settings.rs         AppSettings
│
├── storage/                存储层
│   ├── mod.rs
│   ├── database.rs         SQLite 初始化和迁移
│   ├── markdown.rs         Markdown 文件 I/O
│   ├── vector.rs           向量数据库
│   ├── archive.rs          历史归档（JSONL）
│   └── keychain.rs         OS Keychain
│
├── services/               业务服务层
│   ├── mod.rs
│   ├── task.rs             TaskService
│   ├── knowledge.rs        KnowledgeService
│   └── daily.rs            DailyService
│
├── commands/               Tauri IPC 命令（薄层）
│   ├── mod.rs
│   ├── conversation.rs     send_message / get_session_history
│   ├── tasks.rs            list / create / update
│   ├── knowledge.rs        search / get
│   ├── daily.rs            get / save
│   ├── settings.rs         get / save / set_api_key
│   └── system.rs           get_system_status
│
├── gateway/                HTTP/WebSocket 网关
│   ├── mod.rs
│   ├── server.rs           HTTP 服务器
│   ├── api.rs              REST API 路由
│   ├── ws.rs               WebSocket
│   └── auth.rs             Bearer Token 认证
│
├── cron/                   定时任务
│   ├── mod.rs
│   ├── scheduler.rs        任务调度器
│   └── jobs.rs             具体定时任务（daily_summary 等）
│
├── heartbeat/              健康监测
│   └── mod.rs
│
└── bin/
    └── cli.rs              CLI 二进制入口
```

---

## 前端（`src/`）

```
src/
├── App.tsx                 根组件，路由配置
├── main.tsx                Vite 入口
├── components/
│   └── ui/                 shadcn/ui 生成的组件
├── routes/                 TanStack Router 页面
├── stores/                 Zustand 状态
└── hooks/                  自定义 Hooks
```

---

## 配置文件

```
src-tauri/
├── tauri.conf.json         Tauri 应用配置（标识符、窗口、Bundle）
├── capabilities/           Tauri 2.0 权限声明
├── Cargo.toml              Rust 依赖
└── build.rs                构建脚本

docs/design/architecture/   架构设计文档
docs/design/architecture/reference/  参考文档（本目录）
```
