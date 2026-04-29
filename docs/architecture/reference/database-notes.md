> **Status**: `active`
>
> 本文档描述目标 SQLite 索引、运行时表和并发模型。SQLite 只保存索引、缓存和运行时状态；内容真相源见 [../06-storage.md](../06-storage.md)。

# 数据库说明

MindClaw 使用双 SQLite 数据库架构：

- **全局 DB**（`~/.config/mindclaw/mindclaw.db`）：保存跨 vault 的活跃会话和 turn 运行记录。
- **Vault DB**（`{vault}/.mindclaw/mindclaw.db`）：保存当前 vault 的 ContextIndex、PrivateIndex、ChecklistIndex、Inbox 队列视图、来源映射、查询缓存、后台队列和运行时锁。

---

## 并发模型

- **WAL 模式**：`PRAGMA journal_mode = WAL`，支持读写并发。
- **忙等待超时**：`PRAGMA busy_timeout = 5000`，5 秒超时后返回错误。
- **外键约束**：`PRAGMA foreign_keys = ON`。
- **连接模型**：每个 DB 使用 `Arc<Mutex<Connection>>`，Tokio 异步运行时通过 Mutex 序列化访问。
- **重建策略**：业务索引表必须可从 Vault Markdown、Inbox Markdown 和原始资源重建；运行时表不可重建。

---

## 全局 DB 表清单

| 表名 | 写入方 | 说明 | 可重建 |
|------|--------|------|--------|
| `sessions` | SessionManager | 活跃对话会话元数据 | 否 |
| `turns` | SessionManager | 活跃会话 turn 运行记录 | 否 |

### sessions

| 列名 | 类型 | 约束 | 说明 |
|------|------|------|------|
| `id` | TEXT | PRIMARY KEY | 会话唯一标识 |
| `sender` | TEXT | NOT NULL | 发送者标识 |
| `mode` | TEXT | NOT NULL | 会话模式 |
| `summary` | TEXT | | 会话摘要 |
| `created` | TEXT | NOT NULL | 创建时间 |
| `updated` | TEXT | NOT NULL | 最后更新时间 |

**索引**：

- `idx_sessions_sender`
- `idx_sessions_updated`

### turns

| 列名 | 类型 | 约束 | 说明 |
|------|------|------|------|
| `id` | INTEGER | PRIMARY KEY AUTOINCREMENT | 回合自增 ID |
| `session_id` | TEXT | NOT NULL, FK → sessions(id) | 所属会话 |
| `user_message` | TEXT | NOT NULL | 用户消息 JSON |
| `assistant_message` | TEXT | | Agent 响应 JSON |
| `tool_trace` | TEXT | NOT NULL DEFAULT '[]' | 工具执行轨迹 JSON |
| `run_status` | TEXT | NOT NULL DEFAULT 'success' | 执行状态 |
| `created` | TEXT | NOT NULL | 创建时间 |

**索引**：

- `idx_turns_session`

活跃 turn 是运行时恢复数据；进入长期审计或回顾链路时，由 Session / Review 相关服务生成 `agent/sessions/*.md` 会话归档摘要，归档摘要进入 ContextIndex。

---

## Vault DB 表清单

| 表名 | 写入方 | 说明 | 可重建 |
|------|--------|------|--------|
| `context_index` | ContextStore | Vault、source、inbox、agent 空间的文档级统一索引 | 是 |
| `context_fts` | ContextStore | 文档级全文搜索索引 | 是 |
| `source_index` | SourceImportService | 原始来源与 Inbox 解析条目映射 | 是 |
| `checklist_index` | ChecklistService | Markdown checklist 行级索引 | 是 |
| `review_queue_index` | ReviewService | 基于 Inbox 审核条目的回顾队列排序和优先级缓存 | 是 |
| `evolution_timeline_index` | EvolutionService | 演化记录时间线缓存 | 是 |
| `private_index` | PrivateService | Private 工作域内部搜索索引 | 是 |
| `semantic_cache` | ContextStore | 摘要、embedding 引用和 rerank 缓存 | 是 |
| `runtime_locks` | Runtime / Services | 文件写入锁、任务锁 | 否 |
| `background_jobs` | Runtime / Services | 后台解析、索引、回顾任务游标 | 否 |

