> **Status**: `active`

# Services — 业务服务层

---

## § 职责定位

Services 层负责笔记、Daily、Inbox、Checklist、Agent 记忆、回顾队列和演化记录的业务规则；不负责存储介质细节、Agent run 执行控制或前端 UI 逻辑。

---

## § 核心原则

**无状态复用**：每个 Service 不持有跨请求的可变业务状态，以 `Arc<Service>` 形式被 Tauri 命令、CLI 和 Agent 工具共享。

**Markdown 真相源**：已确认知识、Checklist、Inbox 待处理产物、Agent 记忆和演化记录都由 Markdown + Frontmatter 承载；SQLite 只保存 ContextIndex、查询缓存和运行时状态。

**Inbox 承载待处理生命周期**：解析结果、知识草稿、观察候选、记忆建议和经验教训候选先进入 Inbox；ReviewService 负责审核语义，不拥有待处理文件本身。

---

## § 边界与实体

**输入**：来自 Tauri 命令层、CLI 命令、Agent 工具或 Agent Runtime 的业务请求。

**输出**：业务操作结果、检索结果、状态变更事件或审核队列项，不暴露底层存储介质。

**核心服务**：

**NoteService**：负责 Markdown Vault 中已确认知识笔记的读写、Frontmatter 维护、共有知识落位规则和 ContextIndex 同步。

**DailyService**：负责 Daily Note 的读写和按日期寻址。

**InboxService**：负责 Inbox 条目的创建、状态变更、归档、恢复和目标去向引用。

**ChecklistService**：负责从 Markdown checklist 中解析、更新和索引轻量任务项。

**MemoryService**：负责 Agent 记忆 Markdown 的确认、修正、删除、降权、召回和知识引用。

**ReviewService**：负责读取 Inbox 中的审核型条目，组织观察候选、记忆更新建议和经验教训候选的审核流程。

**EvolutionService**：负责追加和查询演化记录 Markdown，并保证关键记忆变化可审计。

**SourceImportService**：负责外部网页、PDF、文件和媒体资源的原始来源保存、解析执行和来源索引同步；解析后的 Markdown 交给 InboxService 写入 Inbox。

---

## § 服务调用矩阵

| 服务 | Tauri 命令 | CLI 命令 | Agent Runtime / Tools |
|------|------------|----------|------------------------|
| NoteService | Vault / Knowledge 命令 | search / note 命令 | 知识检索、保存草稿、解析知识保存位置 |
| DailyService | Daily 命令 | daily 命令 | Daily 读写工具 |
| InboxService | Inbox 命令 | inbox 命令 | 待处理条目创建、归档和恢复 |
| ChecklistService | Daily / Vault checklist 命令 | checklist 命令 | checklist 更新工具 |
| SourceImportService | Source 导入和预览命令 | source 命令 | 外部原始来源保存和解析调度 |
| MemoryService | Memory 工作域命令 | memory inspect 命令 | ContextPipeline 召回、确认后写入 |
| ReviewService | Review Queue 命令 | review 命令 | 消费 Inbox 审核条目、AgentLoop trigger、后台 review run |
| EvolutionService | Memory / Review 详情命令 | evolution inspect 命令 | 记忆变更后追加记录 |

---

## § ServiceContainer

```rust
pub struct ServiceContainer {
    pub note: Arc<NoteService>,
    pub daily: Arc<DailyService>,
    pub inbox: Arc<InboxService>,
    pub checklist: Arc<ChecklistService>,
    pub source_import: Arc<SourceImportService>,
    pub memory: Arc<MemoryService>,
    pub review: Arc<ReviewService>,
    pub evolution: Arc<EvolutionService>,
}
```

ServiceContainer 在 `AppRuntimeBuilder` 中初始化。服务共享 Storage Adapter，但不直接访问彼此的内部存储。

---

## § 关键流程

### 保存知识草稿

1. NoteService 接收标题、`tags`、`overview`、正文、来源链接和可选目标位置。
2. NoteService 按用户选择、主题目录、`tags` 或 Vault 配置规则解析共有知识保存位置，Storage 原子写入 Markdown 文件。
3. NoteService 更新 ContextIndex。
4. 若来源是 Inbox 候选，InboxService 写入目标知识引用并将源条目移出默认待处理队列；没有明确目标或用户选择关闭时，源条目进入归档。
5. MemoryService 建立或更新 `shared` 记忆引用。
6. EvolutionService 追加知识沉淀记录。

### 确认记忆更新建议

1. ReviewService 从 Inbox 读取 `MemoryUpdateProposal` 条目。
2. 用户确认后，ReviewService 调用 MemoryService 执行新增、修正、删除或降权。
3. MemoryService 写入或更新 Agent 记忆 Markdown，并返回变化前后摘要。
4. EvolutionService 追加 `EvolutionLog` Markdown。
5. InboxService 写入记忆与演化记录引用，并将原建议移出默认待处理队列；没有明确目标或用户选择关闭时归档。

### 生成经验教训候选

1. ReviewService 汇总观察候选、案例、用户修正或执行失败。
2. 可选通过 AgentSpawnDispatcher 调度后台 review run。
3. ReviewService 请求 InboxService 保存 `LessonCandidate(Pending)` Markdown 到 `inbox/review/`。
4. 用户确认后，候选开放 Save as Knowledge 入口。
5. 保存后正文按主题或用户规则进入 Markdown Vault，Memory 只保留引用，Inbox 源条目记录目标引用；无目标或用户选择关闭时归档。

