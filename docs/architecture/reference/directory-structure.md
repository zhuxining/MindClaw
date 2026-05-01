> **Status**: `draft`
>
> 本文档描述 MindClaw 新版需求与架构设计下的目标目录结构。当前代码尚未完全迁移到目标结构，因此本文档保持 `draft`；完成代码目录迁移并通过验证后，才能标记为 `active`。

# 目标目录结构

本文档是目录结构参考，不记录设计理由。设计依据见：

- [Agent Runtime](../03-agent-runtime.md)
- [Services](../05-services.md)
- [Storage](../06-storage.md)
- [Runtime](../07-runtime.md)
- [Desktop Frontend](../08-desktop-frontend.md)

---

## 文档边界

- 本文档描述代码和 Vault 的目标组织方式。
- 本文档不新增 API、IPC、Rust trait 或前端类型。
- 本文档不要求本次同步移动代码。
- 目录完成迁移前，目标结构与当前实现存在差距是允许状态。
- 目录完成迁移后，代码重组必须同步更新本文档。

---

## Rust 后端目标结构（`src-tauri/src/`）

后端目录按架构职责组织：入口层保持薄，业务规则进入 Services，运行状态和依赖装配进入 Runtime，Agent 执行边界进入 Agent Runtime，存储细节进入 Storage。

