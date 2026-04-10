> **Status**: `draft`

# 桌面端前端架构

→ 相关 PRD：[prd/00-overview.md](../prd/00-overview.md)  
→ 系统总览：[00-overview.md](00-overview.md)  
→ IPC 通道：[01-channels.md](01-channels.md)

## 职责定位

桌面端前端负责将 AppRuntime 的服务能力呈现为用户可操作的界面，通过 Tauri IPC（`invoke`）调用后端命令、通过 Tauri 事件系统接收后端推送，不包含任何业务逻辑计算。

**不负责**：任务创建的业务规则、Agent 执行逻辑、文件读写、存储操作——这些全部由 Rust 服务层处理，前端只负责状态展示和用户交互的收集与转发。

## 核心原则

1. **IPC 是唯一数据通道**：前端不直接读写文件系统，不持有业务数据的权威副本；所有数据来自 `invoke` 响应或后端 `emit` 事件。

2. **UI 状态与服务器状态分离**：Zustand 管理纯 UI 状态（当前 Tab、Chat 是否打开、流式消息缓冲），TanStack Query 管理服务器状态（任务列表、日记内容、设置项）并负责缓存和失效。

3. **流式消息不进查询缓存**：Agent 回复的流式 chunk 实时追加到 Zustand chatStore，不经过 TanStack Query；会话历史通过独立的 `get_session_history` 查询加载。

## 边界与实体

**输入**：

- `invoke(cmd, args)` — 用户操作触发的命令调用，返回 Promise
- `listen(event, handler)` — 订阅后端推送事件（Agent 流式输出）
- 用户的键盘/鼠标输入（由 React 事件系统捕获）

**输出**：

- `invoke` 调用参数（发送给 Rust commands 层）
- DOM 渲染结果（Tauri WebView 显示）

**核心实体**（前端视角）：

- **OpenedItem** — 当前中央区域显示的内容，可以是 `daily/` 中某日期的日记、Vault 中某路径的笔记、或 `source/` 中某个资源
- **StreamingMessage** — 正在流式输出的 Agent 消息，包含累积的 chunk 内容和当前阶段（thinking / using_tools / streaming）
- **WorkspaceLayout** — 工作区的当前布局状态：激活的 Tab、Pin 的笔记、Chat 是否打开

**错误边界**：前端捕获 `invoke` 返回的 AppError（序列化的 Rust 错误），翻译为用户可读的错误提示；流式事件中的 Error payload 由 chatStore 处理并显示在消息气泡中。

## 关键流程

### 流程 1：用户发送 Chat 消息

```
用户按 Enter
  → chatStore.addUserMessage(content, requestId)   // 乐观更新，立即渲染
  → invoke('send_message', { content, sessionId })  // 通知后端开始处理
  → chatStore.startStreaming(requestId)             // 添加 Agent 气泡占位
  
后端 emit 'mindclaw://agent-event'
  → Chunk   → chatStore.appendChunk(chunk)         // 文字增量追加
  → Status  → chatStore.setPhase(phase)            // 阶段指示更新
  → Done    → chatStore.completeStreaming()         // 流式结束，Markdown 渲染
  → Error   → chatStore.setError(message)          // 错误气泡
```

### 流程 2：用户切换 Tab

```
用户点击 Tab 按钮
  → workspaceStore.setActiveTab(tab)               // 立即更新 Tab 高亮
  → DirectoryPanel 根据 tab 选择对应子组件渲染    // 目录树切换
  → CenterContent 根据 openedItem 决定显示内容    // 若 openedItem 为空则显示默认视图
```

### 流程 3：任务状态更新（乐观更新）

```
用户点击任务复选框
  → 本地 state 立即标记为 done（乐观更新）
  → invoke('update_task_status', { id, status: 'done' })
  
成功：invalidateQueries(queryKeys.tasks.all)       // 重新获取任务列表
失败：回滚本地 state + 显示错误提示
```

## 组件层次

```
AppShell
├── LeftSidebar
│   ├── TabNav                    # Daily / Private / Vault / Source + 自定义固定 Tab
│   │   └── TabItem               # 右键菜单：固定为 Tab / 取消固定
│   └── DirectoryPanel            # 根据 activeTab 渲染目录内容
│       ├── ViewModeToggle        # Tree / Flat 切换按钮
│       ├── TreeView              # 树状模式：层级缩进，文件夹可折叠
│       └── FlatView              # 平铺模式：所有文件按修改时间倒序排列
│
├── CenterContent                 # 根据 openedItem 选择渲染
│   ├── NoteEditor                # Milkdown Crepe，所有 .md 文件统一使用，点击即编辑，防抖自动保存
│   ├── WebPreview                # Tauri WebView，用于 source/ 中的链接资源
│   ├── PdfViewer                 # PDF 渲染，用于 source/ 中的 PDF 资源
│   └── EmptyState                # 未打开内容时的占位
│
├── RightPanels                   # 三个垂直堆叠的可调整高度区块
│   ├── PinPanel                  # 单个固定笔记的标题 + 快速打开
│   ├── TasksPanel                # 任务分组列表 + 状态切换 + 创建入口
│   └── RelevancePanel            # 自动关联笔记列表
│
└── ChatOverlay                   # Portal 渲染，fixed 定位
    ├── ChatButton                # 右上角固定按钮
    └── ChatWindow                # 悬浮卡片
        ├── ModeSelector          # 5 种模式 ToggleGroup
        ├── MessageList           # 用户/Agent 消息列表
        │   └── MessageBubble     # 含 StreamingIndicator
        └── ChatInput             # Textarea + 发送按钮
```

