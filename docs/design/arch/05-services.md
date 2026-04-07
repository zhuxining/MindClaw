> **Status**: `active`

# Services — 业务服务层

---

## § 职责定位

Services 层负责四大业务领域（任务管理、知识笔记、记忆、日记）的逻辑处理，不负责存储介质操作细节、Agent 执行控制或前端 UI 逻辑。

---

## § 核心原则

**无状态复用**：每个 Service 不持有跨请求的可变状态，以 `Arc<Service>` 形式被 Tauri 命令、CLI 和 Agent 工具三个入口共享，无需同步原语保护业务状态。

**Markdown 真相源**：任务、记忆、笔记的真相源是 Markdown 文件（YAML Frontmatter），SQLite 只存储索引。Service 层负责维护文件与索引的一致性。

---

## § 边界与实体

**输入**：来自 Tauri 命令层、CLI 命令或 Agent 工具调用的业务请求，携带领域对象（Task 创建参数、Note 内容、Memory 等）。

**输出**：业务操作结果，包含持久化后的完整对象或检索结果列表，不含存储层实现细节。

**核心实体**：

**TaskService**：任务生命周期的业务逻辑服务。

- 关键属性：Task 的创建规则（标题必填，状态默认为 Todo）、状态转换规则、优先级（Low/Medium/High）
- 关系：读写 `{vault}/tasks/*.md` 和 `tasks_index`；被 Tauri 命令和 Agent 任务工具共享调用

**NoteService**：知识笔记的检索与索引服务（替换原 KnowledgeService）。

- 关键属性：基于 `mtime` 的增量同步、扫描排除目录、标题/tags 提取
- 关系：扫描 vault 目录维护 `notes_index`；检索时返回轻量 `NoteIndex`，按需读取 Markdown 全文

**MemoryStore**：Agent 记忆的存储与召回服务。

- 关键属性：按 category 组织、importance 排序、关键词召回（LIKE 查询）
- 关系：读写 `{vault}/memory/*.md` 和 `memories_index`；被 Agent Loop 调用进行记忆提取和召回

**DailyService**：日记的读写服务。

- 关键属性：日期到文件路径的映射规则（`daily/YYYY-MM-DD.md`）
- 关系：直接读写 Markdown 文件，不依赖 SQLite 索引

---

## § 四个服务的调用方矩阵

| 服务 | Tauri 命令 | CLI 命令 | Agent 工具 |
|------|-----------|---------|-----------|
| TaskService | `commands/tasks.rs` | `cli/task.rs` | task 工具 |
| NoteService | `commands/knowledge.rs` | `cli/search.rs` | knowledge 工具 |
| MemoryStore | — | — | memory 工具（内部） |
| DailyService | `commands/daily.rs` | `cli/daily.rs` | daily 工具 |

四个入口共享同一 `Arc<ServiceContainer>` 中的 Service 实例，数据一致。

---

## § ServiceContainer

```rust
pub struct ServiceContainer {
    pub task: Arc<TaskService>,      // Task Markdown + tasks_index
    pub memory: Arc<MemoryStore>,    // Memory Markdown + memories_index
    pub note: Arc<NoteService>,      // Note Markdown + notes_index
    pub daily: Arc<DailyService>,    // Daily Markdown（无索引）
}
```

ServiceContainer 在 `AppRuntimeBuilder` 中初始化，接收 `vault_db`（Vault 级 SQLite 连接）。

---

## § 关键流程

### 任务创建

1. TaskService 生成 UUID 和 ISO8601 时间戳
2. 构造 `TaskFrontmatter`，生成文件路径 `tasks/{YYYY-MM-DD}-{slug}.md`
3. 原子写入 Markdown 文件（`markdown::write_file`）
4. UPSERT 到 `tasks_index` 表
5. 返回完整 `Task` 对象

### 任务更新

1. 查 `tasks_index` 获取 `file_path`
2. 读取 Markdown 文件，解析 `TaskFrontmatter`
3. 合并更新字段，更新 `updated` 时间戳
4. 原子写回 Markdown 文件
5. UPSERT 到 `tasks_index`

### 笔记索引同步（sync_index）

1. NoteService 遍历 vault 下所有 `.md` 文件（排除 `folder_mappings.index_exclude`）
2. 对比文件 `mtime` 与 `notes_index.modified_at`
3. 仅更新变化的文件：提取 title/tags，计算 path hash
4. UPSERT 到 `notes_index`
5. 返回 `SyncResult`（新增/更新/删除计数）

### 笔记检索（RAG 简化版）