```text
src-tauri/src/
├── lib.rs                  Tauri App 入口，注册插件、命令和运行时状态
├── main.rs                 桌面应用二进制入口
├── error.rs                统一错误类型 AppError
│
├── runtime/                AppRuntime 与依赖注入
│   ├── mod.rs              AppRuntime 核心结构
│   ├── builder.rs          AppRuntimeBuilder，按依赖顺序组装 Storage / Services / Providers / Agent Runtime
│   ├── services.rs         ServiceContainer
│   └── config/             配置加载与用户 / Vault 配置模型
│
├── agent/                  Agent Runtime
│   ├── mod.rs              模块导出
│   ├── agents.rs           Definition：AgentProfile / AgentRegistry / ModelRouter
│   ├── loop_.rs            Orchestration：AgentLoop，turn 级编排
│   ├── runner.rs           Execution：AgentRunner，run 级 LLM 与工具迭代
│   ├── spec.rs             Run 契约：AgentRunSpec / AgentRunResult / StopReason / TokenUsage
│   ├── context.rs          ContextPipeline / ContextSource / BuiltContext
│   ├── session.rs          AgentSession / TurnRecord / session 串行化
│   ├── events.rs           ProviderEvent / AgentEvent / LoopPhase / UserVisiblePhase
│   ├── hooks.rs            RunHooks 与交互式 Hook
│   ├── spawn.rs            SubAgent / BackgroundAgent 派生执行
│   ├── memory.rs           Agent 记忆召回与 Frontmatter 扩展
│   ├── skills.rs           SkillManifest / SkillMetadata / SkillsRegistry
│   ├── observability.rs    AgentObserver 与 tracing 适配
│   ├── built-in/           内置技能、系统提示或运行时静态资源
│   └── tools/              工具执行系统
│       ├── mod.rs          ToolRegistry / ToolExecutor
│       ├── traits.rs       Tool trait / ToolError
│       ├── path_guard.rs   Agent 工具路径沙箱
│       ├── shell.rs        Shell 工具
│       ├── file_ops.rs     文件读取、写入和编辑
│       ├── find_files.rs   文件搜索
│       ├── search_content.rs 内容搜索
│       ├── agent_spawn.rs  子代理工具入口
│       └── mcp.rs          MCP 外部工具桥接
│
├── providers/              LLM Provider Adapter
│   ├── mod.rs
│   ├── traits.rs           Provider trait / ChatMessage / ProviderEvent 转换边界
│   ├── config.rs           ProviderConfig / ModelConfig
│   ├── registry.rs         ProviderRegistry
│   ├── claude.rs           Anthropic Claude Adapter
│   └── openai_compat.rs    OpenAI-compatible Adapter
│
├── services/               业务服务层
│   ├── mod.rs
│   ├── note.rs             NoteService：共有知识笔记与 Frontmatter 维护
│   ├── daily.rs            DailyService：Daily Markdown
│   ├── inbox.rs            InboxService：待处理条目生命周期、归档和目标引用
│   ├── checklist.rs        ChecklistService：Markdown checklist 索引和更新
│   ├── memory.rs           MemoryService：确认后的 Agent 记忆与召回
│   ├── review.rs           ReviewService：观察、记忆建议和经验候选审核语义
│   ├── evolution.rs        EvolutionService：演化记录追加和查询
│   └── resource_import.rs  ResourceImportService：外部资源保存、解析和 Inbox 解析结果生成
│
├── storage/                存储能力层
│   ├── mod.rs              ContextStore / Storage facade
│   ├── context_fs.rs       ContextURI 与 Vault 文件能力
│   ├── markdown.rs         Markdown + Frontmatter 读写
│   ├── path_guard.rs       Vault / private / Agent 工具路径策略
│   ├── keychain.rs         OS Keychain
│   ├── archive.rs          归档写入能力
│   ├── vector.rs           可重建向量索引或 embedding 引用
│   ├── database/           SQLite 打开、连接池和 RuntimeStore / ContextIndex
│   │   ├── mod.rs
│   │   ├── global.rs       Global RuntimeStore
│   │   └── vault.rs        Vault ContextIndex / Vault RuntimeStore
│   └── migrations/         SQLite migration
│
├── models/                 跨层共享 DTO 与轻量数据模型
│   ├── mod.rs
│   ├── note.rs
│   ├── checklist.rs
│   ├── conversation.rs
│   └── settings.rs
│
├── commands/               Tauri IPC 命令薄层
│   ├── mod.rs
│   ├── conversation.rs     Agent Session 命令
│   ├── daily.rs            Daily 命令
│   ├── inbox.rs            Inbox 命令
│   ├── vault.rs            Vault 文件浏览、资源类型解析和笔记检索命令
│   ├── memory.rs           Agent Memory 命令
│   ├── review.rs           Review & Evolution 命令
│   ├── settings.rs         设置、密钥和 WorkspacePrefs 命令
│   └── system.rs           系统状态命令
│
├── cli/                    CLI 命令薄层
│   ├── mod.rs
│   ├── agent.rs
│   ├── daily.rs
│   ├── inbox.rs
│   ├── memory.rs
│   ├── review.rs
│   ├── search.rs
│   ├── status.rs
│   └── export.rs
│
├── bin/
│   └── cli.rs              CLI 二进制入口
│
├── channels/               Channel Adapter
│   ├── mod.rs
│   ├── traits.rs           Channel trait
│   ├── desktop.rs          Desktop IPC Channel
│   ├── telegram.rs         Telegram Bot Channel
│   └── feishu.rs           Feishu Bot Channel
│
├── bus/                    MessageBus
│   ├── mod.rs
│   └── events.rs           InboundMessage / OutboundMessage
│
├── gateway/                HTTP / WebSocket 入口薄层
│   ├── mod.rs
│   ├── server.rs
│   ├── api.rs
│   ├── ws.rs
│   └── auth.rs
│
├── cron/                   定时任务入口
│   ├── mod.rs
│   ├── scheduler.rs
│   └── jobs.rs
│
└── heartbeat/              健康监测入口
    └── mod.rs
```

### 后端维护规则

- `commands/`、`cli/`、`gateway/` 只做输入解析、权限入口、错误翻译和响应封装。
- 业务规则进入 `services/`，不能写入 `commands/` 或前端。
- 文件、SQLite、Keychain、PathGuard 和 ContextIndex 细节进入 `storage/`。
- Provider 协议差异进入 `providers/`，不能进入 `AgentLoop` 或 `AgentProfile`。
- `agent/tools/` 保持目录结构；工具实现必须通过统一 Tool 接口注册。
- 内置技能和提示类静态资源进入 `agent/built-in/`；只提供上下文指导的内容不能建成可调用工具。

---

## 前端目标结构（`src/`）

前端是薄客户端：负责渲染、收集输入、调用 `invoke()`、订阅事件和维护 UI 状态，不承载业务规则、持久化规则或直接 HTTP 调用。

前端文档和代码同时使用两套命名：UI 概念名使用 Title Case，例如 `Left Panel`、`Calendar Filter Pane`；代码目录使用 kebab-case 小写链接符，例如 `left-panel/`、`calendar-filter-pane/`。

