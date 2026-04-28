> **Status**: `active`
>
> 本文档描述当前 SQLite 数据库表结构、索引和并发模型。随表结构变更同步更新。

# 数据库说明

MindClaw 当前使用双 SQLite 数据库架构：

- **全局 DB**（`~/.config/mindclaw/mindclaw.db`）：存储跨 vault 的会话和回合。
- **Vault DB**（`{vault}/.mindclaw/mindclaw.db`）：存储当前 vault 的任务、笔记和记忆索引。

---

## 并发模型

- **WAL 模式**：`PRAGMA journal_mode = WAL`，支持读写并发。
- **忙等待超时**：`PRAGMA busy_timeout = 5000`，5 秒超时后返回错误。
- **外键约束**：`PRAGMA foreign_keys = ON`。
- **连接模型**：每个 DB 使用 `Arc<Mutex<Connection>>`，Tokio 异步运行时通过 Mutex 序列化访问。

---

## 全局 DB 表清单

| 表名 | 写入方 | 说明 |
|------|--------|------|
| `sessions` | SessionManager | 对话会话元数据 |
| `turns` | SessionManager | 完整对话回合 |

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

---

## Vault DB 表清单

| 表名 | 写入方 | 说明 |
|------|--------|------|
| `tasks_index` | TaskService | 任务索引，可从 Markdown 重建 |
| `notes_index` | NoteService | 笔记索引，可从 Markdown 重建 |
| `memories_index` | MemoryStore | 记忆索引，可从 Markdown 重建 |

当前 Vault DB 中的索引表均可从 Markdown 文件重建。

### tasks_index

| 列名 | 类型 | 约束 | 说明 |
|------|------|------|------|
| `id` | TEXT | PRIMARY KEY | 任务 UUID |
| `title` | TEXT | NOT NULL | 任务标题 |
| `status` | TEXT | NOT NULL CHECK | todo / in_progress / done / cancelled |
| `priority` | TEXT | NOT NULL DEFAULT 'medium' CHECK | low / medium / high |
| `due_date` | TEXT | | YYYY-MM-DD |
| `tags` | TEXT | NOT NULL DEFAULT '[]' | JSON 数组 |
| `file_path` | TEXT | NOT NULL UNIQUE | Vault 相对路径 |
| `created` | TEXT | NOT NULL | 创建时间 |
| `updated` | TEXT | NOT NULL | 更新时间 |

**索引**：

- `idx_tasks_status`
- `idx_tasks_due_date`
- `idx_tasks_updated`

### notes_index

| 列名 | 类型 | 约束 | 说明 |
|------|------|------|------|
| `id` | TEXT | PRIMARY KEY | file_path 的稳定 hash |
| `title` | TEXT | NOT NULL | 笔记标题 |
| `tags` | TEXT | NOT NULL DEFAULT '[]' | JSON 数组 |
| `file_path` | TEXT | NOT NULL UNIQUE | Vault 相对路径 |
| `modified_at` | TEXT | NOT NULL | 文件 mtime |

**索引**：

- `idx_notes_title`
- `idx_notes_modified`

### memories_index

| 列名 | 类型 | 约束 | 说明 |
|------|------|------|------|
| `id` | TEXT | PRIMARY KEY | 记忆 UUID |
| `key` | TEXT | NOT NULL UNIQUE | 记忆键 |
| `category` | TEXT | NOT NULL | user_fact / preference / work_context / relationship / goal |
| `importance` | REAL | NOT NULL DEFAULT 0.5 | 重要性 |
| `file_path` | TEXT | NOT NULL UNIQUE | Vault 相对路径 |
| `updated` | TEXT | NOT NULL | 更新时间 |

**索引**：

- `idx_memories_category`
- `idx_memories_importance`
- `idx_memories_key`

---

## 完整 DDL

当前 DDL 以迁移文件为准：

- `src-tauri/src/storage/migrations/001_init.sql`
- `src-tauri/src/storage/migrations/vault_001_init.sql`

---

## 归档策略

| 数据类型 | 保留策略 | 说明 |
|----------|----------|------|
| Sessions | 永久保留 | 用户主动删除 |
| Turns | 永久保留 | 用户主动删除 |
| tasks_index | 可重建 | 从 `tasks/*.md` 重建 |
| notes_index | 可重建 | 从 vault 扫描重建 |
| memories_index | 可重建 | 从 memory Markdown 文件重建 |

---

## 相关文件

| 文件 | 说明 |
|------|------|
| `src-tauri/src/storage/database/global.rs` | 全局 DB 打开与迁移 |
| `src-tauri/src/storage/database/vault.rs` | Vault DB 打开与迁移 |
| `src-tauri/src/storage/migrations/001_init.sql` | 全局 DB v1 迁移 |
| `src-tauri/src/storage/migrations/vault_001_init.sql` | Vault DB v1 迁移 |
