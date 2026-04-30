> **Status**: `active`
>
> 本文档描述目标 SQLite 表、运行时表和并发模型。SQLite 只保存运行恢复、查询加速、可删除缓存和后台任务状态；内容真相源见 [../06-storage.md](../06-storage.md)。

# 数据库说明

MindClaw 使用双 SQLite 数据库架构：

- **Global DB**（`~/.config/mindclaw/mindclaw.db`）：保存跨 Vault 的活跃会话运行时数据。
- **Vault DB**（`{vault}/.mindclaw/mindclaw.db`）：保存当前 Vault 的可重建索引、全文搜索、可选语义缓存和后台任务状态。

长期内容不写入 SQLite 正文。知识、Daily、Inbox、Agent Memory 和 EvolutionLog 都以 Markdown + Frontmatter 为真相源；`resources/` 只保存原始资源和 manifest；`private/` 是 Vault 文件夹，不进入数据库索引。

---

## 当前代码状态

当前 Rust migration 仍是旧表结构：

- Global migration 已包含 `sessions`、`turns`。
- Vault migration 仍包含 `tasks_index`、`notes_index`、`memories_index`。

本文档描述目标数据库设计。本轮只更新文档，不修改 `src-tauri/src/storage/migrations/` 或 Rust 存储代码；代码迁移需要单独实现计划。

---

## 设计原则

**Markdown 是真相源**：凡是需要人类审阅、迁移、纠偏或长期保留的内容都写入 Markdown 或原始资源文件。

**SQLite 是派生层**：业务索引表必须能从 Vault Markdown、Inbox Markdown、Agent Markdown、resource manifest 和文件路径重建。

**运行状态单独保存**：活跃会话 turn、后台任务、锁和游标可以写入 SQLite，因为它们用于恢复或调度，不是长期知识事实。

**少表优先**：除 `context_index`、`context_fts`、`checklist_index` 和运行时表外，不为 resource、review queue、evolution timeline 建独立目标表；这些视图通过 `context_index` 查询派生。

---

## 并发模型

- **WAL 模式**：`PRAGMA journal_mode = WAL`，支持读写并发。
- **忙等待超时**：`PRAGMA busy_timeout = 5000`，5 秒超时后返回错误。
- **外键约束**：`PRAGMA foreign_keys = ON`。
- **连接模型**：每个 DB 使用 `Arc<Mutex<Connection>>`，Tokio 异步运行时通过 Mutex 序列化访问。
- **重建策略**：可重建表允许删除后重建；运行时表不可重建，但异常退出后可以清理或恢复。

---

## Global DB

Global DB 只保存跨 Vault 的活跃会话恢复数据。

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
| `session_id` | TEXT | NOT NULL, FK -> sessions(id) | 所属会话 |
| `user_message` | TEXT | NOT NULL | 用户消息 JSON |
| `assistant_message` | TEXT | | Agent 响应 JSON |
| `tool_trace` | TEXT | NOT NULL DEFAULT '[]' | 工具执行轨迹 JSON |
| `run_status` | TEXT | NOT NULL DEFAULT 'success' | 执行状态 |
| `created` | TEXT | NOT NULL | 创建时间 |

**索引**：

- `idx_turns_session`

活跃 turn 用于会话恢复和证据追溯。进入长期审计或回顾链路时，EvolutionLog 通过 `refs` 引用相关 session / turn / tool trace，并在正文中记录必要证据摘要；Vault 不再生成完整会话归档 Markdown。

---

## Vault DB

Vault DB 第一版只保留必要索引、可选缓存和运行时表。

| 表名 | 写入方 | 说明 | 可重建 |
|------|--------|------|--------|
| `context_index` | ContextStore | 文档级统一索引，覆盖 Vault、Inbox、Agent 资产和 resource manifest | 是 |
| `context_fts` | ContextStore | 本地全文搜索索引 | 是 |
| `checklist_index` | ChecklistService | Markdown checklist 行级索引 | 是 |
| `semantic_cache` | ContextStore | 可选摘要、embedding 引用和 rerank 缓存 | 是，可删除 |
| `runtime_locks` | Runtime / Services | 文件写入锁、任务锁 | 否 |
| `background_jobs` | Runtime / Services | 解析、索引、缓存生成、后台回顾任务游标 | 否 |

### context_index

`context_index` 是 Vault 级文档索引。它不保存正文真相，只保存从 Frontmatter、文件路径、manifest 和受管资产状态派生的 L0 / L1 检索字段。

| 列名 | 类型 | 约束 | 说明 |
|------|------|------|------|
| `uri` | TEXT | PRIMARY KEY | ContextURI |
| `space` | TEXT | NOT NULL CHECK | vault / resource / inbox / agent |
| `path` | TEXT | NOT NULL UNIQUE | Vault 相对路径或 resource manifest 路径 |
| `title` | TEXT | NOT NULL | 标题 |
| `tags` | TEXT | NOT NULL DEFAULT '[]' | JSON 数组 |
| `overview` | TEXT | NOT NULL DEFAULT '' | L1 概览 |
| `confidence` | REAL | NOT NULL DEFAULT 0.5 CHECK (`confidence` >= 0.0 AND `confidence` <= 1.0) | 0.0-1.0 置信度，从 Frontmatter 派生 |
| `origin` | TEXT | NOT NULL CHECK | user / agent / external |
| `asset_kind` | TEXT | | parse_result / memory_proposal / memory / evolution_log 等 |
| `status` | TEXT | | draft / pending / processing / reviewed / confirmed / rejected / archived / deleted 等 |
| `owner` | TEXT | | user / agent / shared；仅 Agent 资产或记忆相关条目使用 |
| `updated_at` | TEXT | NOT NULL | 更新时间 |
| `frontmatter_hash` | TEXT | NOT NULL | Frontmatter 哈希 |
| `content_hash` | TEXT | | 正文、manifest 或原始资源哈希 |

