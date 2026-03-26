# MindClaw 技术架构设计

> 拆分自 architecture.md，完整索引见 [README.md](./README.md)

## 三、三层命令架构

系统提供三种命令入口，覆盖不同使用场景，底层共享 Services 层：

```
React Frontend ── invoke() ──► Web Commands ──► Services ──► Storage
对话中 /xxx ─────────────────► Agent Commands ─► Agent 生命周期控制
终端 mindclaw ─────────────► CLI Commands ──► Services ──► Storage
```

### 跨层命令矩阵

| 维度 | Web Commands | Agent Commands | CLI Commands |
|------|-------------|----------------|-------------|
| 入口 | React `invoke()` | 对话消息 `/xxx` | 终端 `mindclaw` |
| 职责 | 业务 CRUD（完整） | Agent 生命周期控制 | 自动化/脚本操作 |
| 数量 | ~28 个 | 4 个 | ~7 个 |
| 调用链 | Command → Services | AgentLoop 拦截 → Agent 自身 | CliRuntime → Services |
| 需要 Tauri | 是 | 是（运行在 AgentLoop 内） | 否（独立二进制） |
| 需要 LLM | 否（除 capture_route） | 否（纯控制） | 仅 chat 子命令 |

### 3.1 Web Commands — Tauri IPC（前端调用）

所有前后端通信通过 Tauri `invoke()` 和 Event System 完成。命令统一返回 `Result<T, AppError>`。

调用链：`React invoke() → #[tauri::command] → Services → Storage`

#### Resource（资源）

| 命令 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `resource_submit` | `{ uri: String, type: String }` | `Resource` | 提交资源（URL/文件） |
| `resource_list` | `{ status: Option<String> }` | `Vec<Resource>` | 资源列表（可按状态过滤） |
| `resource_retry` | `{ id: String }` | `()` | 重试失败的资源解析 |

#### Conversation（对话）

| 命令 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `conversation_send` | `{ message: String }` | `String`（session_id） | 发起对话，响应通过 Event 流式推送 |
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
    pub cancel_token: CancellationToken,  // /stop 可触发取消正在进行的操作
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
#[command(name = "mindclaw", about = "MindClaw CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: CliCommand,

    /// 指定 vault 路径（默认 ~/MindClaw）
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
        let key = keyring::Entry::new("mindclaw", "api_key")?.get_password()?;
        self.provider = Some(Arc::new(ClaudeProvider::from_key(&key)));
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
name = "mindclaw"
path = "src/bin/cli.rs"

[dependencies]
clap = { version = "4", features = ["derive"] }
```

#### 命令清单

| 子命令 | 调用 Service | 说明 |
|--------|-------------|------|
| `mindclaw capture "text"` | CaptureService.submit() | source="cli" |
| `mindclaw daily [date]` | DailyService.get() | 输出 Markdown 到 stdout |
| `mindclaw search "query"` | KnowledgeService.search() | 列表输出 |
| `mindclaw task create "content"` | TaskService.create() | 创建任务 |
| `mindclaw task list [--status done]` | TaskService.list() | 列出任务 |
| `mindclaw task complete <id>` | TaskService.update() | 完成任务 |
| `mindclaw chat "message"` | Provider.chat() | 单轮对话，需 API Key |
| `mindclaw status` | HeartbeatMonitor (lite) | DB + vault 状态检查 |
| `mindclaw export [json]` | Storage export | 数据备份导出 |

---