```text
src/
├── App.tsx                 React 根组件，挂载 app/app-shell
├── main.tsx                Vite 入口
├── index.css               全局 design token、基础样式和编辑器样式
├── App.css                 兼容占位，默认不承载样式规则
│
├── app/                    应用级装配
│   ├── app-shell.tsx        根工作台壳层
│   ├── main-window.tsx      Ribbon / Panels / ContentHost / StatusBar 装配
│   └── providers.tsx       Query、事件订阅和应用级 Provider
│
├── shell/                  桌面工作台壳层
│   ├── ribbon/             36px Ribbon 与工作域入口
│   ├── panels/             LeftPanel / RightPanel / PaneHost / PaneFilterToolbar
│   │   ├── left-panel/
│   │   ├── right-panel/
│   │   ├── pane-host/
│   │   └── pane-filter-toolbar/
│   ├── panes/              Panel 内可组合的小 Pane
│   │   ├── calendar-filter-pane/
│   │   ├── tags-filter-pane/
│   │   ├── type-filter-pane/
│   │   ├── saved-filter-pane/
│   │   ├── file-explorer-pane/
│   │   ├── agent-list-pane/
│   │   ├── skill-list-pane/
│   │   ├── memory-list-pane/
│   │   ├── mcp-server-list-pane/
│   │   ├── session-list-pane/
│   │   ├── cron-job-list-pane/
│   │   ├── note-outline-pane/
│   │   ├── note-frontmatter-pane/
│   │   └── related-files-pane/
│   ├── content-host/       TabArea / ContentView / OpenTab 管理
│   ├── status-bar/         编辑、索引、Agent 运行状态
│   └── shell-primitives/   PanelHeader / Toolbar / IconButton / Splitter 等壳层基础件
│
├── workspaces/             Ribbon 工作域定义与默认视图
│   ├── file-workspace/     Daily / Inbox / Private / Vault 复用的文件工作域
│   ├── agent/              自定义 Agent 角色与 Agent 默认视图
│   ├── skills/             Skills 列表和详情
│   ├── memory/             Memory 列表和详情
│   ├── mcp/                MCP Server / Tool 列表和详情
│   ├── session/            Agent Session 列表和详情
│   ├── cron/               Cron Job 列表和详情
│   ├── checklist/          各 Markdown 文件中 checklist 的聚合视图
│   ├── graph/              知识与资源关系图
│   └── settings/           设置视图
│
├── features/               可被多个工作域复用的功能组件
│   ├── editor/             Milkdown 编辑器封装
│   ├── resource-preview/   Web / PDF / Image / 外部资源预览
│   ├── checklist/          Checklist 展示、定位和状态交互
│   └── agent-session/      Agent 消息、模式切换和流式状态
│
├── components/
│   └── ui/                 shadcn/ui 生成组件
│
├── hooks/                  事件订阅、工作区偏好同步和 UI 组合 hook
├── lib/                    IPC facade、事件类型、共享前端类型、日期和通用工具
├── queries/                TanStack Query invoke 查询封装
├── stores/                 Zustand UI 运行状态
├── tools/                  前端调试或开发期辅助代码
└── assets/                 静态资源
```

### 前端维护规则

- UI 概念名与 `docs/ui` 保持 Title Case；代码目录统一使用 kebab-case。
- `shell/` 只放稳定工作台壳层，不放业务工作域实现。
- `shell/panes/` 放可组合 Pane；Pane 只负责筛选、导航和上下文展示。
- 过滤类 Pane 使用 `{domain}-filter-pane/` 命名；文档上下文 Pane 使用 `note-outline-pane/`、`note-frontmatter-pane/`、`related-files-pane/` 这类对象化命名。
- `shell/shell-primitives/` 放 PanelHeader、Toolbar、IconButton、Splitter 等壳层基础件，不放业务 Pane。
- `workspaces/file-workspace/` 承载 Daily、Inbox、Private、Vault 的共享文件浏览模型。
- `workspaces/checklist/` 是各 Markdown 文件中 checklist 的聚合内容视图，不拥有独立任务真相源。
- `queries/` 只封装 invoke 查询和缓存键，不写业务判断。
- `stores/` 只保存 UI 状态和用户交互状态，不保存业务真相源。
- `lib/ipc.ts` 是前端调用后端的唯一 facade；前端不直接发起业务 HTTP 请求。
- `components/ui/` 只放 shadcn/ui 生成或项目级基础 UI 组件，不放业务组件。

