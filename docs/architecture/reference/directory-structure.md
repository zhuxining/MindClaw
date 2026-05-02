> **Status**: `active`
> **Last updated**: 2026-05-02
>
> 本文档描述 MindClaw 当前代码目录结构及目标差距。核心模块已完成 rig 迁移，前端工作域拆分待后续进行。

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

## Rust 后端当前结构（`src-tauri/src/`）

> 以下结构反映当前代码状态（2026-05-02）。
> ✅ 标记 = 已达成目标边界；🔧 标记 = 正在演进；📋 标记 = 待迁移。

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
│   ├── messages.rs         Runtime 消息与工具 Schema 契约
│   ├── context.rs          ContextPipeline / ContextSource / BuiltContext
│   ├── session.rs          AgentSession / TurnRecord / session 串行化
│   ├── events.rs           UserVisiblePhase 对外状态契约
│   ├── hooks.rs            RunHooks observer 与交互式 Hook / CompositeRunHooks
│   ├── subagent.rs            SubAgent 派生执行（inline / detached）
│   ├── retry.rs            RetryPolicy / RetryMode — LLM 调用重试机制
│   ├── compact.rs          AutoCompact — Session 历史自动压缩
│   ├── memory.rs           Agent 记忆召回与 Frontmatter 扩展
│   ├── skills.rs           SkillManifest / SkillMetadata / SkillsRegistry
│   ├── observability.rs    AgentObserver 与 tracing 适配
│   ├── build-in/           内置技能、派生 Agent 定义和运行时静态资源
│   │   ├── subagent/       Markdown 派生 Agent 定义
│   │   └── skills/         内置技能 Markdown
│   └── tools/              工具执行系统 ✅
│       ├── mod.rs          ✅ Rig ToolDyn 列表初始化与 profile 过滤
│       ├── path_guard.rs   ✅ Agent 工具路径沙箱
│       ├── shell.rs        ✅ Shell 工具
│       ├── file_ops.rs     ✅ 文件读取、写入和编辑
│       ├── find_files.rs   ✅ 文件搜索
│       ├── search_content.rs ✅ 内容搜索
│       ├── spawn.rs  ✅ 子代理工具（delegate_to_agent）
│       └── mcp.rs          ✅ MCP 外部工具桥接
│
├── providers/              LLM Provider Adapter ✅
│   ├── mod.rs              ✅ 导出 ProviderRegistry / AgentModelSet / LLMCompletionModel
│   ├── config.rs           ✅ ProviderConfig / ModelConfig / 内置配置
│   └── registry.rs         ✅ LLMClient / LLMCompletionModel / AgentModelSet / ProviderRegistry
│                            ✅ 已删除: traits.rs, claude.rs, openai_compat.rs, rig_adapter.rs
│
├── services/               业务服务层 🔧
│   ├── mod.rs              ✅
│   ├── note.rs             ✅ NoteService
│   ├── daily.rs            ✅ DailyService
│   ├── task.rs             🔧 TaskService（目标：checklist.rs）
│   └── [待添加]            📋 inbox.rs, checklist.rs, memory.rs, review.rs, evolution.rs, resource_import.rs
│
├── storage/                存储能力层 🔧
│   ├── mod.rs              ✅
│   ├── markdown.rs         ✅ Markdown + Frontmatter 读写
│   ├── keychain.rs         📋 OS Keychain（占位）
│   ├── archive.rs          📋 归档写入（占位）
│   ├── vector.rs           📋 向量索引（占位）
│   ├── database/           ✅ SQLite
│   │   ├── mod.rs
│   │   ├── global.rs       ✅ Global DB
│   │   └── vault.rs        ✅ Vault DB
│   └── migrations/         ✅ SQLite migration
│
├── models/                 跨层共享 DTO 与轻量数据模型
│   ├── mod.rs
│   ├── note.rs
│   ├── checklist.rs
│   ├── conversation.rs
│   └── settings.rs
│
├── commands/               Tauri IPC 命令薄层 🔧
│   ├── mod.rs              ✅
│   ├── conversation.rs     ✅ Agent Session 命令
│   ├── daily.rs            ✅ Daily 命令
│   ├── tasks.rs            🔧 Task 命令（目标：checklist）
│   ├── vault.rs            ✅ Vault 文件浏览/资源类型解析/笔记检索
│   ├── settings.rs         ✅ 设置、密钥和 WorkspacePrefs 命令
│   ├── system.rs           ✅ 系统状态命令
│   └── [待添加]            📋 inbox.rs, memory.rs, review.rs
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
- Agent Runtime 的消息、工具 Schema 和运行事件契约进入 `agent/`；Rig 类型只能停留在 `AgentRunner`、`providers/` 和 `agent/tools/` 执行支撑边界内。
- `agent/tools/` 保持目录结构；工具实现必须通过 Rig `ToolDyn` / `Tool` 接口注册。
- 内置派生 Agent 定义进入 `agent/build-in/subagent/*.md`，由 `agents.rs` 统一解析并注册；Main Agent 不进入 Markdown 自定义体系；内置技能和提示类静态资源进入 `agent/build-in/`；只提供上下文指导的内容不能建成可调用工具。

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

## 当前实现差距（2026-05-02）

### ✅ 已达成

- ✅ Providers 层已完成 rig 迁移，自定义 trait/adapters 全部删除
- ✅ AgentRunner 持有 `AgentModelSet`，provider 选择停在 ProviderRegistry
- ✅ Streaming 使用 rig 真实流式 API + 回调桥接
- ✅ AgentSpawnDispatcher 已重新启用
- ✅ 核心 agent/ 目录边界与文档一致
- ✅ Retry Policy 已实现（retry.rs）：transient error 自动重试
- ✅ Checkpoint Recovery 已实现（session.rs）：中断恢复状态
- ✅ AutoCompact 已实现（compact.rs）：Session 历史压缩
- ✅ CompositeRunHooks 已实现（hooks.rs）：Hook 组合器
- ✅ Mid-turn Injection 框架已添加（loop_.rs）：injection_queue 字段

### 🔧 进行中

- 🔧 `services/task.rs` 待迁移为 `services/checklist.rs`（Markdown checklist 语义）
- 🔧 `commands/tasks.rs` 待迁移为 `commands/checklist.rs`

### 📋 待启动

- 📋 `services/` 待添加：inbox.rs, checklist.rs, memory.rs, review.rs, evolution.rs, resource_import.rs
- 📋 `commands/` 待添加：inbox.rs, memory.rs, review.rs
- 📋 `storage/` 待完善：context_fs.rs, path_guard.rs, keychain.rs, vector.rs
- 📋 前端 `src/` 待拆分为 `app/`、`shell/`、`workspaces/`、`features/` 结构
- 📋 前端 `components/tasks/` 待迁移为 `workspaces/checklist/` + `features/checklist/`

---

## 转为 Active 的条件

本文档可以从 `draft` 标记为 `active` 的条件：

- 后端目录完成目标边界迁移，当前实现差距清单已清空或改为现状说明。
- 前端目录完成工作域分组迁移，旧 `tasks` 命名已收敛为 checklist。
- Vault 初始化、读写和索引逻辑与本文档目录语义一致。
- 链接到的架构文档仍存在并保持 `active`。
- 已完成对应代码验证：`bun run check-types`、`bun run check`、`cargo check`。
