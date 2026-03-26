# PartnerClaw 技术架构设计

## 一、系统总览

### 技术栈

| 层 | 技术 | 版本 |
|---|------|------|
| 桌面框架 | Tauri | 2.x |
| 前端 | React + TypeScript | 19.x / 6.x |
| 后端 | Rust | 2021 Edition |
| 构建 | Vite + Bun | 8.x / latest |
| 存储 | SQLite + Markdown + 本地文件系统 | — |
| LLM | Claude API (BYOK) | — |

### 架构分层

```
┌─────────────────────────────────────────────────────────┐
│                React Frontend (UI)                      │
│  Pages · Components · Hooks · Zustand Store             │
├──────────────┬───────────────┬──────────────────────────┤
│ Web Commands │ Agent Cmds    │  CLI Binary              │
│ invoke()     │ /new /stop    │  partnerclaw <sub>       │
│ → Services   │ /restart      │  clap → CliRuntime       │
│ (~28 个 IPC) │ /status       │  → Services              │
├──────────────┴───────────────┴──────────────────────────┤
│   Channel Layer   │         Gateway Layer               │
│   Desktop         │  HTTP Server (PWA/Webhook)          │
│   Telegram        │  WebSocket (实时通信)                │
│   Feishu          │                                     │
├───────────────────┴─────────────────────────────────────┤
│          MessageBus（双向异步消息队列）                   │
│   inbound: Channel → Agent  │  outbound: Agent → Channel│
├─────────────────────────────┴───────────────────────────┤
│             Core Agent Service (编排器)                  │
│  AgentLoop · Context · Session · SubAgent               │
├──────────────────┬──────────────────────────────────────┤
│  Provider Layer  │          Tool Layer                  │
│  Claude API      │  基础能力: fs · shell · mcp_client   │
│  Haiku / Sonnet  │  元工具: operations (按需调用)        │
├──────────────────┼──────────────────────────────────────┤
│  Memory Layer    │        Services Layer                │
│  Agent 私有记忆   │  核心业务逻辑 (前端/Agent 共用)       │
│  观察·偏好·模式   │  Knowledge · Daily · Task · Capture │
│  (SQLite)        │  (操作 Markdown + SQLite)            │
├──────────────────┴──────────────────────────────────────┤
│           Infrastructure Layer (基础设施)               │
│  Cron (定时任务) · Heartbeat (健康检测) · Logging       │
├──────────────────┬──────────────┬───────────────────────┤
│   SQLite         │  Markdown FS │  OS Keychain          │
│  结构+索引+记忆   │  内容真相     │  API Key              │
└──────────────────┴──────────────┴───────────────────────┘

调用关系：
  前端: Commands → Services → Storage
  Agent: AgentLoop → Tools → Services → Storage
                   → Memory → Storage
                   → Provider (LLM)
  记忆是 Agent 的 (Memory/SQLite)，知识是共同的 (Knowledge/Markdown)
```

### 桌面端即服务器

桌面端是数据和 Agent 的唯一运行环境。移动端（Phase 2）通过本地 WiFi 或 Tailscale 接入桌面端的 Web Server，作为薄客户端。MVP 阶段移动对话通过 Telegram/Feishu Bot webhook 实现。

---

## 二、目录结构

### 代码目录

```
src-tauri/
  src/
    main.rs                     # 入口，委托给 lib::run()
    lib.rs                      # Tauri Builder：插件注册、命令注册、状态注入
    error.rs                    # 统一错误类型 AppError（实现 Serialize 用于 IPC）
    channels/
      mod.rs                    # Channel trait 定义 + start_channels()
      traits.rs                 # ChannelMessage, SendMessage, Channel trait
      desktop.rs                # 桌面端 Channel（Tauri IPC ↔ ChannelMessage 桥接）
      telegram.rs               # Telegram Bot Channel（Phase 1 后期）
      feishu.rs                 # 飞书 Bot Channel（Phase 2）
    bus/
      mod.rs                    # MessageBus：双向异步消息队列（Channel ↔ Agent 解耦）
      events.rs                 # InboundMessage, OutboundMessage 定义
    commands/                    # Tier 1: Web Commands（Tauri IPC，前端 invoke() 调用）
      mod.rs                    # 导出所有命令模块
      capture.rs                # 捕获：→ CaptureService
      conversation.rs           # 对话：→ AgentLoop (发消息) + SessionManager (查历史)
      daily.rs                  # 日记：→ DailyService
      tasks.rs                  # 任务：→ TaskService
      knowledge.rs              # 知识库：→ KnowledgeService
      settings.rs               # 设置：→ Storage (settings.json + keychain)
      system.rs                 # 系统：→ Heartbeat + Gateway + Cron 状态
    agent_commands/              # Tier 2: Agent 控制指令（对话内 /xxx 生命周期管控）
      mod.rs                    # AgentCommandRegistry：注册/解析/分发
      traits.rs                 # AgentCommand trait + AgentCommandContext + AgentAction
      new.rs                    # /new — 创建新会话（关闭当前 Session）
      stop.rs                   # /stop — 停止所有进行中操作（取消 SubAgent 任务）
      restart.rs                # /restart — 重启 Agent 服务（重新初始化）
      status.rs                 # /status — 查看 Agent 状态、连接、队列
    cli/                         # Tier 3: CLI 命令行（终端使用，独立二进制）
      mod.rs                    # clap App 定义 + run()
      runtime.rs                # CliRuntime：最小运行时（DB + Services，无 UI）
      capture.rs                # partnerclaw capture "text"
      daily.rs                  # partnerclaw daily [date]
      search.rs                 # partnerclaw search "query"
      task.rs                   # partnerclaw task create/list/complete
      chat.rs                   # partnerclaw chat "message"（需 API Key）
      status.rs                 # partnerclaw status
      export.rs                 # partnerclaw export [format]
    bin/
      cli.rs                    # CLI 独立二进制入口（不启动 Tauri）
    storage/
      mod.rs
      database.rs               # SQLite 连接管理、迁移、CRUD
      markdown.rs               # Markdown 文件读写、frontmatter 解析
      vector.rs                 # sqlite-vss 向量索引（Phase 2，MVP 用 FTS5）
      archive.rs                # JSONL 冷归档读写
      keychain.rs               # OS Keychain 存取（keyring crate）
    agent/
      mod.rs                    # AgentService 构造与初始化、接线
      agent_loop.rs             # AgentLoop：消息处理主循环，Channel → Context → Provider → Tool 循环 → 响应
      context.rs                # ContextBuilder：System Prompt 组装、从 Memory 拉取记忆、token 预算
      session.rs                # SessionManager：按 sender 隔离会话、历史追加、裁剪、持久化
      sub_agent.rs              # SubAgent：异步子任务执行器（写入 Memory、知识沉淀等）
    memory/
      mod.rs                    # MemoryManager：统一记忆层入口（单表 memories，upsert by key）
      types.rs                  # Memory, MemoryCategory 结构定义
      recall.rs                 # 记忆召回：关键词 + 向量检索，importance 排序
    services/
      mod.rs                    # 导出所有业务 Service
      knowledge.rs              # KnowledgeService：知识笔记 CRUD、wikilink 提取、索引同步
      daily.rs                  # DailyService：日记读写、模板创建、条目追加
      task.rs                   # TaskService：任务 CRUD、状态管理
      capture.rs                # CaptureService：捕获队列管理、路由结果写入
    providers/
      mod.rs                    # Provider trait 定义 + create_provider() 工厂
      traits.rs                 # Provider trait、ModelTier、ChatMessage、ProviderResponse
      claude.rs                 # ClaudeProvider：Claude API 实现（Haiku/Sonnet 分层）
      config.rs                 # 模型配置、API endpoint、token 限制
    tools/
      mod.rs                    # ToolRegistry + Tool trait（注册/查找/执行）
      traits.rs                 # Tool trait、ToolInput、ToolOutput
      # --- 基础能力工具（常驻上下文，4 个 Schema）---
      filesystem.rs             # vault 内文件操作（安全边界约束）
      shell.rs                  # 白名单受限 Shell（沙箱执行）
      mcp_client.rs             # MCP Client：接入外部工具服务
      operations.rs             # 元工具：list/call 动态发现并调用 Services + Memory
    gateway/
      mod.rs                    # Gateway 启动与路由配置
      server.rs                 # HTTP Server（actix-web / axum）：PWA 静态文件 + API
      api.rs                    # REST API：webhook 接收、chat endpoint、知识查询
      ws.rs                     # WebSocket endpoint：实时对话通道
      auth.rs                   # 认证：Bearer token、签名验证、IP 限制
    cron/
      mod.rs                    # CronScheduler：定时任务注册与调度
      jobs.rs                   # 具体任务定义
      scheduler.rs              # tokio 定时调度引擎
    heartbeat/
      mod.rs                    # 健康检测与系统状态监控
    models/
      mod.rs
      note.rs                   # Note, DailyNote, KnowledgeEntry
      task.rs                   # Task（状态、截止日期、上下文）
      conversation.rs           # Message, Session, ConversationMode
      capture.rs                # CaptureItem, CaptureRoute
      settings.rs               # AppSettings, UserRole, AgentPreference
  Cargo.toml
  tauri.conf.json
  capabilities/
    default.json

src/
  main.tsx                      # React 入口
  App.tsx                       # 根组件：路由、全局 Provider
  pages/
    DailyPage.tsx               # 日记视图（默认首页，PRD 中的"锚点"）
    InboxPage.tsx               # 捕获收件箱，Agent 路由审核
    KnowledgePage.tsx           # 知识库浏览与搜索
    ConversationPage.tsx        # 对话界面，模式选择
    SettingsPage.tsx            # 设置、API Key、角色模版
  components/
    layout/
      Sidebar.tsx               # 导航：Daily / Inbox / Knowledge / Chat / Settings
      TopBar.tsx                # 全局快捷捕获栏 + 模式指示器
    capture/
      QuickCapture.tsx          # 3 秒捕获输入（文本、链接粘贴）
      CaptureCard.tsx           # 收件箱单项卡片
      RouteReview.tsx           # 审核 Agent 路由决策
    daily/
      DailyEditor.tsx           # Markdown 编辑器
      TaskCard.tsx              # 嵌入式任务卡片（状态切换）
      DailyTimeline.tsx         # 当日时间线视图
    conversation/
      ChatView.tsx              # 消息列表 + 输入
      ModeSelector.tsx          # 五种交互模式切换
      MessageBubble.tsx         # 单条消息
    knowledge/
      KnowledgeList.tsx         # 知识条目列表（可筛选）
      KnowledgeDetail.tsx       # 知识笔记详情
      SearchBar.tsx             # 关键词 + 语义搜索
    settings/
      ApiKeyInput.tsx           # API Key 安全输入
      RoleTemplates.tsx         # 冷启动角色选择
      ModelSelector.tsx         # Haiku/Sonnet 偏好
  hooks/
    useIpc.ts                   # 通用 invoke() 封装（泛型、错误处理、loading）
    useCapture.ts               # 捕获提交与收件箱状态
    useConversation.ts          # 对话状态、消息发送、模式切换
    useDaily.ts                 # 日记 CRUD
    useKnowledge.ts             # 知识搜索与浏览
    useTasks.ts                 # 任务 CRUD
    useSettings.ts              # 设置读写
  store/
    appStore.ts                 # 全局状态：当前页、用户信息、初始化状态
    captureStore.ts             # 收件箱项、待路由队列
    conversationStore.ts        # 活跃会话、消息列表、当前模式、流式状态
  lib/
    ipc.ts                      # IPC 命令类型定义（与 Rust 命令签名对齐）
    types.ts                    # TypeScript 类型（镜像 Rust models）
    constants.ts                # 模式名称、路由类别等常量
  styles/
    global.css
    variables.css
```

### 用户数据目录（运行时）

```
~/PartnerClaw/
  vault/                        # Markdown 内容（Obsidian 兼容）
  │ ├── daily/                  # YYYY-MM-DD.md
  │ ├── knowledge/              # Agent 沉淀的知识笔记
  │ ├── private/                # 私密区（Agent 不可见）
  │ └── _assets/                # 附件（图片、PDF）
  data/
  │ ├── main.db                 # SQLite 主库（结构索引 + FTS5）
  │ ├── queue.db                # 离线捕获队列（轻量独立）
  │ └── archive/                # 冷归档
  │     └── 2026-01.jsonl       # 按月归档对话
  config/
    └── settings.json           # 非敏感设置
```

整个 `~/PartnerClaw/` 目录 zip 打包即完整备份。

---

## 三、三层命令架构

系统提供三种命令入口，覆盖不同使用场景，底层共享 Services 层：

```
React Frontend ── invoke() ──► Web Commands ──► Services ──► Storage
对话中 /xxx ─────────────────► Agent Commands ─► Agent 生命周期控制
终端 partnerclaw ─────────────► CLI Commands ──► Services ──► Storage
```

### 跨层命令矩阵

