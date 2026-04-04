> **Status**: `active`

# 代码目录结构

> 本文档描述当前代码库的目录现状，随代码变化同步更新，不记录设计意图。

---

## Rust 后端（`src-tauri/src/`）

```
src/
├── lib.rs                  Tauri App 入口，注册插件和命令
├── main.rs                 桌面应用二进制入口
├── error.rs                统一错误类型 AppError（实现 Serialize）
│
├── runtime/                统一运行时
│   ├── mod.rs              AppRuntime 核心结构
│   ├── builder.rs          AppRuntimeBuilder
│   ├── config.rs           AppConfig
│   └── services.rs         ServiceContainer
│
├── agent/                  Agent 核心系统
│   ├── mod.rs              模块导出
│   ├── agent_loop.rs       AgentLoop（业务编排层）
│   ├── runner.rs           AgentRunner（纯执行层）
│   ├── builder.rs          AgentBuilder
│   ├── spec.rs             AgentRunSpec / AgentRunResult
│   ├── context.rs          ContextPipeline / ContextSource
│   ├── session.rs          SessionManager / AgentSession
│   ├── events.rs           ProviderEvent / AgentEvent
│   ├── hook.rs             AgentHook trait / LoopHook / NoOpHook
│   ├── observer.rs         AgentObserver（日志/指标）
│   │
│   ├── commands/           Agent 控制指令
│   │   ├── mod.rs          AgentCommandRegistry
│   │   ├── new.rs          /new 新建会话
│   │   ├── stop.rs         /stop 停止
│   │   ├── restart.rs      /restart 重启
│   │   ├── status.rs       /status 状态查询
│   │   └── traits.rs       AgentCommand trait
│   │
│   ├── tools/              工具系统
│   │   ├── mod.rs          ToolRegistry
│   │   ├── traits.rs       Tool trait
│   │   ├── path_guard.rs   PathGuard 路径沙箱
│   │   ├── shell.rs        Shell 工具
│   │   ├── file_read.rs    文件读取
│   │   ├── file_write.rs   文件写入
│   │   ├── file_edit.rs    文件编辑（内容替换）
│   │   ├── find_files.rs   文件搜索（Glob）
│   │   ├── search_content.rs 内容搜索（正则）
│   │   └── mcp.rs          MCP 工具代理
│   │
│   ├── skills/             技能扩展
│   │   ├── mod.rs
│   │   └── registry.rs     SkillsRegistry
│   │
│   ├── memory/             记忆系统
│   │   ├── mod.rs
│   │   ├── recall.rs       向量检索
│   │   └── types.rs        Memory 类型定义
│   │
│   └── subagent/           后台子代理
│       ├── mod.rs
│       ├── manager.rs      SubAgentManager
│       └── types.rs
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