**索引**：

- `idx_context_space`
- `idx_context_origin`
- `idx_context_confidence`
- `idx_context_asset_kind`
- `idx_context_status`
- `idx_context_owner`
- `idx_context_updated`

以下能力不再建独立目标表，统一从 `context_index` 派生：

| 能力 | 派生方式 |
|------|----------|
| Resource 映射 | `space = resource` 的 manifest、`refs` 和 `content_hash` |
| Review Queue | `space = inbox` + `asset_kind` + `status` |
| Evolution Timeline | `space = agent` + `asset_kind = evolution_log` + `updated_at` |
| Agent Memory 列表 | `space = agent` + `asset_kind = memory` + `owner` |
| Inbox 状态列表 | `space = inbox` + `status` + `updated_at` |

### context_fts

`context_fts` 使用 SQLite FTS 保存搜索字段，用于本地搜索加速。它可以从 Markdown、manifest 和 `context_index` 重建。

| 列名 | 说明 |
|------|------|
| `uri` | ContextURI |
| `title` | 标题 |
| `tags_text` | tags 展开文本 |
| `overview` | 概览 |
| `body_excerpt` | 可重建正文片段 |

### checklist_index

Checklist 仍然是 Markdown checklist 的行级索引，不成为独立任务真相源。

| 列名 | 类型 | 约束 | 说明 |
|------|------|------|------|
| `id` | TEXT | PRIMARY KEY | checklist 项稳定 ID |
| `content` | TEXT | NOT NULL | checklist 内容 |
| `status` | TEXT | NOT NULL CHECK | todo / done |
| `priority` | TEXT | | low / medium / high |
| `due_date` | TEXT | | YYYY-MM-DD |
| `file_path` | TEXT | NOT NULL | Vault 相对路径 |
| `line_number` | INTEGER | | 行号 |
| `created` | TEXT | NOT NULL | 创建时间 |
| `updated` | TEXT | NOT NULL | 更新时间 |

**索引**：

- `idx_checklist_status`
- `idx_checklist_due_date`
- `idx_checklist_updated`

### semantic_cache

`semantic_cache` 是可选缓存，不是 MVP 必需表，不保存内容真相。

| 字段 | 说明 |
|------|------|
| `uri` | ContextURI |
| `cache_kind` | summary / embedding_ref / rerank_hint |
| `cache_value` | 缓存值或外部向量引用 |
| `source_hash` | 生成缓存时使用的源内容哈希 |
| `updated_at` | 缓存更新时间 |

缓存失效或删除后，系统从 Markdown、manifest 和 `context_index` 重新生成。

### runtime_locks / background_jobs

运行时表不承载长期知识、记忆或审计事实。

| 表名 | 说明 |
|------|------|
| `runtime_locks` | 文件写入、解析、索引、回顾任务的互斥锁 |
| `background_jobs` | 外部资源解析、L1 缓存生成、索引重建、后台回顾任务游标 |

---

## 不进入 SQLite 的内容

| 内容 | 真相源 | 说明 |
|------|--------|------|
| 知识、项目笔记、Daily | Vault Markdown | `context_index` 只保存检索字段 |
| Inbox 解析结果、草稿、候选 | `inbox/**/*.md` | 审核状态写在 Frontmatter |
| Agent Memory | `agent/memory/*.md` | 数据库只保存索引字段 |
| EvolutionLog | `agent/evolution/*.md` | 时间线从 `context_index` 派生 |
| Agent 经验教训 | `agent/memory/*.md` | 作为 Memory 类型保存，不设独立 lessons 目录 |
| 会话证据 | Global DB sessions / turns | EvolutionLog 通过 `refs` 引用，不设 Vault 会话归档 |
| 外部原始资源 | `resources/` 原始文件和 manifest | manifest 可进入 `context_index` |
| Private 内容 | `private/` Markdown | 不进入 `context_index`，不建立独立索引 |

---

## 保留策略

| 数据类型 | 保留策略 | 说明 |
|----------|----------|------|
| `sessions` / `turns` | 永久保留，用户主动删除 | 活跃会话恢复数据 |
| `context_index` | 可重建 | 从 Markdown、manifest 和文件路径重建 |
| `context_fts` | 可重建 | 从 `context_index` 和 Markdown 正文片段重建 |
| `checklist_index` | 可重建 | 从 Markdown checklist 重建 |
| `semantic_cache` | 可删除 | 后台异步重建 |
| `runtime_locks` / `background_jobs` | 不可重建 | 运行时状态，异常退出后可清理或恢复 |

---

## 相关文件

| 文件 | 说明 |
|------|------|
| `src-tauri/src/storage/database/global.rs` | Global DB 打开与迁移 |
| `src-tauri/src/storage/database/vault.rs` | Vault DB 打开与迁移 |
| `src-tauri/src/storage/migrations/001_init.sql` | 当前 Global DB 迁移 |
| `src-tauri/src/storage/migrations/vault_001_init.sql` | 当前 Vault DB 迁移，仍待迁移到本文档目标设计 |