| 维度 | Web Commands | Agent Commands | CLI Commands |
|------|-------------|----------------|-------------|
| 入口 | React `invoke()` | 对话消息 `/xxx` | 终端 `partnerclaw` |
| 职责 | 业务 CRUD（完整） | Agent 生命周期控制 | 自动化/脚本操作 |
| 数量 | ~28 个 | 4 个 | ~7 个 |
| 调用链 | Command → Services | AgentLoop 拦截 → Agent 自身 | CliRuntime → Services |
| 需要 Tauri | 是 | 是（运行在 AgentLoop 内） | 否（独立二进制） |
| 需要 LLM | 否（除 capture_route） | 否（纯控制） | 仅 chat 子命令 |

### 3.1 Web Commands — Tauri IPC（前端调用）

所有前后端通信通过 Tauri `invoke()` 和 Event System 完成。命令统一返回 `Result<T, AppError>`。

调用链：`React invoke() → #[tauri::command] → Services → Storage`

#### Capture（捕获）

| 命令 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `capture_submit` | `{ raw: String, source: String }` | `CaptureItem` | 提交原始捕获 |
| `capture_list_pending` | `{}` | `Vec<CaptureItem>` | 待处理列表 |
| `capture_route` | `{ id: String }` | `CaptureRoute` | Agent 路由建议（Haiku） |
| `capture_confirm_route` | `{ id: String, route: String, adjusted: bool }` | `()` | 确认/调整路由 |

#### Conversation（对话）

| 命令 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `conversation_send` | `{ message: String, mode: String }` | `String`（session_id） | 发起对话，响应通过 Event 流式推送 |
| `conversation_history` | `{ session_id: String, limit: u32 }` | `Vec<Message>` | 获取会话历史 |
| `conversation_sessions` | `{ limit: u32 }` | `Vec<Session>` | 会话列表 |

流式响应通过 Tauri Event 推送：

```
Event: "conversation_chunk" → { session_id, content, done }
```

前端通过 `listen("conversation_chunk", callback)` 接收。

#### Daily（日记）

| 命令 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `daily_get` | `{ date: String }` | `DailyNote` | 获取或创建当日日记 |
| `daily_save` | `{ date: String, content: String }` | `()` | 保存日记 |
| `daily_list` | `{ limit: u32 }` | `Vec<DailyMeta>` | 日记列表（元数据） |

#### Tasks（任务）

| 命令 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `task_create` | `{ content, due?, context?, note_path? }` | `Task` | 创建任务 |
| `task_update` | `{ id, status?, content?, due? }` | `Task` | 更新任务 |
| `task_list` | `{ status?: String }` | `Vec<Task>` | 任务列表（可按状态筛选） |

#### Knowledge（知识库）

| 命令 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `knowledge_search` | `{ query: String, limit: u32 }` | `Vec<KnowledgeEntry>` | 搜索知识条目 |
| `knowledge_list` | `{ tag?: String }` | `Vec<KnowledgeEntry>` | 知识列表 |
| `knowledge_get` | `{ path: String }` | `KnowledgeNote` | 获取完整知识笔记 |
| `knowledge_update` | `{ path: String, content: String }` | `()` | 人类纠偏修改 |

#### Settings（设置）

| 命令 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `settings_get` | `{}` | `AppSettings` | 读取全部设置 |
| `settings_set` | `{ key: String, value: Value }` | `()` | 更新单项设置 |
| `apikey_store` | `{ key: String }` | `()` | 存入 OS Keychain |
| `apikey_exists` | `{}` | `bool` | 检查 Key 是否存在 |

#### Agent

| 命令 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `agent_memories` | `{ limit: u32, category?: String }` | `Vec<Memory>` | Agent 记忆查询 |
| `agent_status` | `{}` | `AgentStatus` | Agent 运行状态 |

#### System（系统）

| 命令 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `system_health` | `{}` | `SystemHealth` | 系统健康状态（Heartbeat） |
| `gateway_start` | `{ port?: u16 }` | `()` | 启动 Gateway 服务 |
| `gateway_stop` | `{}` | `()` | 停止 Gateway 服务 |
| `gateway_status` | `{}` | `GatewayStatus` | Gateway 运行状态 |
| `cron_list` | `{}` | `Vec<CronJob>` | 定时任务列表 |
| `cron_toggle` | `{ name: String, enabled: bool }` | `()` | 启用/禁用定时任务 |

#### Tauri 状态管理

通过 `.manage()` 注入全局共享状态：

```rust
// lib.rs
.manage(DbState(Mutex::new(connection)))
.manage(AppConfig::load()?)
```

命令通过 `State<'_, DbState>` 参数获取。

### 3.2 Agent Commands — 控制指令（对话内）

Agent 生命周期管控指令。用户在对话中输入 `/xxx` 来控制 Agent 行为，不触发 LLM 调用，直接返回确定性结果。

#### 指令清单

| 指令 | 说明 | 行为 |
|------|------|------|
| `/new` | 新建会话 | 关闭当前 Session，创建新的空 Session，返回确认 |
| `/stop` | 停止操作 | 取消所有进行中的 SubAgent 任务，中断流式响应，返回确认 |
| `/restart` | 重启服务 | 重新初始化 AgentLoop（重载配置、重连 Provider），返回状态 |
| `/status` | 查看状态 | 返回 Agent 运行状态、活跃 Session 数、SubAgent 队列长度、Provider 连接状态、Memory 统计 |

#### 核心设计

```rust
// src-tauri/src/agent_commands/traits.rs

pub struct AgentCommandContext {
    pub session: Session,
    pub session_mgr: Arc<SessionManager>,
    pub sub_agent_tx: mpsc::Sender<SubAgentTask>,
}

pub struct AgentCommandResult {
    pub response: String,          // 返回给用户的文本
    pub action: AgentAction,       // 后续动作
}

pub enum AgentAction {
    None,                          // /status: 仅返回信息
    NewSession(Session),           // /new: 切换到新 Session
    StopAll,                       // /stop: 取消进行中操作
    Restart,                       // /restart: 触发重启流程
}

#[async_trait]
pub trait AgentCommand: Send + Sync {
    fn name(&self) -> &str;        // "new", "stop" 等（不含 /）
    fn description(&self) -> &str;
    async fn execute(&self, ctx: AgentCommandContext) -> Result<AgentCommandResult, AppError>;
}
```

```rust
// src-tauri/src/agent_commands/mod.rs

pub struct AgentCommandRegistry {
    commands: HashMap<String, Arc<dyn AgentCommand>>,
}

impl AgentCommandRegistry {
    pub fn default() -> Self {
        let mut registry = Self { commands: HashMap::new() };
        registry.register(Arc::new(NewCommand));
        registry.register(Arc::new(StopCommand));
        registry.register(Arc::new(RestartCommand));
        registry.register(Arc::new(StatusCommand));
        registry
    }

    pub fn get(&self, name: &str) -> Option<&Arc<dyn AgentCommand>> {
        self.commands.get(name)
    }
}

/// 解析消息是否为 Agent 控制指令
/// 仅匹配以 "/" 开头且命令名在注册表中的消息
pub fn parse_agent_command(input: &str) -> Option<&str> {
    let trimmed = input.trim();
    if trimmed.starts_with('/') {
        Some(trimmed[1..].split_whitespace().next()?)
    } else {
        None
    }
}
```

#### 拦截点

Agent Commands 在 `AgentLoop.process_message()` 中拦截，位于 Session 加载之后、Context 组装之前。所有 Channel（Desktop/Telegram/Feishu）自动获得指令支持：

```rust
// 在 agent_loop.rs process_message() 中

// 1. Session
let session = self.session_mgr.get_or_create(&message.sender, &message.mode).await?;

// 1.5 Agent Command 拦截（/new /stop /restart /status）
if let Some(cmd_name) = parse_agent_command(&message.content) {
    if let Some(cmd) = self.agent_commands.get(cmd_name) {
        let ctx = AgentCommandContext {
            session: session.clone(),
            session_mgr: self.session_mgr.clone(),
            sub_agent_tx: self.sub_agent_tx.clone(),
        };
        let result = cmd.execute(ctx).await?;
        channel.send(SendMessage::text(&result.response)).await?;
        self.handle_action(result.action).await?;
        return Ok(AgentResponse::from_text(result.response));
    }
}

// 2. Context（正常对话流程继续）
let context = self.context_builder.build(&message, &session).await?;
// ...
```

#### 指令实现示例

```rust
// src-tauri/src/agent_commands/new.rs

pub struct NewCommand;

#[async_trait]
impl AgentCommand for NewCommand {
    fn name(&self) -> &str { "new" }
    fn description(&self) -> &str { "创建新会话" }

    async fn execute(&self, ctx: AgentCommandContext) -> Result<AgentCommandResult, AppError> {
        // 关闭当前 Session
        ctx.session_mgr.close(&ctx.session.id).await?;

        // 创建新 Session
        let new_session = ctx.session_mgr
            .create(&ctx.session.sender, &ctx.session.mode).await?;

        Ok(AgentCommandResult {
            response: format!("✓ 已创建新会话 ({})", &new_session.id[..8]),
            action: AgentAction::NewSession(new_session),
        })
    }
}
```

### 3.3 CLI Commands — 终端命令行

无 GUI 场景下的操作入口，用于自动化、脚本、远程管理。独立二进制，不启动 Tauri/UI。

#### CLI 定义

```rust
// src-tauri/src/cli/mod.rs

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "partnerclaw", about = "PartnerClaw CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: CliCommand,

    /// 指定 vault 路径（默认 ~/PartnerClaw）
    #[arg(long, global = true)]
    pub vault: Option<PathBuf>,
}

#[derive(Subcommand)]
pub enum CliCommand {
    /// 快速捕获一个想法
    Capture { text: String },

    /// 查看/创建今日日记
    Daily { date: Option<String> },

    /// 搜索知识库
    Search {
        query: String,
        #[arg(short, default_value = "5")]
        limit: u32,
    },

    /// 任务管理
    Task {
        #[command(subcommand)]
        action: TaskAction,
    },

    /// 发送消息给 Agent（单轮，需 API Key）
    Chat { message: String },

    /// 查看系统状态
    Status,

    /// 导出数据备份
    Export { format: Option<String> },
}

#[derive(Subcommand)]
pub enum TaskAction {
    Create { content: String, #[arg(long)] due: Option<String> },
    List { #[arg(long)] status: Option<String> },
    Complete { id: String },
}
```

#### 最小运行时

CLI 不依赖 Tauri 运行时，仅初始化 DB + Services：

```rust
// src-tauri/src/cli/runtime.rs

pub struct CliRuntime {
    pub db: Arc<DbState>,
    pub services: Arc<ServiceContainer>,
    pub provider: Option<Arc<dyn Provider>>,  // 仅 chat 子命令需要
}

impl CliRuntime {
    pub fn init(vault_path: PathBuf) -> Result<Self, AppError> {
        let db = init_database(&vault_path.join("data/main.db"))?;
        let services = Arc::new(ServiceContainer::new(db.clone()));
        Ok(Self { db, services, provider: None })
    }

    /// chat 子命令需要 Provider
    pub fn with_provider(mut self) -> Result<Self, AppError> {
        let key = keyring::Entry::new("partnerclaw", "api_key")?.get_password()?;
        self.provider = Some(Arc::new(ClaudeProvider::new(&key)));
        Ok(self)
    }
}
```

```rust
// src-tauri/src/bin/cli.rs

fn main() {
    let cli = Cli::parse();
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let vault = cli.vault.unwrap_or_else(default_vault_path);
        let runtime = CliRuntime::init(vault)?;

        match cli.command {
            CliCommand::Capture { text } => {
                let item = runtime.services.capture.submit(&text, "cli").await?;
                println!("已捕获: {} (id: {})", item.raw, item.id);
            }
            CliCommand::Daily { date } => {
                let d = date.unwrap_or_else(today_string);
                let note = runtime.services.daily.get(&d).await?;
                println!("{}", note.markdown);
            }
            CliCommand::Search { query, limit } => {
                let results = runtime.services.knowledge.search(&query, limit).await?;
                for r in results {
                    println!("  {} — {}", r.path, r.title);
                }
            }
            CliCommand::Chat { message } => {
                let runtime = runtime.with_provider()?;
                let provider = runtime.provider.as_ref().unwrap();
                let response = provider.chat(ModelTier::Sonnet, &[
                    ChatMessage::user(&message),
                ]).await?;
                println!("{}", response.text());
            }
            // ...
        }
        Ok::<(), AppError>(())
    }).expect("CLI error");
}
```

#### Cargo.toml 变更

```toml
[[bin]]
name = "partnerclaw"
path = "src/bin/cli.rs"

[dependencies]
clap = { version = "4", features = ["derive"] }
```

#### 命令清单

| 子命令 | 调用 Service | 说明 |
|--------|-------------|------|
| `partnerclaw capture "text"` | CaptureService.submit() | source="cli" |
| `partnerclaw daily [date]` | DailyService.get() | 输出 Markdown 到 stdout |
| `partnerclaw search "query"` | KnowledgeService.search() | 列表输出 |
| `partnerclaw task create "content"` | TaskService.create() | 创建任务 |
| `partnerclaw task list [--status done]` | TaskService.list() | 列出任务 |
| `partnerclaw task complete <id>` | TaskService.update() | 完成任务 |
| `partnerclaw chat "message"` | Provider.chat() | 单轮对话，需 API Key |
| `partnerclaw status` | HeartbeatMonitor (lite) | DB + vault 状态检查 |
| `partnerclaw export [json]` | Storage export | 数据备份导出 |

