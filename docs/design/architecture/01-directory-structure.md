# MindClaw 技术架构设计

> 完整架构文档索引见 [README.md](./README.md)

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
      events.rs                 # InboundMessage, OutboundMessage, OutboundPayload 显式事件定义
    commands/                    # Tier 1: Web Commands（Tauri IPC，前端 invoke() 调用）
      mod.rs                    # 导出所有命令模块
      conversation.rs           # 对话：→ AgentLoop (入队) + SessionManager (查历史)
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
      daily.rs                  # mindclaw daily [date]
      search.rs                 # mindclaw search "query"
      task.rs                   # mindclaw task create/list/complete
      chat.rs                   # mindclaw chat "message"（需 API Key）
      status.rs                 # mindclaw status
      export.rs                 # mindclaw export [format]
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
      agent_loop.rs             # AgentLoop：事件驱动编排器，Bus → Session Queue → RunOnce → ProviderEvent → Tool Loop → OutboundEvent
      events.rs                 # ProviderEvent / AgentEvent / RunPhase 等运行时事件类型
      context.rs                # ContextPipeline：可插拔上下文管线（ContextSource trait + 优先级 + token 预算）
      hooks.rs                  # HookRegistry：事件钩子（PreMessage/PostMessage/PreToolUse/PostToolUse）
      session.rs                # SessionManager：按 sender 隔离会话、turn 追加、裁剪、持久化
      sub_agent.rs              # SubAgentRegistry：trait-based 任务注册表 + SubAgentExecutor
    memory/
      mod.rs                    # MemoryManager：统一记忆层入口（单表 memories，upsert by key）
      types.rs                  # Memory, MemoryCategory 结构定义
      recall.rs                 # 记忆召回：关键词 + 向量检索，importance 排序
    services/
      mod.rs                    # 导出所有业务 Service
      knowledge.rs              # KnowledgeService：知识笔记 CRUD、wikilink 提取、索引同步
      daily.rs                  # DailyService：日记读写、模板创建、条目追加
      task.rs                   # TaskService：任务 CRUD、状态管理
    providers/
      mod.rs                    # 模块导出 + re-export
      traits.rs                 # Provider trait、ChatMessage、ProviderResponse
      config.rs                 # ProviderConfig / ModelConfig 数据结构 + builtin_configs()
      registry.rs               # ProviderRegistry：配置注册 + 工厂方法（配置驱动，非代码驱动）
      openai_compat.rs          # OpenAICompatProvider：通用 OpenAI 兼容实现（OpenAI/DeepSeek/Moonshot 等）
      claude.rs                 # ClaudeProvider：Claude API 实现（独立协议，stub）
    tools/
      mod.rs                    # ToolRegistry + Tool trait（注册/查找/执行）
      traits.rs                 # Tool trait、ToolInput、ToolOutput
      # --- 基础能力工具（常驻上下文，3 个 Schema）---
      filesystem.rs             # vault 内文件操作（安全边界约束）
      shell.rs                  # 白名单受限 Shell（沙箱执行）
      mcp_client.rs             # MCP Client：接入外部工具服务
      operations.rs             # 元工具：list/call 动态发现并调用 Services + Memory
      skills.rs                 # SkillRegistry：技能系统（分发 ContextSources/Hooks/SubAgentTasks/Operations）
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
    KnowledgePage.tsx           # 知识库浏览与搜索
    ConversationPage.tsx        # 对话界面，模式选择
    SettingsPage.tsx            # 设置、API Key、角色模版
  components/
    layout/
      Sidebar.tsx               # 导航：Daily / Knowledge / Chat / Settings
      TopBar.tsx                # 全局状态栏 + 模式指示器
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
      ModelSelector.tsx         # 模型偏好设置
  hooks/
    useIpc.ts                   # 通用 invoke() 封装（泛型、错误处理、loading）
    useConversation.ts          # 对话状态、消息发送、模式切换
    useDaily.ts                 # 日记 CRUD
    useKnowledge.ts             # 知识搜索与浏览
    useTasks.ts                 # 任务 CRUD
    useSettings.ts              # 设置读写
  store/
    appStore.ts                 # 全局状态：当前页、用户信息、初始化状态
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
~/MindClaw/
  vault/                        # Markdown 内容（Obsidian 兼容）
  │ ├── daily/                  # YYYY-MM-DD.md
  │ ├── knowledge/              # Agent 沉淀的知识笔记（按主题分目录）
  │ │   ├── 投资/               # 主题目录
  │ │   │   ├── 价值投资.md     # 单篇知识笔记（含 frontmatter L0 + 正文 L2）
  │ │   │   └── 风险管理.md
  │ │   ├── 教育/
  │ │   │   └── 蒙特梭利.md
  │ │   └── 工作方法论/
  │ │       └── 深度工作.md
  │ ├── private/                # 私密区（Agent 不可见）
  │ └── _assets/                # 附件（图片、PDF）
  data/
  │ ├── main.db                 # SQLite 主库（L0/L1 索引 + FTS5）
  │ └── archive/                # 冷归档
  │     └── 2026-01.jsonl       # 按月归档对话
  config/
    └── settings.json           # 非敏感设置
```

整个 `~/MindClaw/` 目录 zip 打包即完整备份。

---
