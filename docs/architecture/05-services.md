> **Status**: `active`

# Services — 业务服务层

---

## § 职责定位

Services 层负责笔记、Daily、Checklist、Agent 记忆、回顾队列和演化记录的业务规则；不负责存储介质细节、Agent run 执行控制或前端 UI 逻辑。

---

## § 核心原则

**无状态复用**：每个 Service 不持有跨请求的可变业务状态，以 `Arc<Service>` 形式被 Tauri 命令、CLI 和 Agent 工具共享。

**真相源分离**：已确认知识正文由 Markdown 承载；Agent 记忆、候选、审核状态和演化记录由 SQLite 承载。

**候选由 Review 管理**：观察候选、记忆建议和经验教训候选的生命周期属于 ReviewService，不属于 MemoryService 或 NoteService。

---

## § 边界与实体

**输入**：来自 Tauri 命令层、CLI 命令、Agent 工具或 Agent Runtime 的业务请求。

**输出**：业务操作结果、检索结果、状态变更事件或审核队列项，不暴露底层存储介质。

**核心服务**：

**NoteService**：负责 Markdown Vault 中已确认知识笔记的读写、Frontmatter 维护和索引同步。

**DailyService**：负责 Daily Note 的读写和按日期寻址。

**ChecklistService**：负责从 Markdown checklist 中解析、更新和索引轻量任务项。

**MemoryService**：负责 Agent 记忆的确认、修正、删除、降权、召回和知识引用。

**ReviewService**：负责回顾队列、观察候选、记忆更新建议和经验教训候选。

**EvolutionService**：负责追加和查询演化记录，并保证关键记忆变化可审计。

---

## § 服务调用矩阵

| 服务 | Tauri 命令 | CLI 命令 | Agent Runtime / Tools |
|------|------------|----------|------------------------|
| NoteService | Vault / Knowledge 命令 | search / note 命令 | 知识检索、保存草稿 |
| DailyService | Daily 命令 | daily 命令 | Daily 读写工具 |
| ChecklistService | Daily / Vault checklist 命令 | checklist 命令 | checklist 更新工具 |
| MemoryService | Memory 工作域命令 | memory inspect 命令 | ContextPipeline 召回、确认后写入 |
| ReviewService | Review Queue 命令 | review 命令 | AgentLoop trigger、后台 review run |
| EvolutionService | Memory / Review 详情命令 | evolution inspect 命令 | 记忆变更后追加记录 |

---

## § ServiceContainer

```rust
pub struct ServiceContainer {
    pub note: Arc<NoteService>,
    pub daily: Arc<DailyService>,
    pub checklist: Arc<ChecklistService>,
    pub memory: Arc<MemoryService>,
    pub review: Arc<ReviewService>,
    pub evolution: Arc<EvolutionService>,
}
```

ServiceContainer 在 `AppRuntimeBuilder` 中初始化。服务共享 Storage Adapter，但不直接访问彼此的内部存储。

---

## § 关键流程

### 保存知识草稿

1. NoteService 接收标题、`tags`、`overview`、正文和来源链接。
2. Storage 原子写入 Markdown 文件。
3. NoteService 更新 `notes_index`。
4. 若来源是经验教训候选，ReviewService 标记候选已沉淀。
5. MemoryService 建立或更新 `shared` 记忆引用。
6. EvolutionService 追加知识沉淀记录。

### 确认记忆更新建议

1. ReviewService 读取 `MemoryUpdateProposal`。
2. 用户确认后，ReviewService 调用 MemoryService 执行新增、修正、删除或降权。
3. MemoryService 返回变化前后摘要。
4. EvolutionService 追加 `EvolutionLog`。
5. ReviewService 将审核项归档。

### 生成经验教训候选

1. ReviewService 汇总观察候选、案例、用户修正或执行失败。
2. 可选通过 AgentSpawnDispatcher 调度后台 review run。
3. ReviewService 保存 `LessonCandidate(Pending)`。
4. 用户确认后，候选开放 Save as Knowledge 入口。
5. 保存后正文进入 Markdown Vault，Memory 只保留引用。

---

## § 数据所有权

| 数据 | 所有者 Service | 真相源 | 说明 |
|------|----------------|--------|------|
| Markdown 知识正文 | NoteService | Markdown | 用户和 Agent 共享知识 |
| Daily 正文 | DailyService | Markdown | 按日期直接寻址 |
| Checklist 项 | ChecklistService | Markdown | SQLite 只保存可重建索引 |
| Agent 记忆 | MemoryService | SQLite | 结构化运行数据，不是知识正文 |
| 回顾队列和候选 | ReviewService | SQLite | 审核状态不可从 Markdown 重建 |
| 演化记录 | EvolutionService | SQLite | 审计记录，追加为主 |
| API Key | Provider/Settings | OS Keychain | 不进入 Services 明文状态 |

---

## § 服务间协作规则

- Service 之间通过公开方法协作，不直接访问对方存储表。
- ReviewService 可以请求 MemoryService 执行已确认记忆更新，但不能直接写 Memory 表。
- MemoryService 可以建立知识引用，但不能写 Markdown 知识正文。
- NoteService 可以保存知识草稿，但不能修改记忆状态。
- EvolutionService 只追加或查询演化记录，不反向修改业务对象。

---

## § 设计决策与权衡

| 决策问题 | 选择 | 放弃的替代方案 | 理由 |
|---------|------|--------------|------|
| Services 是否持有可变状态？ | 无状态，以 Arc 共享 | 有状态服务含内部缓存 | 三个入口共享更简单，业务状态落在 Storage |
| Memory 是否继续使用 Markdown 作为真相源？ | 否，使用 SQLite 结构化运行数据 | 独立 Markdown 记忆文件 | 记忆需要状态、来源、审核、降权和删除语义，不适合作为用户知识正文 |
| 候选生命周期归谁？ | ReviewService | MemoryService 或 NoteService | 候选横跨观察、记忆、经验和知识草稿，单独归属更清晰 |
| 演化记录是否由 MemoryService 内部维护？ | 否，EvolutionService 独立追加 | MemoryService 写自己的审计字段 | 演化记录会关联记忆、候选、Session 和知识，不只属于 Memory |
| Checklist 是否成为独立任务系统？ | 否，ChecklistService 维护 Markdown checklist 索引 | 独立任务 Service 管理任务实体 | 产品主线不把任务做成一等对象 |

---

## § 目标实现边界

| 边界 | 说明 |
|------|------|
| NoteService | Markdown 知识笔记和索引 |
| DailyService | Daily Note 读写 |
| ChecklistService | Markdown checklist 索引 |
| MemoryService | Agent 记忆与召回 |
| ReviewService | 回顾队列与候选 |
| EvolutionService | 演化记录 |
| ServiceContainer | Service 依赖注入入口 |