---

## 四、核心数据流

### 4.1 捕获流（Capture Flow）

```
用户在 QuickCapture 输入
  → invoke("capture_submit", { raw, source: "desktop" })
  → Command → CaptureService.submit(): 写入 capture_queue 表，返回 CaptureItem
  → SubAgent 异步: Haiku 分类 → { task | thought | feeling | link }
  → CaptureService.update_route(): 更新 suggested_route
  → 前端: InboxPage 轮询/监听 → 展示待审核项
  → 用户确认 → invoke("capture_confirm_route")
  → Command → CaptureService.confirm(): 按路由委托对应 Service
      ├── task    → TaskService.create()
      ├── thought → KnowledgeService.create() → vault/knowledge/ 草稿
      ├── feeling → DailyService.append() → vault/daily/当日.md
      └── link    → CaptureService.mark_processed()
```

### 4.2 对话流（Conversation Flow）

消息经过 Channel 抽象层统一处理，无论来源是桌面 UI 还是 Telegram Bot：

```
用户发送消息（桌面 UI / Telegram / Feishu）
  → Channel 将平台消息转为 ChannelMessage { sender, content, source, mode }
  → AgentService.turn_streamed(message, channel)
      ├─ SessionManager: 按 sender 加载/创建 Session
      ├─ ContextBuilder: 组装 prompt
      │    [1] 基础人格 + 模式指令
      │    [2] 用户角色上下文（user_roles 表）
      │    [3] KnowledgeService.search(): RAG 检索知识片段
      │    [4] 压缩对话历史（近 5 轮完整 + 早期摘要）
      │    [5] Memory.unsurfaced_observations(): 记忆召回
      │    [6] 用户消息
      ├─ Provider.chat_stream(): 调用 Claude API（流式）
      ├─ Channel.send_chunk(): 逐 chunk 推送到来源通道
      ├─ PostProcess: 写入 Memory + 派发 SubAgent 任务
      └─ SessionManager: 追加消息对，触发裁剪
  → 来源通道渲染响应
      Desktop: Tauri Event → 前端 useConversation 实时渲染
      Telegram: sendMessage API → 用户手机
```

### 4.3 日记流（Daily Flow）

```
DailyPage 挂载，传入今日日期
  → invoke("daily_get", { date: "2026-03-26" })
  → Command → DailyService.get(): 读取 vault/daily/2026-03-26.md（不存在则模板创建）
  → Command → TaskService.list_by_date(): 查询关联任务
  → 返回 DailyNote { markdown, tasks: Vec<Task> }
  → 前端: 渲染 Markdown + 嵌入 TaskCard 组件
  → 用户编辑 → invoke("daily_save") → Command → DailyService.save()
  → 用户切换任务状态 → invoke("task_update") → Command → TaskService.update()
```

---

## 五、存储架构

### 核心原则

**Markdown 是内容真相，SQLite 是查询索引。** SQLite 和向量索引都是 Markdown 的派生层，丢失可从 Markdown 完整重建。

### SQLite 表结构

```sql
-- Markdown 索引（派生，可从文件系统重建）
CREATE TABLE notes (
  id       TEXT PRIMARY KEY,
  path     TEXT UNIQUE NOT NULL,
  title    TEXT,
  created  TEXT NOT NULL,
  updated  TEXT NOT NULL,
  status   TEXT DEFAULT 'active',
  tags     TEXT,  -- JSON 数组
  last_indexed TEXT
);

-- 任务（一等公民，独立结构）
CREATE TABLE tasks (
  id        TEXT PRIMARY KEY,
  content   TEXT NOT NULL,
  status    TEXT DEFAULT 'pending',  -- pending | in_progress | done | cancelled
  due       TEXT,
  note_path TEXT,
  context   TEXT,
  created   TEXT NOT NULL,
  completed TEXT
);

-- 笔记链接关系（从 wikilinks 提取，派生）
CREATE TABLE links (
  source_path TEXT NOT NULL,
  target_path TEXT NOT NULL,
  context     TEXT,
  PRIMARY KEY (source_path, target_path)
);

-- Memory Layer: Agent 私有记忆（单表统一，不进 Markdown）
CREATE TABLE memories (
  id             TEXT PRIMARY KEY,
  key            TEXT UNIQUE NOT NULL,   -- 去重键，同一认知 upsert 而非 insert
  content        TEXT NOT NULL,          -- 记忆内容
  category       TEXT NOT NULL,          -- observation | preference | pattern
  type           TEXT,                   -- 子类型：insight/blindspot/emotion | communication_style | emotion_trend
  namespace      TEXT DEFAULT 'default', -- 上下文隔离（不同角色/模式下的记忆）
  importance     REAL DEFAULT 0.5,       -- 重要度（recall 排序、衰减基准）
  session_id     TEXT,                   -- 关联会话（溯源）
  related_path   TEXT,                   -- 关联笔记路径
  embedding      BLOB,                   -- 向量（Phase 2 语义检索）
  surfaced       INTEGER DEFAULT 0,      -- 是否已浮出给用户
  superseded_by  TEXT,                   -- 被哪条新记忆替代（认知演进链）
  created_at     TEXT NOT NULL,
  updated_at     TEXT NOT NULL
);

-- 索引：按 category 筛选 + importance 排序
CREATE INDEX idx_memories_category ON memories(category, importance DESC);
-- 索引：按 namespace 隔离
CREATE INDEX idx_memories_namespace ON memories(namespace);
-- 索引：未浮出的记忆（ContextBuilder 注入用）
CREATE INDEX idx_memories_unsurfaced ON memories(surfaced, importance DESC)
  WHERE surfaced = 0 AND superseded_by IS NULL;

-- 捕获队列
CREATE TABLE capture_queue (
  id         TEXT PRIMARY KEY,
  raw        TEXT NOT NULL,
  type       TEXT,  -- task | thought | feeling | link
  source     TEXT DEFAULT 'desktop',
  created    TEXT NOT NULL,
  processed  INTEGER DEFAULT 0,
  routed_to  TEXT
);

-- 对话会话
CREATE TABLE sessions (
  id      TEXT PRIMARY KEY,
  mode    TEXT NOT NULL,  -- companion | reflect | challenge | knowledge | treehole
  created TEXT NOT NULL,
  updated TEXT NOT NULL,
  summary TEXT
);

-- 对话消息（热存，90 天后转冷归档）
CREATE TABLE messages (
  id         TEXT PRIMARY KEY,
  session_id TEXT NOT NULL REFERENCES sessions(id),
  role       TEXT NOT NULL,  -- user | assistant
  content    TEXT NOT NULL,
  created    TEXT NOT NULL
);

-- 用户角色
CREATE TABLE user_roles (
  id         TEXT PRIMARY KEY,
  name       TEXT NOT NULL,
  priority   INTEGER DEFAULT 0,
  weak_point TEXT,
  created    TEXT NOT NULL
);

-- Agent 记忆偏好等在 memories 表中（category='preference'）
```

### Markdown 与 SQLite 同步

- **写入时**：Markdown 先写，然后更新 SQLite 索引（frontmatter → notes 表）
- **冲突时**：Markdown frontmatter 为权威，SQLite 索引可随时从文件系统重建
- **重建索引**：启动时检查 `last_indexed` 与文件 mtime，仅增量更新

### 对话历史分层

| 层 | 内容 | 存储 | 保留 |
|---|------|------|------|
| 原始消息 | 每句对话 | SQLite messages 表 | 90 天 |
| 会话摘要 | 每次会话精华 | SQLite sessions.summary | 永久 |
| 蒸馏知识 | 提炼的洞见 | vault/knowledge/ Markdown | 永久 |

90 天后原始消息导出为 JSONL 冷归档（`data/archive/YYYY-MM.jsonl`），SQLite 中删除。

### 设置存储分工

```
settings.json              OS Keychain                 SQLite
─────────────              ─────────────               ──────
LLM 模型选择               API Key（加密）              角色模版
主题 / 语言                                             Agent 学习偏好
Vault 路径                                              使用统计
同步配置
```

API Key 必须存入 OS Keychain，绝不能存在任何明文文件中。

---

## 六、Agent 架构

