> **Status**: `draft`

# Desktop Frontend — 桌面工作台前端架构

→ UI 设计源：[desktop-ui.pen](../ui/desktop-ui.pen)
→ 相关 PRD：[01-workspace-shell.md](../prd/01-workspace-shell.md)
→ IPC 通道：[01-channels.md](01-channels.md)

## § 职责定位

桌面端前端负责把 AppRuntime 的服务能力组织成桌面工作台：Ribbon、Left Panel、Content Host、Right Panel 和 Status Bar。前端通过 Tauri IPC 调用后端命令，通过 Tauri 事件订阅接收运行时推送。

不负责：业务规则、Markdown checklist 解析、Agent 执行、文件读写、ContextIndex、Private 隔离和持久化真相源。这些能力属于 Rust Services、Agent Runtime 和 Storage。

## § 命名规则

本文档同时使用两套命名：

| 场景 | 规则 | 示例 |
|------|------|------|
| UI 设计概念 | Title Case，与 `docs/ui` 画板命名一致 | `Left Panel`、`File Explorer Pane`、`Content Host` |
| React 组件 / 类型 | PascalCase | `LeftPanel`、`CalendarFilterPane`、`NoteOutlinePane` |
| 代码目录 | kebab-case 小写链接符 | `left-panel/`、`calendar-filter-pane/`、`note-outline-pane/` |
| 状态字段 / 函数 | camelCase | `activeWorkspaceId`、`openTabs` |

文档描述交互和 UI 区域时使用 UI 设计概念名；描述文件路径时必须使用 kebab-case。

## § 核心原则

**工作台壳层稳定**：Ribbon、Left Panel、Content Host、Right Panel、Status Bar 是长期稳定的 UI 边界，工作域只替换这些边界内的 Pane 和内容视图。

**Pane 组合优先**：Left Panel 与 Right Panel 都由多个 Pane 小组件组成。Pane 负责局部筛选、导航或上下文展示，不直接拥有业务真相源。

**文件工作域复用同一浏览模型**：Daily、Inbox、Private、Vault 都默认使用 File Explorer Pane，并通过 Calendar Filter / Tags Filter / Type Filter / Saved Filters / File Explorer 这些 Pane Filter Toolbar 切换过滤入口。

**列表工作域复用同一查询模型**：Agent、Skills、Memory、MCP、Session、Cron 在 Left Panel 中都使用列表查询相关内容，点击列表项后在 Content Host 打开详情或编辑视图。

**独立工具视图不伪装成文件工作域**：Checklist、Graph、Settings 是独立 Content View。Checklist 是各个 Markdown 文件中 checklist 的聚合视图，不成为独立任务真相源。

## § 工作台区域

```text
main-window
├── Ribbon
├── LeftPanel
│   ├── PanelHeader
│   ├── PaneHost
│   └── PaneFilterToolbar
├── ContentHost
│   ├── TabArea
│   └── ContentView
├── RightPanel
│   ├── PanelHeader
│   ├── PaneHost
│   └── PaneFilterToolbar
└── StatusBar
```

**Ribbon**：固定工作域与全局动作入口。工作域入口包括 Daily、Inbox、Private、Vault、Checklist、Graph、Agent、Skills、Memory、MCP、Session、Cron、Settings。Open Today、New Note、New Session、Add Link 等是全局动作，不改变工作域定义。

**Left Panel**：当前工作域的导航与筛选区。它不直接等同于文件树，而是由多个 Pane 组成。

**Content Host**：中央多 Tab 内容区。所有文件、资源、Agent Session、列表详情和独立工具视图都以 Tab 形式承载。

**Right Panel**：当前 Content View 的 Inspector。Markdown 内容默认展示 Note Outline、Note Frontmatter、Related Content；Agent Session 展示引用上下文、执行状态和草稿；不支持上下文的视图显示空状态。

**Status Bar**：展示编辑保存状态、当前 Vault、索引状态、Agent 运行阶段和最近错误。

## § Pane 系统

Pane 是 Panel 内可组合的小组件。Pane 只负责展示、筛选和选择，不直接持久化业务数据。

