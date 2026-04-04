> **Status**: `active`

# Services — 业务服务层

---

## § 职责定位

Services 层负责三大业务领域（任务管理、知识笔记、日记）的逻辑处理，不负责存储介质操作细节、Agent 执行控制或前端 UI 逻辑。

---

## § 核心原则

**无状态复用**：每个 Service 不持有跨请求的可变状态，以 `Arc<Service>` 形式被 Tauri 命令、CLI 和 Agent 工具三个入口共享，无需同步原语保护业务状态。

---

## § 边界与实体

**输入**：来自 Tauri 命令层、CLI 命令或 Agent 工具调用的业务请求，携带领域对象（Task 创建参数、Note 内容、检索查询文本等）。

**输出**：业务操作结果，包含持久化后的完整对象或检索结果列表，不含存储层实现细节。

**核心实体**：

**TaskService**：任务生命周期的业务逻辑服务。
关键属性：Task 的创建规则（标题必填，状态默认为 Pending）、状态转换规则（Pending → InProgress → Done 单向流转）。
关系：读写 SQLite tasks 表；被 Tauri 命令（`commands/tasks.rs`）和 Agent 的任务工具共享调用。

**KnowledgeService**：知识笔记的检索与写入服务，维护 Markdown 文件与 SQLite 索引的一致性。
关键属性：三级索引结构（L0 Tags / L1 Overview / L2 Detail）、Markdown 文件作为 L2 真相源。
关系：写入时同时更新 Markdown 文件（`vault/knowledge/`）和 SQLite note_index；检索时以 SQLite 为入口、按需读取 Markdown 全文。

**DailyService**：日记的读写服务，按日期管理 `vault/daily/` 目录下的 Markdown 文件。
关键属性：日期到文件路径的映射规则（`vault/daily/YYYY-MM-DD.md`）。
关系：直接读写 Markdown 文件，不依赖 SQLite 索引；被 Tauri 命令和 Agent 工具调用。

---

## § 三个服务的调用方矩阵

| 服务 | Tauri 命令 | CLI 命令 | Agent 工具 |
|------|-----------|---------|-----------|
| TaskService | `commands/tasks.rs` | `cli/task.rs` | task 工具 |
| KnowledgeService | `commands/knowledge.rs` | `cli/search.rs` | knowledge 工具 |
| DailyService | `commands/daily.rs` | `cli/daily.rs` | daily 工具 |

三个入口共享同一 `Arc<ServiceContainer>` 中的 Service 实例，数据一致。

---

## § 关键流程

**任务创建**：

1. TaskService 接收创建请求，验证必填字段（标题）。
2. 分配 UUID，设置初始状态（Pending）和创建时间。
3. 写入 SQLite tasks 表，返回完整 Task 对象。

**知识笔记检索（RAG 流程）**：

1. KnowledgeService 接收查询文本，在 SQLite note_index 的 L0 tags 字段中进行关键词匹配，获取候选文件路径列表。
2. 对候选集的 L1 overview 字段进行相关性计算，按分数排序，取前 N 条。
3. 返回候选集的 L0 + L1 数据给调用方（Context Building 的 KnowledgeSource）；KnowledgeSource 按需读取 Markdown 全文（L2）注入上下文。

**知识笔记写入**：

1. KnowledgeService 接收笔记标题和内容。
2. 将 Markdown 内容写入 `vault/knowledge/{title}.md`（真相源更新）。
3. 从内容提取标签（关键词）和摘要（前 3 段），更新 SQLite note_index 的 L0 和 L1 字段。

---

## § 设计决策与权衡

| 决策问题 | 选择 | 放弃的替代方案 | 理由 |
|---------|------|--------------|------|
| Services 是否持有可变状态？ | 无状态（不可变），以 Arc 共享 | 有状态服务（含内部缓存） | 无状态服务可被三个入口安全共享，无需 Mutex 保护；缓存的一致性维护会增加复杂度 |
| 知识检索是否每次全量扫描 Markdown？ | 不是；以 SQLite note_index 为入口 | 每次全量扫描 vault/ 目录 | 全量文件扫描随笔记量线性增长；SQLite 三级索引在 I/O 和 token 两个维度均更高效 |
| 三个 Service 是否共享 SQLite 连接？ | 是；通过 Arc<Database> 共享连接池 | 每个 Service 独立连接 | SQLite WAL 模式支持并发读；共享连接减少文件句柄数，避免锁竞争 |
| Agent 工具如何调用 Services？ | 工具实现直接持有 Arc<Service> 引用，进程内调用 | 工具通过 HTTP 调用内部 API | 进程内直接调用无序列化开销，延迟最低；HTTP 方案引入额外网络层和序列化复杂度 |
| 日记是否需要 SQLite 索引？ | 不需要；日记按日期直接寻址 | 日记也建 note_index | 日记按 `YYYY-MM-DD.md` 格式命名，日期即索引；建 SQLite 索引收益低，增加写入复杂度 |
| Service 之间如何通信？ | 不直接通信，通过各自持有的 Storage 层间接协调 | Service 之间相互调用 | 避免 Service 层循环依赖；每个 Service 只依赖 Storage，依赖图简单清晰 |
| 服务错误如何传播？ | 返回 `Result<T, AppError>`，调用方处理 | 服务内部捕获并静默处理 | 显式错误传播使调用方（Tauri 命令/CLI/Agent 工具）可以决定错误处理策略；静默处理会掩盖问题 |
| 如何处理并发写入冲突？ | SQLite 事务 + 乐观锁 | 悲观锁 | 个人应用冲突概率低；乐观锁无锁开销，冲突时返回错误让调用方重试 |
| 服务层是否需要缓存？ | 不需要，直接查询 Storage | Service 层缓存结果 | 缓存增加一致性复杂度；SQLite WAL 模式下读性能足够，缓存收益不明显 |