> 参考 [zeroclaw](https://github.com/zeroclaw-labs/zeroclaw) 的 Channel + Agent 分层模式。

### 6.1 整体结构

```
┌─────────────────────────────────────────────────────────────┐
│                      Channel Layer                          │
│  ┌──────────┐  ┌──────────────┐  ┌───────────────┐         │
│  │ Desktop  │  │ Telegram Bot │  │  Feishu Bot   │         │
│  │ Channel  │  │   Channel    │  │   Channel     │         │
│  └─────┬────┘  └──────┬───────┘  └───────┬───────┘         │
│        └──────────────┼──────────────────┘                  │
│                       │                                     │
│              ChannelMessage / SendMessage                    │
└───────────────────────┼─────────────────────────────────────┘
                        │
┌───────────────────────┼─────────────────────────────────────┐
│                  Gateway Layer                              │
│  ┌──────────────┐  ┌──────────────┐  ┌────────────────┐    │
│  │ HTTP Server  │  │  WebSocket   │  │  Auth Guard    │    │
│  │ (PWA/API)    │  │  (实时对话)   │  │  (Token/签名)  │    │
│  └──────┬───────┘  └──────┬───────┘  └────────────────┘    │
│         └─────────────────┘                                 │
└───────────────────────┼─────────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────────────────┐
│                  Core Agent Service                         │
│                                                             │
│  ┌──────────┐ ┌──────────┐ ┌────────┐ ┌────────────────┐  │
│  │ Context  │ │ Session  │ │ Router │ │   Observer     │  │
│  │ Builder  │ │ Manager  │ │        │ │ (Layer 3 观察)  │  │
│  └──────────┘ └──────────┘ └────────┘ └────────────────┘  │
│                                                             │
│  ┌───────────────────────┐   ┌──────────────────────────┐  │
│  │     Tool Registry     │   │   Memory / Knowledge     │  │
│  │  搜索·分析·写作·文件    │   │   RAG · 观察 · 知识库    │  │
│  └───────────────────────┘   └──────────────────────────┘  │
└───────────────────────┬─────────────────────────────────────┘
                        │
┌───────────────────────┼─────────────────────────────────────┐
│                  Provider Layer                             │
│  ┌──────────┐  ┌──────────┐  ┌──────────────────────────┐  │
│  │  Haiku   │  │  Sonnet  │  │  Local Embedding         │  │
│  │ (路由/分类)│  │ (深度对话)│  │  (向量检索, Phase 2)     │  │
│  └──────────┘  └──────────┘  └──────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│               Infrastructure Layer                          │
│  ┌──────────────┐  ┌────────────────┐  ┌────────────────┐  │
│  │     Cron     │  │   Heartbeat    │  │    Logging     │  │
│  │  定时任务调度  │  │  健康检测/监控   │  │  tracing 日志  │  │
│  └──────────────┘  └────────────────┘  └────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

**Core Agent 是唯一持有完整人模型的编排器**。Channel、Gateway、Provider、Tools 都是可替换的适配层，通过 trait 解耦。Cron 和 Heartbeat 提供后台运行能力。

### 6.2 Channel Trait — 统一消息通道

Channel 是所有通信平台的抽象接口。无论消息来自桌面 UI、Telegram 还是 Feishu，Agent 看到的都是统一的 `ChannelMessage`。

```rust
// src-tauri/src/channels/traits.rs

/// 通道消息（入站）
pub struct ChannelMessage {
    pub id: String,
    pub sender: String,        // 用户标识
    pub content: String,       // 消息内容
    pub source: ChannelSource, // 来源通道
    pub timestamp: DateTime<Utc>,
    pub mode: ConversationMode, // 交互模式
}

/// 发送消息（出站）
pub struct SendMessage {
    pub content: String,
    pub recipient: String,
    pub metadata: Option<serde_json::Value>,
}

/// 通道来源
pub enum ChannelSource {
    Desktop,    // Tauri 桌面端
    Telegram,   // Telegram Bot
    Feishu,     // 飞书 Bot
    Webhook,    // 通用 Webhook
}

/// Channel trait — 所有通道实现此接口
#[async_trait]
pub trait Channel: Send + Sync {
    /// 通道名称
    fn name(&self) -> &str;

    /// 通道来源标识
    fn source(&self) -> ChannelSource;

    /// 发送消息到通道（由 outbound 消费循环调用）
    async fn send(&self, message: OutboundMessage) -> Result<(), AppError>;

    /// 监听平台消息，推入 Bus 入站队列（长运行）
    async fn listen(&self, bus: Arc<MessageBus>) -> Result<(), AppError>;

    // --- 可选能力（默认空实现）---

    /// 是否支持流式输出
    fn supports_streaming(&self) -> bool { false }

    /// 发送流式 chunk
    async fn send_chunk(&self, _chunk: &str, _session_id: &str) -> Result<(), AppError> {
        Ok(())
    }

    /// 发送 typing 指示器
    async fn start_typing(&self) -> Result<(), AppError> { Ok(()) }
    async fn stop_typing(&self) -> Result<(), AppError> { Ok(()) }
}
```

### 6.3 MessageBus — 双向异步消息队列

MessageBus 解耦 Channel 与 Agent 的消息传递。Channel 推入站消息，Agent 推出站消息，双方互不直接引用。

```
Channel.listen()                          Channel.send()
      │                                        ▲
      ▼                                        │
┌──────────────────────────────────────────────────┐
│                  MessageBus                      │
│                                                  │
│  inbound: Queue<InboundMessage>     ──► Agent 消费│
│  outbound: Queue<OutboundMessage>   ◄── Agent 推送│
│                                                  │
└──────────────────────────────────────────────────┘
```

**核心价值**：出站队列使 Channel 断线时消息不丢失，重连后可继续消费。

```rust
// src-tauri/src/bus/events.rs

/// 入站消息：Channel → Agent
pub struct InboundMessage {
    pub id: String,
    pub channel_message: ChannelMessage,
    pub source: ChannelSource,
    pub reply_to: ChannelSource,         // 响应应发回哪个通道
}

/// 出站消息：Agent → Channel
pub struct OutboundMessage {
    pub id: String,
    pub target: ChannelSource,           // 目标通道
    pub session_id: String,
    pub payload: OutboundPayload,
}

pub enum OutboundPayload {
    Text(String),                        // 完整文本响应
    Chunk { content: String, done: bool }, // 流式片段
    Typing(bool),                        // typing 指示器
    Error(String),                       // 错误消息
}
```

```rust
// src-tauri/src/bus/mod.rs

pub struct MessageBus {
    inbound: mpsc::Sender<InboundMessage>,
    inbound_rx: Mutex<Option<mpsc::Receiver<InboundMessage>>>,
    outbound: mpsc::Sender<OutboundMessage>,
    outbound_rx: Mutex<Option<mpsc::Receiver<OutboundMessage>>>,
}

impl MessageBus {
    pub fn new(buffer_size: usize) -> Self {
        let (in_tx, in_rx) = mpsc::channel(buffer_size);
        let (out_tx, out_rx) = mpsc::channel(buffer_size);
        Self {
            inbound: in_tx,
            inbound_rx: Mutex::new(Some(in_rx)),
            outbound: out_tx,
            outbound_rx: Mutex::new(Some(out_rx)),
        }
    }

    /// Channel 调用：推送入站消息
    pub async fn publish_inbound(&self, msg: InboundMessage) {
        let _ = self.inbound.send(msg).await;
    }

    /// AgentLoop 调用：取出下一条入站消息（阻塞等待）
    /// inbound_rx 在启动时 take 出来交给 AgentLoop
    pub fn take_inbound_rx(&self) -> mpsc::Receiver<InboundMessage> {
        self.inbound_rx.lock().unwrap().take().expect("inbound_rx already taken")
    }

    /// AgentLoop 调用：推送出站消息
    pub async fn publish_outbound(&self, msg: OutboundMessage) {
        let _ = self.outbound.send(msg).await;
    }

    /// 出站消费循环调用：取出下一条出站消息
    pub fn take_outbound_rx(&self) -> mpsc::Receiver<OutboundMessage> {
        self.outbound_rx.lock().unwrap().take().expect("outbound_rx already taken")
    }

    /// 队列状态（/status 指令可用）
    pub fn inbound_pending(&self) -> usize;
    pub fn outbound_pending(&self) -> usize;
}
```

**出站消费循环**：根据 `target` 路由到对应 Channel：

```rust
// src-tauri/src/channels/mod.rs

pub async fn run_outbound_dispatcher(
    mut rx: mpsc::Receiver<OutboundMessage>,
    channels: HashMap<ChannelSource, Arc<dyn Channel>>,
) {
    while let Some(msg) = rx.recv().await {
        if let Some(channel) = channels.get(&msg.target) {
            if let Err(e) = channel.send(msg).await {
                tracing::error!("Outbound dispatch failed: {}", e);
                // 失败消息可放回队列重试（Phase 2）
            }
        }
    }
}
```

### 6.4 Channel 实现

#### Desktop Channel（MVP 核心）

桌面端 Channel 是 Tauri IPC 的桥梁——前端 `invoke()` 调用通过 Desktop Channel 转化为 `ChannelMessage`，Agent 响应通过 Tauri Event 推回前端。

```rust
// src-tauri/src/channels/desktop.rs

pub struct DesktopChannel {
    app_handle: AppHandle,
}

#[async_trait]
impl Channel for DesktopChannel {
    fn name(&self) -> &str { "desktop" }
    fn source(&self) -> ChannelSource { ChannelSource::Desktop }
    fn supports_streaming(&self) -> bool { true }

    async fn send(&self, msg: OutboundMessage) -> Result<(), AppError> {
        match msg.payload {
            OutboundPayload::Text(text) => {
                self.app_handle.emit("agent_response", json!({
                    "session_id": msg.session_id, "content": text
                }))?;
            }
            OutboundPayload::Chunk { content, done } => {
                self.app_handle.emit("conversation_chunk", json!({
                    "session_id": msg.session_id, "content": content, "done": done
                }))?;
            }
            OutboundPayload::Typing(active) => {
                self.app_handle.emit("typing", json!({"active": active}))?;
            }
            OutboundPayload::Error(err) => {
                self.app_handle.emit("agent_error", json!({"error": err}))?;
            }
        }
        Ok(())
    }

    async fn listen(&self, bus: Arc<MessageBus>) -> Result<(), AppError> {
        // Desktop 的入站由 Tauri command 驱动：
        // commands/conversation.rs 接收前端 invoke() 后调用
        // bus.publish_inbound(InboundMessage { ... })
        Ok(())
    }
}
```

#### Telegram Channel（Phase 1 后期）

```rust
// src-tauri/src/channels/telegram.rs

pub struct TelegramChannel {
    bot_token: String,
    http_client: reqwest::Client,
}

#[async_trait]
impl Channel for TelegramChannel {
    fn name(&self) -> &str { "telegram" }
    fn source(&self) -> ChannelSource { ChannelSource::Telegram }

    async fn send(&self, msg: OutboundMessage) -> Result<(), AppError> {
        match msg.payload {
            OutboundPayload::Text(text) => {
                // POST https://api.telegram.org/bot{token}/sendMessage
                // ...
            }
            _ => {} // Telegram 不支持 chunk/typing
        }
        Ok(())
    }

    async fn listen(&self, bus: Arc<MessageBus>) -> Result<(), AppError> {
        // Long polling getUpdates 或 Webhook 模式
        // 将 Telegram Update 转为 ChannelMessage
        // bus.publish_inbound(InboundMessage { ... })
    }
}
```

### 6.5 Agent 三件套：Loop · Context · Session

Agent 模块内部由三个核心组件驱动，职责清晰分离：

```
              Bus.inbound (入站队列)
                        │
                        ▼
┌───────────────────────────────────────────────────────┐
│                AgentLoop (主循环)                      │
│  消费入站 → 协调 Context/Session → 调用 Provider      │
│  → 工具调用循环 → 派发 SubAgent → 推送 Bus.outbound   │
│                                                       │
│  ┌─────────────────┐  ┌────────────────────────┐      │
│  │  SessionManager │  │   ContextBuilder       │      │
│  │  会话生命周期    │  │   上下文组装引擎        │      │
│  │  历史存取/裁剪   │  │   RAG/压缩/token预算   │      │
│  └────────┬────────┘  └───────────┬────────────┘      │
│           │                       │                   │
│           └───────────┬───────────┘                   │
│                       ▼                               │
│              Provider.chat() / chat_stream()          │
│                       │                               │
│                       ▼                               │
│              ToolRegistry.execute() (工具调用循环)     │
│                       │                               │
│              ┌────────┴────────┐                      │
│              │                 │                      │
│              ▼                 ▼                      │
│   Bus.outbound 推送   SubAgent 派发 (异步)           │
│         Channel ←      ├── KnowledgeDistill          │
│                        ├── ObservationAnalyze         │
│                        ├── SessionSummarize           │
│                        ├── CaptureRoute               │
│                        └── DailySummary               │
└───────────────────────┬───────────────────────────────┘
                        │
                        ▼
                   SendMessage (出站，立即返回)
```

#### AgentLoop — 消息处理主循环

AgentLoop 是 Agent 的驱动引擎。它从 Channel 接收 `ChannelMessage`，协调 Session 和 Context 组装完整 prompt，向 Provider 发起请求，处理工具调用循环，最终通过 Channel 返回响应。

```rust
// src-tauri/src/agent/agent_loop.rs

pub struct AgentLoop {
    bus: Arc<MessageBus>,                      // 双向消息总线
    session_mgr: Arc<SessionManager>,         // 会话管理
    context_builder: Arc<ContextBuilder>,      // 上下文组装
    provider: Arc<dyn Provider>,               // LLM 调用（外部注入）
    tools: Arc<ToolRegistry>,                  // 工具注册表（外部注入）
    memory: Arc<MemoryManager>,                // 记忆层（观察/偏好/模式/召回）
    agent_commands: Arc<AgentCommandRegistry>, // 控制指令（/new /stop /restart /status）
    sub_agent_tx: mpsc::Sender<SubAgentTask>,  // SubAgent 任务派发
}

impl AgentLoop {
    /// 启动消息消费循环（长运行，从 Bus inbound 队列持续消费）
    pub async fn run(&self, mut inbound_rx: mpsc::Receiver<InboundMessage>) {
        while let Some(inbound) = inbound_rx.recv().await {
            let result = self.process_message(inbound.channel_message, &inbound.reply_to).await;
            if let Err(e) = result {
                tracing::error!("AgentLoop error: {}", e);
                self.bus.publish_outbound(OutboundMessage {
                    id: uuid(),
                    target: inbound.reply_to,
                    session_id: String::new(),
                    payload: OutboundPayload::Error(e.to_string()),
                }).await;
            }
        }
    }

    /// 处理单条消息的完整生命周期
    async fn process_message(
        &self,
        message: ChannelMessage,
        reply_to: &ChannelSource,
    ) -> Result<AgentResponse, AppError> {
        // 1. Session：加载或创建会话
        let session = self.session_mgr
            .get_or_create(&message.sender, &message.mode).await?;

        // 1.5 Agent Command 拦截（/new /stop /restart /status）
        if let Some(cmd_name) = parse_agent_command(&message.content) {
            if let Some(cmd) = self.agent_commands.get(cmd_name) {
                let ctx = AgentCommandContext {
                    session: session.clone(),
                    session_mgr: self.session_mgr.clone(),
                    sub_agent_tx: self.sub_agent_tx.clone(),
                };
                let result = cmd.execute(ctx).await?;
                self.bus.publish_outbound(OutboundMessage {
                    id: uuid(), target: reply_to.clone(),
                    session_id: session.id.clone(),
                    payload: OutboundPayload::Text(result.response.clone()),
                }).await;
                self.handle_action(result.action).await?;
                return Ok(AgentResponse::from_text(result.response));
            }
        }

        // 2. Context：组装完整 prompt
        let context = self.context_builder.build(&message, &session).await?;

        // 3. 选择模型
        let model = self.select_model(&message.mode);

        // 4. 调用 Provider（流式，通过 Bus 推送 chunk）
        let response = self.call_streamed(model, &context, &session, reply_to).await?;

        // 5. 工具调用循环（最多 10 轮）
        let final_response = self.tool_loop(response, &session, model, &channel).await?;

        // 6. Session：追加消息对 + 裁剪
        self.session_mgr.append(&session.id, &message, &final_response).await?;

        // 7. 后处理：写入 Memory 记忆、派发 SubAgent 任务
        self.post_process(&message, &final_response, &session).await?;

        Ok(final_response)
    }

    /// 工具调用循环
    async fn tool_loop(
        &self,
        mut response: ProviderResponse,
        session: &Session,
        model: ModelTier,
        channel: &Arc<dyn Channel>,
    ) -> Result<AgentResponse, AppError> {
        let mut iterations = 0;
        let mut seen_hashes = HashSet::new(); // 循环检测

        while let Some(tool_calls) = response.parse_tool_calls() {
            if iterations >= 10 { break; }

            // 循环检测：相同输出 hash 说明死循环
            let hash = hash_tool_calls(&tool_calls);
            if !seen_hashes.insert(hash) { break; }

            // 执行工具
            let results = self.tools.execute_batch(tool_calls).await;

            // 将工具结果追加到上下文，再次调用 Provider
            let context_with_tools = self.context_builder
                .append_tool_results(&session, &results);
            response = self.provider.chat(model, &context_with_tools).await?;

            iterations += 1;
        }

        Ok(AgentResponse::from(response))
    }

    /// 流式调用 Provider，逐 chunk 通过 Bus outbound 推送
    async fn call_streamed(
        &self,
        model: ModelTier,
        context: &[ChatMessage],
        session: &Session,
        reply_to: &ChannelSource,
    ) -> Result<ProviderResponse, AppError> {
        let mut stream = self.provider.chat_stream(model, context).await?;
        let mut full_response = String::new();

        self.bus.publish_outbound(OutboundMessage {
            id: uuid(), target: reply_to.clone(),
            session_id: session.id.clone(),
            payload: OutboundPayload::Typing(true),
        }).await;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            full_response.push_str(&chunk);
            self.bus.publish_outbound(OutboundMessage {
                id: uuid(), target: reply_to.clone(),
                session_id: session.id.clone(),
                payload: OutboundPayload::Chunk { content: chunk, done: false },
            }).await;
        }

        // 完成信号
        self.bus.publish_outbound(OutboundMessage {
            id: uuid(), target: reply_to.clone(),
            session_id: session.id.clone(),
            payload: OutboundPayload::Chunk { content: String::new(), done: true },
        }).await;

        Ok(ProviderResponse::from_text(full_response))
    }

    fn select_model(&self, mode: &ConversationMode) -> ModelTier {
        match mode {
            ConversationMode::Companion => ModelTier::Sonnet,
            ConversationMode::Knowledge => ModelTier::Sonnet,
            ConversationMode::Reflect   => ModelTier::Sonnet,
            ConversationMode::Challenge => ModelTier::Sonnet,
            ConversationMode::TreeHole  => ModelTier::Sonnet,
        }
    }
}
```

#### ContextBuilder — 上下文组装引擎

ContextBuilder 负责将分散的数据源组装为完整的 LLM prompt，并严格控制 token 预算。

```rust
// src-tauri/src/agent/context.rs

