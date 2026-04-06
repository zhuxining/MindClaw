> **Status**: `active`

# Storage — 存储层

---

## § 职责定位

Storage 层负责三类存储介质（SQLite、Markdown 文件、OS Keychain）的读写操作，不负责任何业务逻辑判断、数据聚合或索引重建决策。

---

## § 核心原则

**真相源不可混淆**：知识笔记的真相源是 Markdown 文件，SQLite 存储的是索引（允许过时，可重建）；混淆两者的写入权会导致数据不一致无法恢复。

---

## § 边界与实体

**输入**：来自 Services 层的读写请求，携带领域对象（Task、Note 内容、Memory 向量等）。

**输出**：存储操作结果（成功/失败）或检索结果（记录列表、文件内容），对上层屏蔽存储介质的差异。

**核心实体**：

**Database**：SQLite 数据库访问器，管理所有需要快速结构化检索的数据。
关键属性：数据库文件路径（`data/mindclaw.db`）、WAL 模式（支持并发读）。
关系：由 AppRuntimeBuilder 初始化（执行迁移），以 `Arc<Database>` 形式被 Services 层共享引用。

**VaultStorage**：`vault/` 目录的文件访问层，提供 Markdown 文件的读写接口。
关键属性：vault 根路径（`vault_path`，从 AppConfig 读取）。
关系：KnowledgeService 写入知识笔记后，同步更新 SQLite note_index；DailyService 直接读写日记文件，不依赖 SQLite 索引。

**KeychainStorage**：OS Keychain（macOS Keychain / Windows Credential Manager）的访问封装。
关键属性：服务名称（Keychain 中的分组标识）。
关系：AppRuntimeBuilder 在初始化 ProviderRegistry 时调用，读取 API Key 注入 Provider 构造函数。

---

## § 存储职责分配

| 数据类型 | 存储位置 | 写入方 | 同步机制 |
|---------|---------|-------|---------|
| 会话消息历史（Turn） | SQLite conversations 表 | SessionManager | 单一写入方，无需同步 |
| Agent 记忆（Memory）+ 向量嵌入 | SQLite memories 表 | MemoryStore | 单一写入方，无需同步 |
| 任务（Task） | SQLite tasks 表 | TaskService | 单一写入方，无需同步 |
| 笔记索引（L0 tags / L1 overview） | SQLite note_index 表 | KnowledgeService | 写入 Markdown 时同步更新 |
| 笔记原文（L2 内容） | Markdown `vault/knowledge/` | KnowledgeService + 用户直接编辑 | 用户编辑后需触发 re-index |
| 日记 | Markdown `vault/daily/` | DailyService + 用户直接编辑 | 日记不建 SQLite 索引 |
| 私密笔记 | Markdown `vault/private/` | 仅用户直接编辑 | Agent 不可访问（PathGuard 拒绝） |
| API Key、Bearer Token | OS Keychain | 用户通过设置界面写入 | 独立存储，不与 SQLite 同步 |
| 历史归档（90天后） | JSONL `data/archive/` | 归档定时任务 | 从 SQLite conversations 迁移 |

---

## § 三级笔记索引设计

| 级别 | 内容 | 存储位置 | Token 成本 | 用途 |
|------|------|---------|-----------|------|
| L0 Tags | 关键词标签列表 | SQLite note_index | 极低（~100 tokens） | 快速筛选候选集 |
| L1 Overview | 笔记摘要（2-3段） | SQLite note_index | 低（~2K tokens） | 相关性排序和内容导航 |
| L2 Detail | 完整 Markdown 内容 | Markdown 文件 | 无限制 | 实际内容消费 |

检索路径：L0 筛选候选 → L1 排序 → L2 按需读取全文（不是每次检索都读全文）。

---

## § 关键流程

**笔记写入**：

1. KnowledgeService 接收内容，写入 `vault/knowledge/{title}.md`（真相源更新）。
2. KnowledgeService 从内容提取标签和摘要，更新 SQLite note_index 的 L0 和 L1 字段。

**笔记检索（RAG）**：

1. KnowledgeService 在 SQLite note_index 的 L0 tags 字段中匹配查询关键词，获得候选文件路径列表。
2. 对候选集的 L1 overview 进行相关性计算，按分数排序。
3. 返回 Top-N 结果的 L1 内容给调用方；调用方按需读取 Markdown 文件获取 L2 全文。

---

## § 设计决策与权衡

| 决策问题 | 选择 | 放弃的替代方案 | 理由 |
|---------|------|--------------|------|
| 知识笔记的真相源是哪里？ | Markdown 文件（`vault/`） | SQLite 为真相源，Markdown 为导出格式 | 文件对用户直接可见可编辑，SQLite 损坏后可从 Markdown 重建索引；反之无法从 SQLite 恢复格式化的 Markdown |
| 向量嵌入存储在哪里？ | SQLite BLOB 列（内嵌向量） | 独立向量数据库服务（如 Qdrant） | 本地桌面应用不适合依赖独立数据库服务；个人规模的笔记量不需要专业向量数据库的性能 |
| 会话历史是否永久保留在 SQLite？ | 90 天后归档到 JSONL 冷存储 | 永久保留在 SQLite | SQLite 文件随会话历史增长影响查询性能；归档保留完整历史，主库保持快速 |
| 私密内容如何与 Agent 隔离？ | PathGuard 在 Rust 层拒绝 `vault/private/` 路径 | 文件系统权限（chmod/ACL） | Rust 层强制比文件系统权限更可靠、更细粒度；用户不需要手动维护文件权限 |
| SQLite 并发访问如何处理？ | WAL 模式（Writer-Ahead Logging） + Arc 共享连接池 | 每个 Service 独立 SQLite 连接 | WAL 模式支持并发读；共享连接池减少文件句柄数量；独立连接需要应用层协调并发写 |
| 向量嵌入如何存储？ | SQLite BLOB 列（与结构化数据同库） | 独立向量数据库 | 个人应用数据量小，SQLite BLOB 足够；独立数据库增加部署复杂度 |
| Markdown 文件命名冲突如何处理？ | 标题 slugify + 序号后缀 | 覆盖或拒绝 | 保留用户意图（标题），同时避免覆盖；序号后缀明确标识不同版本 |
| 如何备份用户数据？ | 导出 vault/ 目录 + SQLite 文件 | 云同步 | 本地优先设计；导出功能让用户完全控制数据，不依赖第三方云服务 |
| 如何处理存储空间不足？ | 返回错误，调用方处理 | 自动清理旧数据 | 自动清理可能导致数据丢失；返回错误让用户决定清理策略 |
| 文件写入如何保证原子性？ | temp + rename 模式 | 写前复制（Copy-on-Write） | rename 原子性由 OS 保证，跨平台兼容；COW 依赖文件系统特性 |
| vault 与 SQLite 如何保持一致？ | 文件先写，SQLite 事务提交；启动时重建索引 | 单事务覆盖两者 | SQLite 不支持文件系统事务；文件先写确保数据不丢，索引可重建 |
| 崩溃后如何恢复？ | 启动时扫描 vault，重建 SQLite 索引 | 依赖 SQLite 日志回滚 | 索引可重建使恢复简单可靠；SQLite 日志只能保证自身一致性，不能恢复文件 |