---

## context_index

`context_index` 是文档级统一索引。它不保存正文真相，只保存从 Markdown Frontmatter、文件路径和受管资产状态派生出的检索字段。

| 列名 | 类型 | 约束 | 说明 |
|------|------|------|------|
| `uri` | TEXT | PRIMARY KEY | ContextURI |
| `space` | TEXT | NOT NULL CHECK | vault / source / inbox / agent |
| `path` | TEXT | NOT NULL UNIQUE | Vault 相对路径 |
| `title` | TEXT | NOT NULL | 标题 |
| `tags` | TEXT | NOT NULL DEFAULT '[]' | JSON 数组 |
| `overview` | TEXT | NOT NULL DEFAULT '' | L1 概览 |
| `source` | TEXT | NOT NULL CHECK | user / external / agent / derived / system |
| `asset_kind` | TEXT | | parse_result / memory_proposal / memory / evolution_log 等 |
| `status` | TEXT | | draft / pending / processing / reviewed / confirmed / rejected / archived / deleted 等 |
| `owner` | TEXT | | user / agent / shared |
| `updated_at` | TEXT | NOT NULL | 更新时间 |
| `frontmatter_hash` | TEXT | NOT NULL | Frontmatter 哈希 |
| `content_hash` | TEXT | | 正文或资源哈希 |

**索引**：

- `idx_context_space`
- `idx_context_source`
- `idx_context_asset_kind`
- `idx_context_status`
- `idx_context_owner`
- `idx_context_updated`

### context_fts

`context_fts` 使用 SQLite FTS 保存标题、标签、概览和可选正文片段，用于本地搜索。它可以从 Markdown 和 ContextIndex 重建。

| 列名 | 说明 |
|------|------|
| `uri` | ContextURI |
| `title` | 标题 |
| `tags_text` | tags 展开文本 |
| `overview` | 概览 |
| `body_excerpt` | 可重建正文片段 |

---

## source_index

`source_index` 保存外部资料的原始资源、来源 manifest 和 Inbox 解析条目的映射关系。解析后的 Markdown 不写入 `sources/`。

| 列名 | 类型 | 约束 | 说明 |
|------|------|------|------|
| `uri` | TEXT | PRIMARY KEY | 来源 ContextURI |
| `source_kind` | TEXT | NOT NULL | web / pdf / file / image / audio / video |
| `original_uri` | TEXT | | 原始 URL 或导入前路径 |
| `raw_path` | TEXT | | Vault 内原始资源路径 |
| `manifest_path` | TEXT | | 来源 manifest 路径 |
| `latest_inbox_uri` | TEXT | | 最新解析结果或导入摘要的 Inbox ContextURI |
| `checksum` | TEXT | | 原始资源校验值 |
| `parser` | TEXT | | 解析器名称 |
| `captured_at` | TEXT | NOT NULL | 捕获时间 |

---

## checklist_index

Checklist 仍然是 Markdown checklist 的行级索引，不成为独立任务真相源。

| 列名 | 类型 | 约束 | 说明 |
|------|------|------|------|
| `id` | TEXT | PRIMARY KEY | checklist 项稳定 ID |
| `content` | TEXT | NOT NULL | checklist 内容 |
| `status` | TEXT | NOT NULL CHECK | todo / done |
| `priority` | TEXT | | low / medium / high |
| `due_date` | TEXT | | YYYY-MM-DD |
| `tags` | TEXT | NOT NULL DEFAULT '[]' | JSON 数组 |
| `file_path` | TEXT | NOT NULL | Vault 相对路径 |
| `line_number` | INTEGER | | 行号 |
| `created` | TEXT | NOT NULL | 创建时间 |
| `updated` | TEXT | NOT NULL | 更新时间 |

**索引**：

- `idx_checklist_status`
- `idx_checklist_due_date`
- `idx_checklist_updated`