pub struct ContextBuilder {
    memory: Arc<MemoryManager>,       // 记忆层（观察召回 + 偏好注入）
    services: Arc<ServiceContainer>,  // 业务层（知识检索 RAG）
    db: Arc<DbState>,                // 用户角色等
}

impl ContextBuilder {
    /// 组装完整 prompt（返回 ChatMessage 数组）
    pub async fn build(
        &self,
        message: &ChannelMessage,
        session: &Session,
    ) -> Result<Vec<ChatMessage>, AppError> {
        let mut messages = Vec::new();

        // [1] System Prompt = 人格 + 模式指令 + 角色上下文 + 工具描述
        let system = self.build_system_prompt(&message.mode).await?;
        messages.push(ChatMessage::system(system));

        // [2] RAG 知识片段注入（top 5，每条 ≤500 tokens）
        let knowledge = self.services.knowledge
            .search(&message.content, 5).await?;
        if !knowledge.is_empty() {
            messages.push(ChatMessage::system(
                format_knowledge_context(&knowledge)
            ));
        }

        // [3] Memory 记忆召回：未浮出的观察 + 用户偏好
        let observations = self.memory.unsurfaced_observations(3).await?;
        if !observations.is_empty() {
            messages.push(ChatMessage::system(
                format_observations(&observations)
            ));
        }

        // [4] 压缩的对话历史（近 5 轮完整 + 早期摘要）
        messages.extend(session.compressed_history());

        // [5] 用户消息
        messages.push(ChatMessage::user(&message.content));

        // Token 预算检查
        self.enforce_budget(&mut messages, &message.mode)?;

        Ok(messages)
    }

    /// Token 预算控制
    fn enforce_budget(
        &self,
        messages: &mut Vec<ChatMessage>,
        mode: &ConversationMode,
    ) -> Result<(), AppError> {
        let budget = match self.select_tier(mode) {
            ModelTier::Haiku => 8_000,
            ModelTier::Sonnet => 30_000,
        };
        // 超预算时：先裁剪 RAG 片段数 → 再压缩历史 → 最后截断观察
        // ...
        Ok(())
    }

    /// 追加工具执行结果到上下文（用于工具调用循环）
    pub fn append_tool_results(
        &self,
        session: &Session,
        results: &[Result<ToolOutput, AppError>],
    ) -> Vec<ChatMessage>;
}
```

#### SessionManager — 会话生命周期管理

SessionManager 管理会话的创建、历史存取、裁剪和持久化。每个 sender 维护独立的会话上下文。

```rust
// src-tauri/src/agent/session.rs

pub struct Session {
    pub id: String,
    pub sender: String,
    pub mode: ConversationMode,
    pub messages: Vec<ChatMessage>,  // 内存中的活跃历史
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
}

impl Session {
    /// 返回压缩后的历史（近 5 轮完整 + 早期摘要）
    pub fn compressed_history(&self) -> Vec<ChatMessage>;
}

pub struct SessionManager {
    db: Arc<DbState>,
    max_turns: usize,      // 内存中保留的最大轮数（默认 50）
    keep_recent: usize,    // 裁剪时保护的近期轮数（默认 5）
}

impl SessionManager {
    /// 按 sender 获取或创建 session
    pub async fn get_or_create(
        &self, sender: &str, mode: &ConversationMode
    ) -> Result<Session, AppError>;

    /// 追加消息对（user + assistant），触发自动裁剪
    pub async fn append(
        &self, session_id: &str, user_msg: &ChannelMessage, agent_resp: &AgentResponse
    ) -> Result<(), AppError> {
        // 1. 追加到内存 + SQLite
        // 2. 如果超过 max_turns，触发 prune
        Ok(())
    }

    /// 历史裁剪：保护近 N 轮 + 系统消息，压缩早期为摘要
    async fn prune(&self, session: &mut Session) -> Result<(), AppError> {
        // Phase 1: 折叠工具调用/结果对
        // Phase 2: 用 Haiku 将早期消息压缩为摘要
        // Phase 3: 删除超龄原始消息
        Ok(())
    }

    /// 持久化 session 到 SQLite（追加消息时自动调用）
    async fn persist(&self, session: &Session) -> Result<(), AppError>;
}
```

PartnerClaw 是单用户桌面应用，sender 通常只有一个，但 Channel 抽象保证了来自不同通道的消息能正确路由到同一个 Agent 上下文。

### 6.6 Agent 初始化与接线

应用启动时，各模块组装并注入 AgentLoop：

```rust
// src-tauri/src/agent/mod.rs

pub fn init_agent(
    db: Arc<DbState>,
    provider: Arc<dyn Provider>,
    tools: Arc<ToolRegistry>,
    memory: Arc<MemoryManager>,
    services: Arc<ServiceContainer>,
    agent_commands: Arc<AgentCommandRegistry>,
    bus: Arc<MessageBus>,
) -> AgentLoop {
    // SubAgent 后台执行器
    let (sub_tx, sub_rx) = mpsc::channel(32);
    let sub_agent = SubAgentExecutor::new(provider.clone(), db.clone(), memory.clone());
    tokio::spawn(sub_agent.run(sub_rx));

    let session_mgr = Arc::new(SessionManager::new(db.clone()));
    let context_builder = Arc::new(ContextBuilder::new(
        memory.clone(), services.clone(), db.clone(),
    ));

    AgentLoop {
        bus,
        session_mgr,
        context_builder,
        provider,
        tools,
        memory,
        agent_commands,
        sub_agent_tx: sub_tx,
    }
}
```

```rust
// src-tauri/src/lib.rs（启动时接线）

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let db = init_database()?;
            let provider = create_provider(&settings)?;
            let memory = Arc::new(MemoryManager::new(db.clone()));
            let services = Arc::new(ServiceContainer::new(db.clone()));
            let tools = ToolRegistry::default_tools(&services, &memory, vault_path, mcp_configs);

            // MessageBus：Channel ↔ Agent 双向解耦
            let bus = Arc::new(MessageBus::new(64));

            let agent_commands = Arc::new(AgentCommandRegistry::default());
            let agent_loop = init_agent(
                db.clone(), provider, tools, memory.clone(),
                services.clone(), agent_commands, bus.clone(),
            );

            // 启动 AgentLoop（消费 inbound）
            let inbound_rx = bus.take_inbound_rx();
            tokio::spawn(agent_loop.run(inbound_rx));

            // 启动 Channel 出站分发（消费 outbound）
            let desktop_channel: Arc<dyn Channel> = Arc::new(DesktopChannel::new(app.handle()));
            let mut channels = HashMap::new();
            channels.insert(ChannelSource::Desktop, desktop_channel.clone());
            let outbound_rx = bus.take_outbound_rx();
            tokio::spawn(run_outbound_dispatcher(outbound_rx, channels));

            // 启动 Channel 入站监听
            let bus_clone = bus.clone();
            tokio::spawn(async move { desktop_channel.listen(bus_clone).await });

            // 注入 Tauri 状态
            app.manage(bus.clone());  // commands/conversation.rs 用 bus.publish_inbound()
            app.manage(db);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![...])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### 6.7 SubAgent — 异步子任务执行器

AgentLoop 负责处理主对话流，但有些任务不应阻塞对话响应，需要在后台独立完成。SubAgent 就是为这些异步任务设计的轻量执行器。

```
AgentLoop (主对话)
    │
    ├── 对话响应 → 立即返回给用户
    │
    └── 派发 SubAgent 任务（不阻塞）
         ├── CaptureRouteTask:  捕获分类（Haiku）
         ├── KnowledgeDistill:  从对话中提炼知识笔记
         ├── SessionSummarize:  会话摘要生成
         ├── ObservationAnalyze: Layer 3 模式识别
         └── DailySummary:      当日回顾生成
```

```rust
// src-tauri/src/agent/sub_agent.rs

/// 子任务类型
pub enum SubAgentTask {
    /// 捕获路由：对原始输入进行分类（Haiku 调用）
    CaptureRoute {
        capture_id: String,
        raw_content: String,
    },
    /// 知识蒸馏：从对话中提炼出值得沉淀的知识笔记
    KnowledgeDistill {
        session_id: String,
        messages: Vec<ChatMessage>,
    },
    /// 会话摘要：生成对话精华摘要
    SessionSummarize {
        session_id: String,
    },
    /// 观察分析：分析对话模式，记录 Layer 3 观察
    ObservationAnalyze {
        session_id: String,
        recent_messages: Vec<ChatMessage>,
    },
    /// 日记摘要：生成当日回顾
    DailySummary {
        date: String,
    },
}

pub struct SubAgentExecutor {
    provider: Arc<dyn Provider>,
    db: Arc<DbState>,
    memory: Arc<MemoryManager>,
    task_tx: mpsc::Sender<SubAgentTask>,
}

impl SubAgentExecutor {
    /// 启动后台任务消费循环
    pub async fn run(mut self, mut rx: mpsc::Receiver<SubAgentTask>) {
        while let Some(task) = rx.recv().await {
            // 每个任务在独立 tokio task 中执行，互不阻塞
            let executor = self.clone_refs();
            tokio::spawn(async move {
                if let Err(e) = executor.execute(task).await {
                    tracing::error!("SubAgent task failed: {}", e);
                }
            });
        }
    }

    async fn execute(&self, task: SubAgentTask) -> Result<(), AppError> {
        match task {
            SubAgentTask::CaptureRoute { capture_id, raw_content } => {
                // Haiku 调用：分类为 task/thought/feeling/link
                let result = self.classify(&raw_content).await?;
                self.db.update_capture_route(&capture_id, &result).await?;
            }
            SubAgentTask::KnowledgeDistill { session_id, messages } => {
                // Sonnet 调用：从对话中提炼知识
                let draft = self.distill_knowledge(&messages).await?;
                // 写入 vault/knowledge/ 草稿，等待人类确认
                self.memory.save_knowledge_draft(&draft).await?;
            }
            SubAgentTask::SessionSummarize { session_id } => {
                // Haiku 调用：生成会话摘要
                let summary = self.summarize_session(&session_id).await?;
                self.db.update_session_summary(&session_id, &summary).await?;
            }
            SubAgentTask::ObservationAnalyze { session_id, recent_messages } => {
                // Sonnet 调用：分析模式，发现盲区
                let observations = self.analyze_patterns(&recent_messages).await?;
                for obs in observations {
                    self.memory.record_observation(obs).await?;
                }
            }
            SubAgentTask::DailySummary { date } => {
                // Sonnet 调用：生成当日回顾
                let summary = self.generate_daily_summary(&date).await?;
                self.memory.append_to_daily(&date, &summary).await?;
            }
        }
        Ok(())
    }
}
```

**SubAgent 与 AgentLoop 的协作**：

```rust
// 在 AgentLoop.process_message() 中，对话完成后派发后台任务
async fn process_message(&self, message: ChannelMessage, channel: Arc<dyn Channel>)
    -> Result<AgentResponse, AppError>
{
    // ... 主对话处理 ...
    let response = /* ... */;

    // 对话完成后，异步派发 SubAgent 任务（不阻塞响应返回）
    if message.mode == ConversationMode::Knowledge {
        let _ = self.sub_agent_tx.send(SubAgentTask::KnowledgeDistill {
            session_id: session.id.clone(),
            messages: session.recent_messages(10),
        }).await;
    }

    // 每次对话后都尝试 Layer 3 观察分析
    let _ = self.sub_agent_tx.send(SubAgentTask::ObservationAnalyze {
        session_id: session.id.clone(),
        recent_messages: session.recent_messages(5),
    }).await;

    Ok(response) // 立即返回，SubAgent 后台运行
}
```

**SubAgent 模型选择**：