### 导入外部资料

1. SourceImportService 保存原始资源、HTML 快照、URL 和 metadata 到 `sources/`。
2. SourceImportService 解析网页、PDF 或文件为 Markdown。
3. InboxService 将解析结果写入 `inbox/imports/`，标记 `source: external` 和 `inbox.type: parse_result`。
4. ContextIndex 记录 Inbox 条目的 L0/L1 字段，SourceIndex 记录原始来源与 Inbox 条目的映射。
5. 用户确认后，NoteService 按主题或用户规则另存为共有知识，InboxService 保留目标引用；无目标或用户选择关闭时归档源条目。

---

## § 数据所有权

| 数据 | 所有者 Service | 真相源 | 说明 |
|------|----------------|--------|------|
| Markdown 知识正文 | NoteService | Markdown | 用户和 Agent 共享知识 |
| Daily 正文 | DailyService | Markdown | 按日期直接寻址 |
| Inbox 待处理条目 | InboxService | Markdown | 捕获、解析结果、草稿和审核候选的统一待处理源 |
| 外部原始资料 | SourceImportService | 原始资源 + manifest | 只保存来源，不保存待审核解析 Markdown |
| Checklist 项 | ChecklistService | Markdown | SQLite 只保存可重建索引 |
| Agent 记忆 | MemoryService | Markdown | 可审阅行动上下文，ContextIndex 只保存召回索引 |
| 回顾队列语义 | ReviewService | Inbox Markdown 派生 | 审核状态写入 Inbox Frontmatter，ContextIndex 只保存队列索引 |
| 演化记录 | EvolutionService | Markdown | 审计记录以追加为主，ContextIndex 只保存时间线索引 |
| API Key | Provider/Settings | OS Keychain | 不进入 Services 明文状态 |

---

## § 服务间协作规则

- Service 之间通过公开方法协作，不直接访问对方存储表或文件细节。
- Service 使用 ContextStore / ContextURI 访问上下文，不直接拼接 Vault 路径或查询 SQLite 表。
- ReviewService 可以请求 MemoryService 执行已确认记忆更新，但不能直接改写 MemoryService 拥有的记忆语义。
- ReviewService 可以消费 Inbox 审核条目，但不能绕过 InboxService 直接移动、归档或删除待处理文件。
- MemoryService 可以建立知识引用，但不能写 Markdown 知识正文。
- NoteService 可以保存知识草稿，但不能修改记忆状态。
- NoteService 负责解析共有知识的保存位置；InboxService 只记录处理状态和去向，不判断知识应属于哪个主题。
- SourceImportService 只保存原始来源和生成解析结果，不能绕过 InboxService 直接落盘 Inbox、共有知识或 Agent 记忆。
- InboxService 负责待处理条目的状态和归档，不负责判断候选是否成立。
- EvolutionService 只追加或查询演化记录，不反向修改业务对象。

---

## § 设计决策与权衡

| 决策问题 | 选择 | 放弃的替代方案 | 理由 |
|---------|------|--------------|------|
| Services 是否持有可变状态？ | 无状态，以 Arc 共享 | 有状态服务含内部缓存 | 三个入口共享更简单，业务状态落在 Storage |
| Memory 是否使用 Markdown 作为真相源？ | 是，使用受管 Markdown + Frontmatter | SQLite 结构化运行数据作为真相源 | 记忆影响长期行为，必须可审阅、可迁移、可纠偏 |
| 待处理产物生命周期归谁？ | InboxService | ReviewService、MemoryService 或 NoteService 各自维护 | Inbox 是统一待处理源，文件状态、归档和去向引用应由单一服务维护 |
| 审核语义归谁？ | ReviewService | InboxService | InboxService 维护条目生命周期，ReviewService 判断观察、记忆建议和经验候选如何处理 |
| 共有知识保存位置归谁？ | NoteService | InboxService 或前端直接决定路径 | 主题目录、`tags` 和用户规则属于知识库落位语义，不应由待处理队列拥有 |
| 演化记录是否由 MemoryService 内部维护？ | 否，EvolutionService 独立追加 | MemoryService 写自己的审计字段 | 演化记录会关联记忆、候选、Session 和知识，不只属于 Memory |
| Checklist 是否成为独立任务系统？ | 否，ChecklistService 维护 Markdown checklist 索引 | 独立任务 Service 管理任务实体 | 产品主线不把任务做成一等对象 |
| 外部资料解析结果是否写入 sources？ | 否，解析结果进入 Inbox | 写入 `sources/` 或直接生成知识笔记 | `sources/` 只保存原始来源；解析产物需要先进入待处理审核 |

---

## § 目标实现边界

| 边界 | 说明 |
|------|------|
| NoteService | Markdown 知识笔记、保存位置规则和索引 |
| DailyService | Daily Note 读写 |
| InboxService | 待处理条目、状态、归档和去向引用 |
| ChecklistService | Markdown checklist 索引 |
| SourceImportService | 外部原始资料保存、解析调度和来源索引 |
| MemoryService | Agent 记忆与召回 |
| ReviewService | 回顾队列与候选 |
| EvolutionService | 演化记录 |
| ServiceContainer | Service 依赖注入入口 |