1. NoteService 在 `notes_index` 中对 `title`/`tags` 进行 LIKE 查询
2. 返回 `NoteIndex` 列表（不含正文）
3. 调用方按需调用 `note.read(file_path)` 获取完整内容

### 记忆召回

1. MemoryStore 在 `memories_index` 中对 `key` 进行 LIKE 查询
2. 读取匹配的 Markdown 文件获取正文
3. 按 `importance` 降序排序
4. 返回 `Memory` 列表

---

## § 数据模型

### Task

**Frontmatter**：

```yaml
---
id: "uuid"
title: "任务标题"
status: todo          # todo | in_progress | done | cancelled
priority: medium      # low | medium | high
due_date: 2026-04-10
tags: [work, urgent]
created: 2026-04-07T14:30:00+08:00
updated: 2026-04-07T14:30:00+08:00
---
```

**Rust 结构**：

```rust
pub struct Task {
    pub id: String,
    pub title: String,
    pub status: TaskStatus,      // Todo/InProgress/Done/Cancelled
    pub priority: TaskPriority,  // Low/Medium/High
    pub due_date: Option<String>,
    pub tags: Vec<String>,
    pub created: String,         // ISO 8601
    pub updated: String,
    pub body: Option<String>,    // Frontmatter 后的正文
    pub file_path: PathBuf,      // vault 相对路径（不序列化）
}
```

### Memory

**Frontmatter**：

```yaml
---
id: "uuid"
key: "user-preference-theme"
category: preference  # user_fact/preference/work_context/relationship/goal
importance: 0.85
created: 2026-04-07T14:30:00+08:00
updated: 2026-04-07T14:30:00+08:00
---
```

**Rust 结构**：

```rust
pub struct Memory {
    pub id: String,
    pub key: String,
    pub category: MemoryCategory,
    pub content: String,      // 文件正文
    pub importance: f32,
    pub created: String,
    pub updated: String,
    pub file_path: PathBuf,   // vault 相对路径（不序列化）
}
```

### NoteIndex

轻量笔记索引（不存储正文）：

```rust
pub struct NoteIndex {
    pub id: String,           // file_path 的 SHA256 短 hash
    pub title: String,
    pub tags: Vec<String>,
    pub file_path: String,    // vault 相对路径
    pub modified_at: String,  // 文件 mtime
}
```

---

## § 设计决策与权衡

| 决策问题 | 选择 | 放弃的替代方案 | 理由 |
|---------|------|--------------|------|
| Services 是否持有可变状态？ | 无状态（不可变），以 Arc 共享 | 有状态服务（含内部缓存） | 无状态服务可被三个入口安全共享，无需 Mutex 保护 |
| 知识笔记的真相源是哪里？ | **Markdown 文件**（`{vault}/`） | SQLite 为真相源 | 文件对用户直接可见可编辑，Obsidian 可直接打开 |
| 是否保留向量嵌入？ | **不保留**（先不做） | SQLite BLOB 存储 embedding | 个人应用规模小，LIKE 查询足够；减少复杂度 |
| KnowledgeService 是否保留？ | **重命名为 NoteService**，简化设计 | 保留三级索引（L0/L1/L2） | 简单 LIKE 查询 + 按需读取全文足够 |
| 日记是否需要 SQLite 索引？ | 不需要 | 建 notes_index | 日记按日期直接寻址，无需索引 |
| Service 之间如何通信？ | 不直接通信，通过各自持有的 Storage 层间接协调 | Service 之间相互调用 | 避免 Service 层循环依赖 |
| 服务错误如何传播？ | 返回 `Result<T, AppError>`，调用方处理 | 服务内部捕获并静默处理 | 显式错误传播使调用方可以决定处理策略 |
| 如何处理并发写入冲突？ | SQLite 事务 + 乐观锁 | 悲观锁 | 个人应用冲突概率低；乐观锁无锁开销 |
| 索引如何重建？ | 启动时支持全量重建 | 只依赖增量同步 | 索引可重建是核心设计，确保数据安全 |

---

## § 相关文件

| 文件 | 说明 |
|------|------|
| `src/services/task.rs` | TaskService（Markdown + tasks_index） |
| `src/services/note.rs` | NoteService（Markdown + notes_index） |
| `src/agent/memory.rs` | MemoryStore（Markdown + memories_index） |
| `src/services/daily.rs` | DailyService（Markdown 直接读写） |
| `src/services/mod.rs` | ServiceContainer 定义 |
| `src/models/task.rs` | Task/TaskFrontmatter/TaskPriority 定义 |
| `src/storage/markdown.rs` | Frontmatter 解析与原子写入 |