| 子任务 | 模型 | 原因 |
|--------|------|------|
| CaptureRoute | Haiku | 简单分类，低成本 |
| SessionSummarize | Haiku | 摘要生成，低成本 |
| KnowledgeDistill | Sonnet | 需要深度理解和提炼 |
| ObservationAnalyze | Sonnet | 需要跨域关联和模式识别 |
| DailySummary | Sonnet | 需要综合当日全部信息 |

### 6.8 消息处理流水线（完整）

```
┌────────────────────────────────────────────────────────────────┐
│                      完整消息流                                 │
├────────────────────────────────────────────────────────────────┤
│                                                                │
│  外部平台 (Desktop UI / Telegram / Feishu)                      │
│       │                                                        │
│       ▼                                                        │
│  Channel.listen() ──► bus.publish_inbound(InboundMessage)      │
│                                                                │
│  ┌──────────── MessageBus ────────────┐                        │
│  │  inbound queue ──► AgentLoop 消费   │                        │
│  │  outbound queue ◄── AgentLoop 推送  │                        │
│  └────────────────────────────────────┘                        │
│       │                                                        │
│       ▼ (inbound)                                              │
│  AgentLoop.process_message()                                   │
│       │                                                        │
│       ├─► SessionManager: 加载/创建 Session                    │
│       │                                                        │
│       ├─► Agent Command 拦截 (/new /stop /restart /status)     │
│       │     命中 → 执行控制指令 → bus.outbound 返回             │
│       │     未命中 → 继续正常对话流程 ↓                         │
│       │                                                        │
│       ├─► ContextBuilder: 组装完整 prompt                      │
│       │     [人格] + [模式指令] + [角色上下文]                   │
│       │     + [RAG 知识] + [压缩历史] + [记忆召回]              │
│       │     + Token 预算控制                                   │
│       │                                                        │
│       ├─► Provider.chat_stream()                               │
│       │     逐 chunk → bus.publish_outbound(Chunk)             │
│       │                                                        │
│       ├─► AgentLoop.tool_loop() (最多 10 轮)                   │
│       │     解析工具调用 → ToolRegistry.execute_batch()         │
│       │     → 结果注入上下文 → 再次调用 Provider               │
│       │     → 循环检测（hash 去重）→ 直到无工具请求             │
│       │                                                        │
│       ├─► SessionManager: 追加消息对 + 自动裁剪                │
│       │                                                        │
│       ├─► SubAgent 派发（异步，不阻塞响应返回）                 │
│       │     ├── KnowledgeDistill（知识模式下）                  │
│       │     ├── ObservationAnalyze（每次对话后）                │
│       │     └── SessionSummarize（会话结束时）                  │
│       │                                                        │
│       ▼ (outbound)                                             │
│  run_outbound_dispatcher() ──► 按 target 路由到 Channel        │
│       │                                                        │
│       ▼                                                        │
│  Channel.send(OutboundMessage) ──► 外部平台渲染                │
│                                                                │
└────────────────────────────────────────────────────────────────┘
```

### 6.9 Provider Layer — LLM 抽象（独立模块）

Provider 是独立于 Agent 的顶层模块，通过 trait 注入 AgentService。未来可替换为 OpenAI、Ollama 等实现。

```rust
// src-tauri/src/providers/traits.rs

pub enum ModelTier {
    Haiku,   // 路由、分类、简单任务（~1x 成本）
    Sonnet,  // 深度对话、知识沉淀、洞见生成（~10x 成本）
}

#[async_trait]
pub trait Provider: Send + Sync {
    /// 同步调用
    async fn chat(
        &self, model: ModelTier, messages: &[ChatMessage]
    ) -> Result<ProviderResponse, AppError>;

    /// 流式调用
    async fn chat_stream(
        &self, model: ModelTier, messages: &[ChatMessage]
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String, AppError>> + Send>>, AppError>;

    /// 能力查询
    fn supports_streaming(&self) -> bool;
    fn max_tokens(&self, model: ModelTier) -> usize;
}
```

```rust
// src-tauri/src/providers/claude.rs

pub struct ClaudeProvider {
    http_client: reqwest::Client,
    // API Key 运行时从 Keychain 获取，不持久化在结构体中
}

impl ClaudeProvider {
    pub async fn new() -> Result<Self, AppError> {
        let api_key = keychain::get("claude_api_key")?;
        // 验证 key 有效性...
    }
}
```

```rust
// src-tauri/src/providers/mod.rs

/// 工厂函数：根据配置创建 Provider 实例
pub fn create_provider(config: &AppSettings) -> Result<Arc<dyn Provider>, AppError> {
    match config.provider.as_str() {
        "claude" => Ok(Arc::new(ClaudeProvider::new()?)),
        // 未来可扩展：
        // "openai" => Ok(Arc::new(OpenAIProvider::new()?)),
        // "ollama" => Ok(Arc::new(OllamaProvider::new()?)),
        _ => Err(AppError::Validation("unknown provider".into())),
    }
}
```

### 6.10 Services Layer — 核心业务逻辑

Services 是业务操作的核心层。**Web Commands、CLI Commands 和 Agent 共用同一套 Services**，保证业务逻辑单一来源。

```
Web Commands  ──► Services ──► Storage
CLI Commands  ──► Services ──► Storage
Agent         ──► operations (元工具) ──► Services ──► Storage
                                     ──► Memory   ──► Storage
```

#### KnowledgeService — 知识笔记管理

操作人机共有的知识体系（Markdown 文件 + SQLite 索引）。

```rust
// src-tauri/src/services/knowledge.rs

pub struct KnowledgeService {
    storage: Arc<StorageManager>,
}

impl KnowledgeService {
    /// 创建知识笔记（写 Markdown + 更新 SQLite 索引）
    pub async fn create(&self, title: &str, content: &str, tags: &[String])
        -> Result<KnowledgeEntry, AppError>;

    /// 更新笔记内容（人类纠偏 或 Agent 沉淀）
    pub async fn update(&self, path: &str, content: &str)
        -> Result<(), AppError>;

    /// 搜索（FTS5 全文检索）
    pub async fn search(&self, query: &str, limit: u32)
        -> Result<Vec<KnowledgeEntry>, AppError>;

    /// 按标签/日期筛选
    pub async fn list(&self, tag: Option<&str>)
        -> Result<Vec<KnowledgeEntry>, AppError>;

    /// 获取完整笔记（Markdown 原文）
    pub async fn get(&self, path: &str)
        -> Result<KnowledgeNote, AppError>;

    /// 提取 wikilinks 并更新 links 表
    pub async fn sync_links(&self, path: &str)
        -> Result<(), AppError>;

    /// 重建索引（Markdown → SQLite）
    pub async fn rebuild_index(&self, path: Option<&str>)
        -> Result<(), AppError>;
}
```

#### DailyService — 日记管理

```rust
// src-tauri/src/services/daily.rs

pub struct DailyService {
    storage: Arc<StorageManager>,
}

impl DailyService {
    /// 获取日记（不存在则从模板创建）+ 关联任务
    pub async fn get(&self, date: &str)
        -> Result<DailyNote, AppError>;

    /// 保存日记内容
    pub async fn save(&self, date: &str, content: &str)
        -> Result<(), AppError>;

    /// 追加条目到日记指定区域
    pub async fn append_entry(&self, date: &str, content: &str, section: Option<&str>)
        -> Result<(), AppError>;

    /// 日记列表（元数据）
    pub async fn list(&self, limit: u32)
        -> Result<Vec<DailyMeta>, AppError>;
}
```

#### TaskService — 任务管理

```rust
// src-tauri/src/services/task.rs

pub struct TaskService {
    storage: Arc<StorageManager>,
}

impl TaskService {
    pub async fn create(&self, content: &str, due: Option<&str>, context: Option<&str>, note_path: Option<&str>)
        -> Result<Task, AppError>;
    pub async fn update(&self, id: &str, status: Option<&str>, content: Option<&str>, due: Option<&str>)
        -> Result<Task, AppError>;
    pub async fn list(&self, status: Option<&str>)
        -> Result<Vec<Task>, AppError>;
    pub async fn complete(&self, id: &str)
        -> Result<(), AppError>;
}
```

#### CaptureService — 捕获队列管理

```rust
// src-tauri/src/services/capture.rs

pub struct CaptureService {
    storage: Arc<StorageManager>,
}

impl CaptureService {
    pub async fn submit(&self, raw: &str, source: &str) -> Result<CaptureItem, AppError>;
    pub async fn list_pending(&self) -> Result<Vec<CaptureItem>, AppError>;
    pub async fn set_route(&self, id: &str, route: &str) -> Result<(), AppError>;
    pub async fn confirm_route(&self, id: &str, route: &str, adjusted: bool) -> Result<(), AppError>;
}
```

### 6.11 Memory Layer — Agent 私有记忆

> PRD 核心命题：**记忆是 Agent 的，知识是共同的。**

Memory 管理 Agent 对用户的私有认知——观察、偏好、模式识别等。这些信息存在 SQLite 中，用户不直接操作。Knowledge（Markdown）是人机共有的，由 Services 管理。

```
Memory (Agent 私有, SQLite)          Knowledge (人机共有, Markdown)
├── 观察：第三次提到工作疲惫感        ├── vault/knowledge/工作节奏.md
├── 偏好：偏好简短直接的回复           ├── vault/knowledge/投资策略.md
├── 模式：周一情绪通常低落             └── vault/knowledge/育儿方法.md
└── 召回：按相关性检索记忆
                                      ↑
    记忆可以升华为知识 ────────────────┘
    （Agent 发现模式 → 沉淀为知识笔记，需人类确认）
```

#### 单表 `memories` 设计

所有记忆存入单表，通过 `category` 区分类型，`key` 去重，`superseded_by` 追踪认知演进：

```rust
// src-tauri/src/memory/mod.rs

/// 统一记忆结构（对应 memories 表）
pub struct Memory {
    pub id: String,
    pub key: String,                        // 唯一去重键，同一认知 upsert
    pub content: String,                    // 记忆内容
    pub category: MemoryCategory,           // observation | preference | pattern
    pub memory_type: Option<String>,        // 子类型
    pub namespace: String,                  // 上下文隔离（default / companion / reflect）
    pub importance: f32,                    // 重要度 0.0-1.0（衰减基准、recall 排序）
    pub session_id: Option<String>,         // 关联会话（溯源）
    pub related_path: Option<String>,       // 关联笔记路径
    pub surfaced: bool,                     // 是否已浮出给用户
    pub superseded_by: Option<String>,      // 被哪条新记忆替代
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub enum MemoryCategory {
    Observation,  // 模式识别、盲区发现、跨域关联
    Preference,   // 沟通风格、兴趣倾向、角色薄弱点
    Pattern,      // 对话频率、情绪曲线、主题热度
}
```

#### MemoryManager — 统一入口

```rust
pub struct MemoryManager {
    db: Arc<DbState>,
}

impl MemoryManager {
    /// 写入记忆（upsert by key，旧记忆标记 superseded_by）
    pub async fn remember(&self, memory: Memory) -> Result<(), AppError> {
        // 如果 key 已存在：旧记忆.superseded_by = 新记忆.id，再 insert 新记忆
        // 认知演进而非覆盖
    }

    /// 记忆召回：按 importance 排序，过滤 superseded_by IS NULL
    pub async fn recall(&self, query: &str, limit: u32) -> Result<Vec<Memory>, AppError> {
        // Phase 1: FTS5 关键词匹配 + importance 排序
        // Phase 2: embedding 向量语义检索
    }

    /// 按 category 召回
    pub async fn recall_by_category(&self, category: MemoryCategory, limit: u32)
        -> Result<Vec<Memory>, AppError>;

    /// 获取未浮出的记忆（ContextBuilder 注入 prompt 用）
    pub async fn unsurfaced(&self, limit: u32) -> Result<Vec<Memory>, AppError> {
        // WHERE surfaced = 0 AND superseded_by IS NULL ORDER BY importance DESC
    }

    /// 标记已浮出
    pub async fn mark_surfaced(&self, id: &str) -> Result<(), AppError>;

    /// 记忆衰减：降低旧记忆的 importance（Cron 定期调用）
    pub async fn decay(&self) -> Result<u32, AppError> {
        // UPDATE memories SET importance = importance * 0.95
        // WHERE superseded_by IS NULL AND importance > 0.1
        // 返回受影响行数
    }

    /// 记忆升华：高 importance 观察 → 知识笔记草稿
    pub async fn propose_crystallization(&self, id: &str) -> Result<KnowledgeDraft, AppError> {
        // 取出记忆 → 生成知识草稿 → 等人类确认后写入 vault/knowledge/
    }

    /// 清理：importance 低于阈值的旧记忆
    pub async fn cleanup(&self, threshold: f32) -> Result<u32, AppError> {
        // DELETE WHERE importance < threshold AND superseded_by IS NOT NULL
    }
}
```

#### 记忆类别与子类型