| Pane | 所属区域 | 职责 |
|------|----------|------|
| `CalendarFilterPane` | Left Panel | 按日期过滤文件工作域内容 |
| `TagsFilterPane` | Left Panel | 按 Frontmatter tags 过滤文件工作域内容 |
| `TypeFilterPane` | Left Panel | 按资源或 Markdown 类型过滤内容 |
| `SavedFiltersPane` | Left Panel | 使用用户保存的过滤条件 |
| `FileExplorerPane` | Left Panel | 在当前 scope 下浏览文件和目录 |
| `AgentListPane` | Left Panel | 查询和选择自定义 Agent 角色 |
| `SkillListPane` | Left Panel | 查询和选择 Skill |
| `MemoryListPane` | Left Panel | 查询和选择 Agent Memory |
| `McpServerListPane` | Left Panel | 查询和选择 MCP Server / Tool |
| `SessionListPane` | Left Panel | 查询和选择 Agent Session |
| `CronJobListPane` | Left Panel | 查询和选择 Cron Job |
| `NoteOutlinePane` | Right Panel | 展示当前 Markdown 或文档结构大纲 |
| `NoteFrontmatterPane` | Right Panel | 展示和编辑当前 Markdown Frontmatter |
| `RelatedContentPane` | Right Panel | 展示关联笔记、来源和上下文对象 |

Pane Filter Toolbar 负责在同一 Panel 中切换或过滤 Pane。文件工作域默认提供 Calendar Filter、Tags Filter、Type Filter、Saved Filters、File Explorer 五个过滤入口；Right Panel 默认提供 Note Outline、Note Frontmatter、Related Content 三个上下文入口。

## § 工作域模型

```text
WorkspaceDefinition
├── id
├── ribbonItem
├── leftPanelLayout
├── defaultContentView
├── rightPanelLayout
└── openBehavior
```

### 文件工作域

Daily、Inbox、Private、Vault 是同一种 File Workspace 的不同 scope。

| 工作域 | 默认 Left Panel | scope | 默认 Content View |
|--------|-----------------|-------|-------------------|
| Daily | Calendar Filter / Tags Filter / Type Filter / Saved Filters / File Explorer | `daily/` | 今日 Daily Note |
| Inbox | Calendar Filter / Tags Filter / Type Filter / Saved Filters / File Explorer | `inbox/` | Inbox 列表或最近条目 |
| Private | Calendar Filter / Tags Filter / Type Filter / Saved Filters / File Explorer | `private/` | 最近私密笔记 |
| Vault | Calendar Filter / Tags Filter / Type Filter / Saved Filters / File Explorer | `/` | 最近笔记或 Vault 首页 |

Private 工作域使用同一文件浏览模型，但所有 Agent、Memory、Vault 写入类动作在 UI 层隐藏，并由后端 PathGuard 继续强制隔离。

### 列表工作域

Agent、Skills、Memory、MCP、Session、Cron 在 Left Panel 中使用列表查询模型。

| 工作域 | Left Panel | Content Host |
|--------|------------|--------------|
| Agent | 自定义 Agent 角色列表、搜索、状态过滤 | Agent 角色详情、编辑或默认 Agent Session |
| Skills | Skill 列表、搜索、来源过滤 | Skill 详情、启用状态和说明 |
| Memory | Memory 列表、分类、确认状态过滤 | Memory 详情、证据、来源和知识引用 |
| MCP | MCP Server / Tool 列表、连接状态过滤 | MCP Server 详情、工具清单和配置 |
| Session | Session 列表、时间和 Agent 过滤 | Agent Session 详情 |
| Cron | Cron Job 列表、状态过滤 | Cron Job 详情、运行记录和配置 |

这些工作域使用各自命名的列表 Pane，并共用列表查询、过滤、空状态和列表项选择协议；具体字段由各自 query adapter 提供。

### 独立内容视图

Checklist、Graph、Settings 是独立 Content View。

| 视图 | 打开方式 | 说明 |
|------|----------|------|
| Checklist | Ribbon 或命令打开 Tab | 各 Markdown 文件中 checklist 的聚合视图，不拥有独立任务正文 |
| Graph | Ribbon 或命令打开 Tab | Vault / Memory / Resource 关系图 |
| Settings | Ribbon 打开 Tab | 工作区、Vault、Provider、隐私和快捷键设置 |