---

## Vault 目标结构（`{vault}/`）

Vault 目录由 Storage 架构定义，代码目录迁移时需要保持同一语义边界。

```text
{vault}/
├── .obsidian/              Obsidian 配置，系统不接管
├── .mindclaw/
│   ├── config.json         VaultConfig：工作区偏好、目录映射、索引刷新偏好
│   ├── AGENTS.md           Vault 工作规范
│   ├── SOUL.md             Agent 稳定性格
│   ├── TOOLS.md            工具使用指引
│   ├── USER.md             用户摘要
│   ├── MEMORY.md           长期关键记忆摘要
│   ├── mindclaw.db         ContextIndex + Vault RuntimeStore
│   └── cache/              可删除缓存
│
├── daily/                  Daily Markdown
├── inbox/                  Intake & Review Queue
│   ├── captures/           用户手动捕获
│   ├── imports/            PDF / Web / File 解析结果
│   ├── review/             观察、记忆建议和经验候选
│   ├── drafts/             知识草稿和整理草稿
│   └── archive/            无明确去向、已拒绝或已关闭条目
│
├── resources/              外部原始资源
│   ├── web/                URL、HTML 快照和网页 metadata
│   ├── pdf/                PDF 原文和 metadata
│   ├── files/              文档、图片、音视频等附件原件
│   └── manifests/          资源清单、checksum 和导入批次记录
│
├── agent/                  Agent 可审阅长期资产
│   ├── memory/             已确认 Agent 记忆
│   └── evolution/          记忆变化、策略变化和证据链记录
│
├── private/                Agent 不可见内容
└── **/*.md                 共有知识、项目笔记和用户自定义 Markdown
```

### Vault 维护规则

- `inbox/` 是待处理 Markdown 产物的真实存储位置，不只是 UI 聚合视图。
- `resources/` 只保存外部原始资源、快照、附件、checksum 和 manifest；解析结果进入 `inbox/imports/`。
- `agent/` 只保存确认后的长期 Agent 资产；待审核建议进入 `inbox/review/`。
- `.mindclaw/cache/` 可以删除；删除后系统必须能从 Markdown、manifest 和原始资源重建缓存。
- `private/` 是 Vault 保留文件夹，不拥有独立 URI 命名空间；Agent、ContextIndex 和知识沉淀链路必须排除该目录。

---

## 当前实现差距

当前仓库已具备主要目录边界，但尚未完全达到目标结构：

- `src-tauri/src/services/` 仍缺少 Inbox、Checklist、Memory、Review、Evolution、ResourceImport 等目标服务文件。
- `src-tauri/src/storage/` 已有 `database/` 与 `migrations/`，但 ContextFS、ContextStore 和 PathGuard 目标边界尚未完全拆分为独立文件。
- `src-tauri/src/commands/` 仍缺少 Inbox、Memory、Review 等目标命令文件。
- 当前后端仍存在 `task.rs` / `tasks.rs` 命名；目标结构使用 checklist 表达 Markdown checklist，不把任务做成独立一等业务对象。
- 当前后端存在 `agent/build-in/`；目标结构使用 `agent/built-in/` 命名内置静态资源目录。
- `src/` 当前仍以 `components/chat/`、`components/layout/`、`components/tasks/` 为主，尚未拆出 `app/`、`shell/`、`workspaces/`、`features/`。
- 当前前端存在 `components/tasks/`；目标结构使用 `workspaces/checklist/` 表达独立内容视图，使用 `features/checklist/` 表达 Markdown checklist 复用能力。

这些差距不要求在本文档变更中解决。目录迁移实施时，应另起计划并同步更新代码、文档索引和验证命令。

---

## 转为 Active 的条件

本文档可以从 `draft` 标记为 `active` 的条件：

- 后端目录完成目标边界迁移，当前实现差距清单已清空或改为现状说明。
- 前端目录完成工作域分组迁移，旧 `tasks` 命名已收敛为 checklist。
- Vault 初始化、读写和索引逻辑与本文档目录语义一致。
- 链接到的架构文档仍存在并保持 `active`。
- 已完成对应代码验证：`bun run check-types`、`bun run check`、`cargo check`。