| category | 子类型 (type) | 说明 | key 示例 |
|----------|-------------|------|---------|
| **observation** | pattern | "这是第三次提到工作疲惫感" | `obs:work_fatigue_pattern` |
| **observation** | insight | "工作压力与陪孩子质量高度相关" | `obs:work_parenting_correlation` |
| **observation** | blindspot | "用户从未考虑过健康问题" | `obs:health_blindspot` |
| **observation** | emotion | "周一情绪持续低落" | `obs:monday_mood_low` |
| **preference** | communication_style | "偏好直接简洁的沟通方式" | `pref:communication_style` |
| **preference** | interest_topic | "对教育方法论很感兴趣" | `pref:interest_education` |
| **pattern** | emotion_trend | "近两周焦虑情绪上升" | `pat:emotion_trend_2w` |
| **pattern** | topic_frequency | "「创业」话题本月提及 12 次" | `pat:topic_startup_monthly` |
| **pattern** | engagement | "晚上 10 点后对话质量最高" | `pat:engagement_peak_time` |

#### 认知演进链（superseded_by）

```
记忆 A: "用户对教育有兴趣" (importance: 0.6)
  ↓ 新对话后 Agent 理解更深
记忆 B: "用户关注蒙特梭利教育方法，孩子 3 岁" (importance: 0.8)
  A.superseded_by = B.id

recall() 只返回 B（superseded_by IS NULL）
但 A 仍保留在库中，可追溯认知变化
```

#### 记忆生命周期

```
写入 → 演进 → 衰减 → 升华/清理

1. 写入：SubAgent 对话后分析 → remember() upsert by key
2. 演进：同一 key 的新认知替代旧认知（superseded_by 链）
3. 衰减：Cron 定期 decay()，importance *= 0.95
4. 升华：高 importance 观察 → propose_crystallization()
         → 知识笔记草稿 → 人类确认 → vault/knowledge/
5. 清理：被替代 + importance < 阈值的旧记忆 cleanup()
```

#### Memory 与 ContextBuilder 的关系

```rust
// ContextBuilder 从 Memory 拉取记忆注入 prompt
let memories = self.memory.recall(&message.content, 5).await?;
let unsurfaced = self.memory.unsurfaced(3).await?;
// → 注入 System Prompt 的 [Agent 记忆] 区域
```

### 6.12 Tool Layer — Agent 可用工具

Agent 上下文**常驻仅 4 个 Tool Schema**，业务操作通过 `operations` 元工具按需发现和调用，避免上下文膨胀。

```
Tools（常驻上下文，4 个 Schema）
├── filesystem   → 文件系统操作
├── shell        → 受限命令执行
├── mcp_client   → 外部 MCP 工具
└── operations   → 元工具（按需发现 Services + Memory 操作）
        ├── operations.list(category?) → 返回可用操作及参数 Schema
        └── operations.call(name, args) → 执行具体操作
```

#### Tool Trait

```rust
// src-tauri/src/tools/traits.rs

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn json_schema(&self) -> serde_json::Value;
    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, AppError>;
}
```

#### 基础能力工具

| 工具 | 文件 | 操作 | 安全约束 |
|------|------|------|---------|
| **filesystem** | `filesystem.rs` | read/write/append/list/move/delete | vault 内限定，private/ 禁入，审计日志 |
| **shell** | `shell.rs` | exec (白名单) | 白名单命令，禁管道/重定向，30s 超时，10KB 输出截断 |
| **mcp_client** | `mcp_client.rs` | call_tool, list_tools | MCP 协议调用外部工具服务 |

#### MCP Client — 接入外部工具服务

Agent 作为 MCP Client，通过 MCP 协议调用外部工具服务（如浏览器、日历、邮件等）。

```rust
// src-tauri/src/tools/mcp_client.rs

pub struct McpClientTool {
    connections: HashMap<String, McpConnection>,  // server_name → connection
}

impl McpClientTool {
    /// 连接 MCP Server（启动时从配置读取）
    pub async fn connect(&mut self, name: &str, config: McpServerConfig) -> Result<(), AppError>;

    /// 列举所有已连接 Server 的可用工具
    pub fn list_tools(&self) -> Vec<ToolSpec>;

    /// 调用外部工具
    pub async fn call_tool(&self, server: &str, tool: &str, args: serde_json::Value)
        -> Result<ToolOutput, AppError>;
}
```

MCP Server 配置（`config/settings.json`）：

```json
{
  "mcp_servers": [
    { "name": "browser", "command": "npx", "args": ["@anthropic/mcp-browser"] },
    { "name": "calendar", "command": "npx", "args": ["@anthropic/mcp-calendar"] }
  ]
}
```

#### Operations — 业务操作元工具

`operations` 是连接 Agent 与 Services/Memory 的唯一通道。Agent 通过 `list` 按需发现可用操作（含参数 Schema），再通过 `call` 执行。**操作的 JSON Schema 不常驻上下文，仅在 list 时返回**。

```rust
// src-tauri/src/tools/operations.rs

pub struct OperationsTool {
    services: Arc<ServiceContainer>,
    memory: Arc<MemoryManager>,
    registry: OperationRegistry,
}

/// 单个操作定义（Schema 按需返回，不常驻上下文）
pub struct OperationDef {
    pub name: String,           // "knowledge_create"
    pub category: String,       // "knowledge"
    pub description: String,
    pub parameters: Value,      // JSON Schema
}

impl OperationsTool {
    pub fn new(services: Arc<ServiceContainer>, memory: Arc<MemoryManager>) -> Self {
        let registry = Self::build_registry();
        Self { services, memory, registry }
    }

    fn build_registry() -> OperationRegistry {
        let mut r = OperationRegistry::new();
        // Knowledge
        r.register("knowledge_create", "knowledge", "创建知识笔记",
            json!({"properties": {"title": {"type": "string"}, "content": {"type": "string"}, "tags": {"type": "array"}}}));
        r.register("knowledge_search", "knowledge", "搜索知识库",
            json!({"properties": {"query": {"type": "string"}, "limit": {"type": "integer", "default": 5}}}));
        r.register("knowledge_get", "knowledge", "获取知识笔记全文",
            json!({"properties": {"path": {"type": "string"}}}));
        // Daily
        r.register("daily_get", "daily", "获取/创建日记",
            json!({"properties": {"date": {"type": "string"}}}));
        r.register("daily_append", "daily", "追加内容到日记",
            json!({"properties": {"date": {"type": "string"}, "content": {"type": "string"}, "section": {"type": "string"}}}));
        // Task
        r.register("task_create", "task", "创建任务",
            json!({"properties": {"content": {"type": "string"}, "due": {"type": "string"}, "context": {"type": "string"}}}));
        r.register("task_list", "task", "列出任务",
            json!({"properties": {"status": {"type": "string"}}}));
        r.register("task_complete", "task", "完成任务",
            json!({"properties": {"id": {"type": "string"}}}));
        // Capture
        r.register("capture_submit", "capture", "快速捕获",
            json!({"properties": {"raw": {"type": "string"}, "source": {"type": "string"}}}));
        // Search（跨知识 + 记忆）
        r.register("memory_search", "search", "搜索 Agent 记忆（观察/偏好/模式）",
            json!({"properties": {"query": {"type": "string"}, "limit": {"type": "integer", "default": 5}}}));
        r
    }
}

#[async_trait]
impl Tool for OperationsTool {
    fn name(&self) -> &str { "operations" }
    fn description(&self) -> &str {
        "业务操作元工具。先用 list 查看可用操作及参数，再用 call 执行。"
    }
    fn json_schema(&self) -> Value {
        // 常驻上下文的 Schema 非常小
        json!({
            "type": "object",
            "properties": {
                "action": { "enum": ["list", "call"] },
                "category": {
                    "type": "string",
                    "description": "筛选类别: knowledge | daily | task | capture | search"
                },
                "name": {
                    "type": "string",
                    "description": "操作名称（call 时必填）"
                },
                "args": {
                    "type": "object",
                    "description": "操作参数（call 时必填）"
                }
            },
            "required": ["action"]
        })
    }

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, AppError> {
        match input.args["action"].as_str() {
            Some("list") => {
                let category = input.args["category"].as_str();
                let ops = self.registry.list(category);
                // 返回操作列表 + 参数 Schema（此时才注入上下文）
                Ok(ToolOutput::success(serde_json::to_string_pretty(&ops)?))
            }
            Some("call") => {
                let name = input.args["name"].as_str()
                    .ok_or(AppError::Validation("name required".into()))?;
                let args = input.args.get("args").cloned().unwrap_or(json!({}));
                self.dispatch(name, args).await
            }
            _ => Err(AppError::Validation("action must be 'list' or 'call'".into()))
        }
    }
}

impl OperationsTool {
    async fn dispatch(&self, name: &str, args: Value) -> Result<ToolOutput, AppError> {
        match name {
            // Knowledge
            "knowledge_create" => {
                let entry = self.services.knowledge.create(
                    args["title"].as_str().unwrap_or_default(),
                    args["content"].as_str().unwrap_or_default(),
                    &[],
                ).await?;
                Ok(ToolOutput::success(format!("Created: {}", entry.path)))
            }
            "knowledge_search" => {
                let results = self.services.knowledge.search(
                    args["query"].as_str().unwrap_or_default(),
                    args["limit"].as_u64().unwrap_or(5) as u32,
                ).await?;
                Ok(ToolOutput::success(serde_json::to_string(&results)?))
            }
            "knowledge_get" => {
                let note = self.services.knowledge.get(
                    args["path"].as_str().unwrap_or_default(),
                ).await?;
                Ok(ToolOutput::success(serde_json::to_string(&note)?))
            }
            // Daily
            "daily_get" => {
                let note = self.services.daily.get(
                    args["date"].as_str().unwrap_or_default(),
                ).await?;
                Ok(ToolOutput::success(serde_json::to_string(&note)?))
            }
            "daily_append" => {
                self.services.daily.append_entry(
                    args["date"].as_str().unwrap_or_default(),
                    args["content"].as_str().unwrap_or_default(),
                    args["section"].as_str(),
                ).await?;
                Ok(ToolOutput::success("Appended".into()))
            }
            // Task
            "task_create" => {
                let task = self.services.task.create(
                    args["content"].as_str().unwrap_or_default(),
                    args["due"].as_str(),
                    args["context"].as_str(),
                    None,
                ).await?;
                Ok(ToolOutput::success(format!("Task created: {}", task.id)))
            }
            "task_list" => {
                let tasks = self.services.task.list(args["status"].as_str()).await?;
                Ok(ToolOutput::success(serde_json::to_string(&tasks)?))
            }
            "task_complete" => {
                self.services.task.complete(
                    args["id"].as_str().unwrap_or_default(),
                ).await?;
                Ok(ToolOutput::success("Task completed".into()))
            }
            // Capture
            "capture_submit" => {
                let item = self.services.capture.submit(
                    args["raw"].as_str().unwrap_or_default(),
                    args["source"].as_str().unwrap_or("agent"),
                ).await?;
                Ok(ToolOutput::success(format!("Captured: {}", item.id)))
            }
            // Search（记忆）
            "memory_search" => {
                let results = self.memory.recall(
                    args["query"].as_str().unwrap_or_default(),
                    args["limit"].as_u64().unwrap_or(5) as u32,
                ).await?;
                Ok(ToolOutput::success(serde_json::to_string(&results)?))
            }
            _ => Err(AppError::Validation(format!("unknown operation: {}", name)))
        }
    }
}
```

#### Agent 调用流程示例

```
场景：Agent 需要搜索用户的学习相关知识

  Round 1: tool_call("operations", {action: "list", category: "knowledge"})
  返回:
    knowledge_create(title, content, tags) — 创建知识笔记
    knowledge_search(query, limit) — 搜索知识库
    knowledge_get(path) — 获取笔记全文

  Round 2: tool_call("operations", {action: "call", name: "knowledge_search", args: {query: "学习方法", limit: 5}})
  返回:
    [{path: "knowledge/learning.md", title: "有效学习方法", snippet: "..."}]

  Round 3: tool_call("operations", {action: "call", name: "knowledge_get", args: {path: "knowledge/learning.md"}})
  返回:  完整 Markdown 内容
```

注意：熟练的 Agent 可以跳过 `list` 直接 `call`（操作名称是确定性的），`list` 主要用于 Agent 初次使用或不确定有哪些操作时。

#### ToolRegistry

```rust
// src-tauri/src/tools/mod.rs

impl ToolRegistry {
    pub fn default_tools(
        services: &ServiceContainer,
        memory: &MemoryManager,
        vault_path: PathBuf,
        mcp_configs: Vec<McpServerConfig>,
    ) -> Self {
        let tools: Vec<Arc<dyn Tool>> = vec![
            Arc::new(FilesystemTool::new(vault_path)),
            Arc::new(ShellTool::new_sandboxed()),
            Arc::new(McpClientTool::new(mcp_configs)),
            Arc::new(OperationsTool::new(
                Arc::new(services.clone()),
                Arc::new(memory.clone()),
            )),
        ];
        Self { tools }
    }
}
```

**工具调用循环**（在 AgentLoop.tool_loop() 中）：

```
LLM 响应 → 解析工具调用 → ToolRegistry.execute_batch()
    → 将工具结果追加到上下文 → 再次调用 Provider
    → 重复直到 LLM 不再请求工具（最多 10 轮）
    → 循环检测：输出 hash 去重，防止无限循环
```

### 6.13 Gateway Layer — HTTP/WebSocket 服务（独立模块）

Gateway 是桌面端对外暴露的网络服务层。为移动端 PWA 提供静态文件和 API，为 Webhook 通道提供接入点。