独立内容视图可以使用 Left Panel 提供筛选或导航，但它们的主体交互在 Content Host 中完成。

## § 核心实体

**WorkspaceDefinition**：描述一个 Ribbon 工作域如何装配 Left Panel、默认 Content View 和 Right Panel。

**PaneDefinition**：描述 Pane 的组件、数据来源、过滤状态和适用工作域。

**ContentDescriptor**：描述 Content Host 可以打开的对象，包括文件、资源、Agent Session、实体详情和独立工具视图。

**OpenTab**：Content Host 中的一个 Tab，持有 `ContentDescriptor`、标题、关闭状态和保存状态。

**InspectorContext**：Right Panel 根据当前 active tab 派生的上下文，包括 note outline、frontmatter、related content、Agent 状态或空状态。

**WorkspacePrefs**：工作台持久化偏好，包括 active workspace、open tabs、active tab、面板宽度、Pane 展开状态、Pane Filter Toolbar 选择和最近打开内容。

## § 状态设计

```text
ShellStore
  activeWorkspaceId
  leftPanelCollapsed
  rightPanelCollapsed
  panelSizes
  statusBarState

PaneStore
  leftPaneStateByWorkspace
  rightPaneStateByContent
  activeLeftPaneFilter
  activeRightPaneFilter

TabStore
  openTabs
  activeTabId
  dirtyTabIds

AgentSessionStore
  currentSessionId
  mode
  messages
  streamingRequestId
```

TanStack Query 只缓存后端查询结果；流式 Agent 事件继续由 `AgentSessionStore` 实时处理，不进入 query cache。

## § IPC 与事件边界

前端所有业务数据通过 `src/lib/ipc.ts` 调用 Rust commands。Pane 和 Content View 不直接调用 `invoke()`，而是通过 query hooks 或 feature adapter 调用统一 IPC facade。

`src/lib/events.ts` 订阅 Tauri 事件，并把 runtime event 转换为 AgentSessionStore 或 StatusBar 可消费的 UI 状态。

## § 目标代码结构

```text
src/
├── app/
│   ├── app-shell.tsx
│   ├── main-window.tsx
│   └── providers.tsx
├── shell/
│   ├── ribbon/
│   ├── panels/
│   │   ├── left-panel/
│   │   ├── right-panel/
│   │   ├── pane-host/
│   │   └── pane-filter-toolbar/
│   ├── panes/
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
│   ├── content-host/
│   ├── status-bar/
│   └── shell-primitives/
├── workspaces/
│   ├── file-workspace/
│   ├── agent/
│   ├── skills/
│   ├── memory/
│   ├── mcp/
│   ├── session/
│   ├── cron/
│   ├── checklist/
│   ├── graph/
│   └── settings/
├── features/
│   ├── editor/
│   ├── resource-preview/
│   ├── checklist/
│   └── agent-session/
├── components/ui/
├── hooks/
├── lib/
├── queries/
└── stores/
```

## § 设计决策与权衡

| 决策问题 | 选择 | 放弃的替代方案 | 理由 |
|---------|------|--------------|------|
| Left Panel 是否等同文件树？ | 否，Left Panel 是 PaneHost | 固定 DirectoryPanel | UI 设计中 Left Panel 包含 Calendar Filter、Tags Filter、Type Filter、Saved Filters、File Explorer、Session 等多个 Pane |
| Daily / Inbox / Private / Vault 是否各自实现导航？ | 否，共用 File Workspace + scope | 每个工作域各自实现文件树 | 四者都是文件与 Markdown 空间的不同入口，复用 Pane 能保持过滤体验一致 |
| Agent / Skills / Memory / MCP / Session / Cron 是否共用列表模式？ | 是，共用列表 Pane 协议，各自使用精准命名的 Pane | 每个工作域自定义列表结构 | 它们都是实体查询、筛选、选中详情的交互，变更理由一致 |
| Checklist 是否是一等业务对象？ | 否，Checklist 是独立 Content View | 独立任务工作域和真相源 | 产品原则要求任务以 Markdown checklist 表达，Checklist 只聚合和定位 |
| 视觉规范是否写入架构文档？ | 否，架构文档引用 `docs/ui` | 在架构文档复制视觉规则 | UI 细节以 `docs/ui` 为准，架构文档只记录边界和数据流 |