---

## review_queue_index

`review_queue_index` 是回顾工作域的队列视图。审核状态真相源仍在对应 Inbox Markdown Frontmatter。

| 列名 | 类型 | 约束 | 说明 |
|------|------|------|------|
| `uri` | TEXT | PRIMARY KEY | ReviewItem 对应 Inbox ContextURI |
| `asset_kind` | TEXT | NOT NULL | observation / memory_proposal / lesson_candidate |
| `status` | TEXT | NOT NULL | pending / processing / reviewed / rejected / archived |
| `priority` | INTEGER | NOT NULL DEFAULT 0 | 队列排序权重 |
| `updated_at` | TEXT | NOT NULL | 更新时间 |

**索引**：

- `idx_review_queue_kind`
- `idx_review_queue_status`
- `idx_review_queue_priority`

---

## evolution_timeline_index

`evolution_timeline_index` 是演化记录时间线缓存。记录正文和证据链在 `agent/evolution/*.md`。

| 列名 | 类型 | 约束 | 说明 |
|------|------|------|------|
| `uri` | TEXT | PRIMARY KEY | EvolutionLog ContextURI |
| `status` | TEXT | NOT NULL | active / archived |
| `changed_asset_uri` | TEXT | | 被影响的记忆、Inbox 候选或知识 URI |
| `updated_at` | TEXT | NOT NULL | 更新时间 |

---

## private_index

`private_index` 只服务 Private 工作域搜索。Agent Runtime、ContextPipeline、MemoryService、ReviewService 和 EvolutionService 不得访问该表。

| 列名 | 类型 | 约束 | 说明 |
|------|------|------|------|
| `uri` | TEXT | PRIMARY KEY | Private ContextURI |
| `path` | TEXT | NOT NULL UNIQUE | Private 相对路径 |
| `title` | TEXT | NOT NULL | 标题 |
| `tags` | TEXT | NOT NULL DEFAULT '[]' | JSON 数组 |
| `overview` | TEXT | NOT NULL DEFAULT '' | 概览 |
| `updated_at` | TEXT | NOT NULL | 更新时间 |

---

## runtime_locks / background_jobs

运行时表不承载长期知识、记忆或审计事实。

| 表名 | 说明 |
|------|------|
| `runtime_locks` | 文件写入、解析、索引、回顾任务的互斥锁 |
| `background_jobs` | 外部资料解析、L1 缓存生成、索引重建、后台回顾任务游标 |

---

## 归档策略

| 数据类型 | 保留策略 | 说明 |
|----------|----------|------|
| Sessions / Turns | 永久保留，用户主动删除 | 活跃会话恢复数据 |
| Session Archive | Vault Markdown | 会话进入回顾或审计后生成可迁移摘要 |
| context_index | 可重建 | 从 Vault Markdown、Inbox Markdown、sources 和 agent 资产重建 |
| context_fts | 可重建 | 从 ContextIndex 和 Markdown 重建 |
| source_index | 可重建 | 从 `sources/` manifest、原始文件和 Inbox 来源引用重建 |
| checklist_index | 可重建 | 从 Markdown checklist 重建 |
| review_queue_index | 可重建 | 从 `inbox/review/*.md` Frontmatter 重建 |
| evolution_timeline_index | 可重建 | 从 `agent/evolution/*.md` 重建 |
| private_index | 可重建 | 从 `private/` Markdown 重建 |
| semantic_cache | 可删除 | 后台异步重建 |
| runtime_locks / background_jobs | 不可重建 | 运行时状态，异常退出后可清理或恢复 |

---

## 相关文件

| 文件 | 说明 |
|------|------|
| `src-tauri/src/storage/database/global.rs` | 全局 DB 打开与迁移 |
| `src-tauri/src/storage/database/vault.rs` | Vault DB 打开与迁移 |
| `src-tauri/src/storage/migrations/001_init.sql` | 全局 DB 迁移 |
| `src-tauri/src/storage/migrations/vault_001_init.sql` | Vault DB 迁移 |