```rust
// src-tauri/src/gateway/mod.rs

pub struct GatewayServer {
    agent: Arc<AgentService>,
    channels: Vec<Arc<dyn Channel>>,
    auth: AuthGuard,
    port: u16,  // 默认 7878，可配置
}

impl GatewayServer {
    /// 启动 HTTP + WebSocket 服务
    pub async fn start(&self) -> Result<(), AppError> {
        // 绑定本地端口，启动 axum/actix-web 服务
    }
}
```

**REST API 端点**：

| 端点 | 方法 | 说明 | Phase |
|------|------|------|-------|
| `/api/chat` | POST | 发送消息，返回 Agent 响应 | Phase 1 后期 |
| `/api/daily/:date` | GET | 获取日记内容 | Phase 2 |
| `/api/knowledge` | GET | 知识库搜索 | Phase 2 |
| `/api/tasks` | GET | 任务列表 | Phase 2 |
| `/api/capture` | POST | 提交捕获（Webhook 入口） | Phase 1 后期 |
| `/ws/chat` | WS | WebSocket 实时对话 | Phase 2 |
| `/webhook/telegram` | POST | Telegram Bot Webhook 接收 | Phase 1 后期 |
| `/webhook/feishu` | POST | 飞书 Bot Webhook 接收 | Phase 2 |
| `/` | GET | PWA 静态文件服务 | Phase 2 |

**认证**（`gateway/auth.rs`）：

```rust
pub struct AuthGuard {
    bearer_token: String,  // 简单 Bearer Token，存 settings.json
}

impl AuthGuard {
    /// 验证请求：Header / Query Param / WebSocket Subprotocol
    pub fn verify(&self, request: &Request) -> Result<(), AppError>;
}
```

- 本地 WiFi 直连：Bearer Token 认证（用户在桌面端设置中生成）
- Tailscale 穿透：Tailscale 本身提供加密 + 身份验证，Gateway 再加 Token 双重保护
- Webhook：平台签名验证（Telegram: X-Telegram-Bot-Api-Secret-Token, 飞书: 签名校验）

### 6.14 Cron — 定时任务调度（独立模块）

Agent 不仅被动响应用户消息，还需主动执行后台任务。Cron 模块基于 tokio 定时驱动。

```rust
// src-tauri/src/cron/mod.rs

pub struct CronScheduler {
    agent: Arc<AgentService>,
    db: Arc<DbState>,
    jobs: Vec<CronJob>,
}

pub struct CronJob {
    pub name: String,
    pub schedule: CronSchedule,  // cron 表达式
    pub enabled: bool,
    pub last_run: Option<DateTime<Utc>>,
}
```

**内置定时任务**：

| 任务 | 默认频率 | 说明 | Phase |
|------|---------|------|-------|
| `daily_summary` | 每日 22:00 | Agent 生成当日回顾摘要，写入日记 | MVP |
| `capture_process` | 每 5 分钟 | 处理未路由的捕获队列 | MVP |
| `history_prune` | 每日 03:00 | 压缩旧对话历史，超 90 天转冷归档 | Phase 2 |
| `knowledge_review` | 每周日 10:00 | Agent 回顾知识库，发现新关联（Layer 2） | Phase 2 |
| `index_rebuild` | 每日 04:00 | 增量重建 SQLite 索引（Markdown → notes 表） | MVP |
| `observation_surface` | 每日 09:00 | 检查 Layer 3 观察是否到浮出时机 | Phase 2 |
| `heartbeat_check` | 每 30 秒 | 系统健康检测 | MVP |

```rust
// src-tauri/src/cron/scheduler.rs

impl CronScheduler {
    /// 启动调度循环（应用启动时调用）
    pub async fn start(&mut self) {
        for job in &self.jobs {
            if !job.enabled { continue; }
            let agent = self.agent.clone();
            let db = self.db.clone();
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(job.schedule.next_interval()).await;
                    if let Err(e) = Self::run_job(&job.name, &agent, &db).await {
                        tracing::error!("cron job {} failed: {}", job.name, e);
                    }
                }
            });
        }
    }

    async fn run_job(name: &str, agent: &AgentService, db: &DbState) -> Result<(), AppError> {
        match name {
            "daily_summary" => { /* Agent 生成日记摘要 */ }
            "capture_process" => { /* 批量处理捕获队列 */ }
            "index_rebuild" => { /* 增量同步 Markdown → SQLite */ }
            _ => {}
        }
        Ok(())
    }
}
```

### 6.15 Heartbeat — 健康检测与系统状态

Heartbeat 持续监控系统各组件的运行状态，确保服务可靠性。

```rust
// src-tauri/src/heartbeat/mod.rs

pub struct HeartbeatMonitor {
    db: Arc<DbState>,
    provider: Arc<dyn Provider>,
    gateway: Option<Arc<GatewayServer>>,
    channels: Vec<Arc<dyn Channel>>,
}

/// 系统健康状态
pub struct SystemHealth {
    pub status: HealthStatus,          // healthy | degraded | down
    pub db_connected: bool,            // SQLite 连接正常
    pub api_key_valid: bool,           // Claude API Key 存在且可用
    pub vault_accessible: bool,        // Vault 目录可读写
    pub gateway_running: bool,         // Gateway 服务运行中
    pub channels: Vec<ChannelHealth>,  // 各通道状态
    pub last_check: DateTime<Utc>,
    pub uptime_seconds: u64,
}

pub struct ChannelHealth {
    pub name: String,
    pub connected: bool,
    pub last_message: Option<DateTime<Utc>>,
}

impl HeartbeatMonitor {
    /// 执行一次健康检查
    pub async fn check(&self) -> SystemHealth;

    /// 通道重连（带指数退避：2s → 4s → 8s → ... → 60s）
    pub async fn reconnect_channel(&self, name: &str) -> Result<(), AppError>;
}
```

**前端集成**：通过 IPC 命令 `system_health` 查询，Settings 页面展示系统状态。

### 6.16 System Prompt 组装

```
┌─────────────────────────────────────────────┐
│ [1] 基础人格                                 │ 固定
│     PartnerClaw 身份、沟通风格                │
├─────────────────────────────────────────────┤
│ [2] 模式指令                                 │ 按当前模式切换
│     陪伴 / 反思 / 挑战 / 知识 / 树洞          │
├─────────────────────────────────────────────┤
│ [3] 用户角色上下文                            │ 从 user_roles 表读取
│     角色、薄弱点、优先级                       │
├─────────────────────────────────────────────┤
│ [4] RAG 知识片段                             │ 动态检索，3-5 条
│     每条截断至 ~500 tokens                    │
├─────────────────────────────────────────────┤
│ [5] 压缩对话历史                             │ 动态
│     近 5 轮完整 + 早期摘要                    │
├─────────────────────────────────────────────┤
│ [6] Agent 观察                               │ Layer 3 候选
│     未浮出的模式识别                          │
├─────────────────────────────────────────────┤
│ [7] 用户消息                                 │
└─────────────────────────────────────────────┘
```

### 6.17 模型分层调用

| 任务类型 | 模型 | 成本比 |
|---------|------|--------|
| 内容分类 · 路由 · 任务提取 | Haiku | 1x |
| 日常捕获处理 · 简单提醒判断 | Haiku | 1x |
| 知识沉淀 · 综合分析 · 异步总结 | Sonnet | ~10x |
| Layer 3 洞见生成 · 深度对话 | Sonnet | ~10x |

### 6.18 上下文工程

Token 管理是核心产品能力：

| 策略 | 实现 |
|------|------|
| 知识库注入 | FTS5/向量检索 top 5 片段，不全量灌入 |
| 对话历史 | 近 5 轮完整 + Haiku 压缩早期为摘要 |
| Token 预算 | Haiku 调用 ≤ 8K tokens，Sonnet 调用 ≤ 30K tokens |

---

## 七、安全架构

### CSP 策略

`tauri.conf.json` 中设置 Content Security Policy：

```json
{
  "app": {
    "security": {
      "csp": "default-src 'self'; script-src 'self'; connect-src 'self' https://api.anthropic.com"
    }
  }
}
```

仅允许本地内容和 Claude API 请求。

### 私密区隔离

- `vault/private/` 路径下的所有文件，Agent 不可见
- Rust storage 模块在读取 Markdown 供 Agent 使用时，显式拒绝 `private/` 路径前缀
- 私密区内容永不进入 SQLite 索引、不参与 RAG 检索、不出现在任何 IPC 响应中

### Tauri Capabilities

`capabilities/default.json` 需声明：

```json
{
  "identifier": "default",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "opener:default",
    "fs:read-files",
    "fs:write-files"
  ]
}
```

文件系统权限限定在 vault 和 data 目录范围内。

### 树洞模式特殊处理

- 原始消息保留时间更短（用户可配置或手动清除）
- 摘要只存 `memories` 表（category='observation'），不生成人类可读摘要
- 内容永不进入共有知识库

---

## 八、MVP 范围

### Phase 1（MVP）包含

| 能力 | 模块 | 说明 |
|------|------|------|
| 快速捕获 + Agent 路由 | capture, agent::router | Haiku 分类，人类审核确认 |
| 日记视图 + 嵌入式任务 | daily, tasks | Daily Note 为锚点，任务一等公民 |
| 基础对话 | conversation, agent::core | 陪伴 + 知识两种模式 |
| 知识库浏览与搜索 | knowledge | 关键词搜索（FTS5） |
| 设置 + API Key | settings, keychain | BYOK Claude API |
| SQLite 存储 | storage::database | Schema 迁移、基础 CRUD |
| Markdown 读写 | storage::markdown | 日记和知识笔记 |
| Provider 层 | providers::claude | Claude API Haiku/Sonnet 调用 |
| 基础工具 | tools::search, file_ops | 知识库搜索 + Markdown 文件操作 |
| Cron 基础任务 | cron | capture_process, index_rebuild, daily_summary |
| Heartbeat | heartbeat | 系统健康检测（DB、Vault、API Key） |
| 统一错误处理 | error.rs | AppError → 前端展示 |
| 结构化日志 | tracing crate | 开发调试 |

### Phase 1 后期

| 能力 | 模块 | 说明 |
|------|------|------|
| Telegram Bot 通道 | channels::telegram | 移动端对话通道（最低开发成本） |
| Gateway 基础 | gateway::api | Webhook 接收（Telegram）+ 简单 chat API |
| Gateway 认证 | gateway::auth | Bearer Token + Telegram 签名验证 |

### Phase 2（延期）

| 能力 | 说明 |
|------|------|
| sqlite-vss 向量搜索 | MVP 阶段用 FTS5 关键词搜索替代 |
| 反思 / 挑战 / 树洞模式 | 陪伴 + 知识模式验证后再扩展 |
| Layer 3 认知循环 | 需要积累足够数据才有意义 |
| 分析 / 写作工具 | tools::analysis, writer，需 Agent 能力成熟后再加 |
| 角色模版冷启动 | 可手动设置角色，模版系统后补 |
| 飞书 Bot 通道 | channels::feishu |
| Gateway WebSocket | ws.rs，实时对话 |
| PWA 移动查看 | Gateway 提供静态文件服务 |
| Tailscale 远程穿透 | 移动端远程接入 |
| JSONL 冷归档 | history_prune cron，90 天后才需要 |
| Cron 高级任务 | knowledge_review, observation_surface |
| Agent 主动推送 | 异步日志与浮出机制 |
| 知识图谱可视化 | — |
| 本地 Embedding 模型 | 用 API embedding 或延期向量搜索 |
| 多 Provider 支持 | OpenAI, Ollama 等 |

---

## 九、技术依赖

### Rust（Cargo.toml 新增）

```toml
[dependencies]
# 存储
rusqlite = { version = "0.31", features = ["bundled", "fts5"] }

# 网络
reqwest = { version = "0.12", features = ["json", "stream"] }
axum = { version = "0.8", features = ["ws"] }           # Gateway HTTP/WS 服务
tower-http = { version = "0.6", features = ["cors", "fs"] }  # 静态文件 + CORS

# 异步运行时
tokio = { version = "1", features = ["full"] }
tokio-stream = "0.1"                                      # Stream 工具
async-trait = "0.1"

# 安全
keyring = "3"

# 可观测性
tracing = "0.1"
tracing-subscriber = "0.3"

# 工具
chrono = { version = "0.4", features = ["serde"] }
serde_yaml = "0.9"
uuid = { version = "1", features = ["v4"] }
futures = "0.3"                                            # Stream trait
```

已有依赖：`tauri 2`, `tauri-plugin-opener 2`, `serde 1`, `serde_json 1`

### 前端（package.json 新增）

```json
{
  "zustand": "^5",
  "react-markdown": "^9",
  "date-fns": "^4",
  "@tauri-apps/plugin-fs": "^2"
}
```

路由方案：MVP 页面仅 5 个，使用 Zustand 状态管理当前页面即可，无需引入路由库。
样式方案：保持 CSS 方案，按需引入 Tailwind（团队决策点）。

已有依赖：`react 19`, `react-dom 19`, `@tauri-apps/api ^2`, `@tauri-apps/plugin-opener ^2`