## 状态设计

### WorkspaceStore（Zustand）

管理工作区的 UI 状态，与服务器数据无关。

```
Tab = BuiltinTab | PinnedDirTab

BuiltinTab = 'daily' | 'private' | 'vault' | 'source'

PinnedDirTab = {
  id: string          // 唯一标识
  dirPath: string     // Vault 内相对路径
  label: string       // 文件夹名称
}

DirectoryViewMode = 'tree' | 'flat'

WorkspaceStore
  activeTabId: string                           // BuiltinTab id 或 PinnedDirTab.id
  pinnedDirTabs: PinnedDirTab[]                 // 自定义固定目录 Tab 列表（持久化）
  dirViewMode: Record<string, DirectoryViewMode> // 按 tabId 记录视图模式（持久化）
  openedItem: DailyItem | NoteItem | SourceItem | null
  pinnedNote: { path, title } | null
  chatOpen: boolean
```

### ChatStore（Zustand）

管理 Chat 的会话状态，包含流式消息的实时缓冲。

```
ChatStore
  currentSessionId: string | null
  mode: 'companion' | 'reflection' | 'challenge' | 'knowledge' | 'vault'
  messages: (UserMessage | StreamingMessage)[]
  streamingRequestId: string | null    // 当前正在流式的 request_id
```

### TanStack Query（服务器状态缓存）

```
queryKeys.tasks.list(status?)          → invoke('list_tasks')
queryKeys.daily.byDate(date)           → invoke('get_daily')
queryKeys.knowledge.search(query)      → invoke('search_knowledge')
queryKeys.settings.all                 → invoke('get_settings')
```

## IPC 层

`src/lib/ipc.ts` 封装所有 `invoke` 调用，统一错误转换：

```
ipc.sendMessage(content, sessionId?) → call<string>('send_message', ...)
ipc.listTasks(status?)               → call<Task[]>('list_tasks', ...)
ipc.createTask(params)               → call<Task>('create_task', ...)
ipc.updateTaskStatus(id, status)     → call<void>('update_task_status', ...)
ipc.getDaily(date)                   → call<string>('get_daily', ...)
ipc.saveDaily(date, content)         → call<void>('save_daily', ...)
ipc.searchKnowledge(query)           → call<KnowledgeEntry[]>('search_knowledge', ...)
ipc.getSettings()                    → call<AppSettings>('get_settings', ...)
ipc.setApiKey(key)                   → call<void>('set_api_key', ...)
```

`src/lib/events.ts` 封装 Tauri 事件订阅：

```
listenAgentEvents(callback) → listen('mindclaw://agent-event', ...)
```

事件 payload 类型（对应 Rust `OutboundPayload`）：

- `{ type: 'Chunk', data: { content: string } }`
- `{ type: 'Status', data: { status: AgentPhase } }`
- `{ type: 'Done' }`
- `{ type: 'Error', data: { message: string, retryable: boolean } }`

## 设计决策与权衡

| 决策问题 | 选择 | 放弃的替代方案 | 理由 |
|---------|------|--------------|------|
| 工作区布局是否使用路由（URL）驱动？ | 用 Zustand 状态驱动布局，不用 URL 路由 | TanStack Router 路由树 | 工作区是单一视图应用，Tab 切换和笔记打开是面板内状态变化，不是页面导航；URL 路由会导致三栏布局在导航时重新挂载 |
| Chat 是否作为独立路由/页面？ | Chat 作为悬浮覆盖层（Portal + fixed 定位） | 独立路由页面 | "对话是万能入口"要求 Chat 随时可达且不打断当前工作区状态；独立路由会失去工作区上下文 |
| 流式消息是否进入 TanStack Query 缓存？ | 不进缓存，由 chatStore 管理 | 用 useQuery + streaming | 流式消息是实时增量数据，不是标准的请求-响应模型；强行适配 TanStack Query 会引入不必要的复杂度 |
| 任务状态更新是否使用乐观更新？ | 使用乐观更新 + 失败回滚 | 等待服务器确认后更新 | 任务状态切换是高频操作，等待 I/O 会造成明显延迟感；本地文件操作失败率低，回滚情况罕见 |
| 编辑器保存策略 | 防抖 1 秒自动保存 + Cmd+S 手动强制保存 | 每次输入立即保存 | 每次 keystroke 触发文件 I/O 会产生过高的写入频率；1 秒防抖在用户感知上接近实时，且大幅减少写入次数 |
| 自定义 Tab 数据存储位置 | 持久化到 `AppSettings`（后端配置文件）| 前端 localStorage | Tab 配置是用户设置的一部分，与 Vault 路径等设置共同管理；localStorage 在应用重装后丢失且对后端 CLI 入口不可见 |
| 目录视图模式存储 | WorkspaceStore 持久化，按 tabId 独立记录 | 全局单一模式 | 不同 Tab 的使用场景不同（Daily 适合平铺按时间浏览，Vault 适合树状导航），按 Tab 独立记忆符合实际使用习惯 |
