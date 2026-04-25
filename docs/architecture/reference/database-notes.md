> **Status**: `active`
>
> 本文档描述 SQLite 数据库表结构、索引和并发模型。随表结构变更同步更新。

# 数据库说明

MindClaw 使用**双 SQLite 数据库**架构：

- **全局 DB**（`~/.config/mindclaw/mindclaw.db`）：存储跨 vault 的会话/回合
- **Vault DB**（`{vault}/.mindclaw/mindclaw.db`）：存储当前 vault 的任务/笔记/记忆索引

---

## 并发模型

- **WAL 模式**：`PRAGMA journal_mode = WAL`，支持读写并发
- **忙等待超时**：`PRAGMA busy_timeout = 5000`，5 秒超时后返回错误
- **外键约束**：`PRAGMA foreign_keys = ON`
- **连接模型**：每个 DB 使用 `Arc<Mutex<Connection>>`，Tokio 异步运行时通过 Mutex 序列化访问

---

## 全局 DB 表清单

| 表名 | 写入方 | 说明 |
|------|--------|------|
| `sessions` | SessionManager | 对话会话元数据 |
| `turns` | SessionManager | 完整对话回合 |

### sessions

会话元数据表。

| 列名 | 类型 | 约束 | 说明 |
|------|------|------|------|
| `id` | TEXT | PRIMARY KEY | 会话唯一标识 |
| `sender` | TEXT | NOT NULL | 发送者标识 |
| `mode` | TEXT | NOT NULL | 会话模式 |
| `summary` | TEXT | | 会话摘要 |
| `created` | TEXT | NOT NULL | 创建时间（ISO 8601） |
| `updated` | TEXT | NOT NULL | 最后更新时间 |

**索引**：

- `idx_sessions_sender`：按发送者查询
- `idx_sessions_updated`：按更新时间倒序

### turns

对话回合表。

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

- `idx_turns_session`：按会话 ID 查询

---

## Vault DB 表清单

| 表名 | 写入方 | 说明 |
|------|--------|------|
| `tasks_index` | TaskService | 任务索引（可从 Markdown 重建） |
| `notes_index` | NoteService | 笔记索引（可从 Markdown 重建） |
| `memories_index` | MemoryStore | 记忆索引（可从 Markdown 重建） |

**重要**：Vault DB 中的所有索引表均可从 Markdown 文件重建，删除 DB 不会丢失数据。

### tasks_index

任务索引表。

| 列名 | 类型 | 约束 | 说明 |
|------|------|------|------|
| `id` | TEXT | PRIMARY KEY | 任务 UUID |
| `title` | TEXT | NOT NULL | 任务标题 |
| `status` | TEXT | NOT NULL CHECK(...) | todo/in_progress/done/cancelled |
| `priority` | TEXT | NOT NULL DEFAULT 'medium' | low/medium/high |
| `due_date` | TEXT | | YYYY-MM-DD |
| `tags` | TEXT | NOT NULL DEFAULT '[]' | JSON 数组 |
| `file_path` | TEXT | NOT NULL UNIQUE | Vault 相对路径 |
| `created` | TEXT | NOT NULL | ISO 8601 |
| `updated` | TEXT | NOT NULL | ISO 8601 |

**索引**：

- `idx_tasks_status`：按状态查询
- `idx_tasks_due_date`：按截止日期查询
- `idx_tasks_updated`：按更新时间倒序

### notes_index

笔记索引表。

| 列名 | 类型 | 约束 | 说明 |
|------|------|------|------|
| `id` | TEXT | PRIMARY KEY | file_path 的 SHA256 短 hash |
| `title` | TEXT | NOT NULL | 笔记标题 |
| `tags` | TEXT | NOT NULL DEFAULT '[]' | JSON 数组 |
| `file_path` | TEXT | NOT NULL UNIQUE | Vault 相对路径 |
| `modified_at` | TEXT | NOT NULL | 文件 mtime，用于增量 sync |

**索引**：

- `idx_notes_title`：按标题查询
- `idx_notes_modified`：按修改时间倒序

### memories_index

记忆索引表。

| 列名 | 类型 | 约束 | 说明 |
|------|------|------|------|
| `id` | TEXT | PRIMARY KEY | 记忆 UUID |
| `key` | TEXT | NOT NULL UNIQUE | 记忆键 |
| `category` | TEXT | NOT NULL | user_fact/preference/work_context/relationship/goal |
| `importance` | REAL | NOT NULL DEFAULT 0.5 | 重要性 0.0-1.0 |
| `file_path` | TEXT | NOT NULL UNIQUE | Vault 相对路径 |
| `updated` | TEXT | NOT NULL | ISO 8601 |

**索引**：

- `idx_memories_category`：按类别查询
- `idx_memories_importance`：按重要性倒序

---

## 归档策略

| 数据类型 | 保留策略 | 说明 |
|----------|----------|------|
| Sessions | 永久保留 | 用户主动删除 |
| Turns | 永久保留 | 用户主动删除 |
| tasks_index | 可重建 | 从 `tasks/*.md` 重建 |
| notes_index | 可重建 | 从 vault 扫描重建 |
| memories_index | 可重建 | 从 `memory/*.md` 重建 |

---

## Schema 版本

- **全局 DB 版本**：`1`
  - v1: 初始版本（sessions/turns）
- **Vault DB 版本**：`1`
  - v1: 初始版本（tasks_index/notes_index/memories_index）

迁移文件位置：`src/storage/migrations/`

---

## 完整 DDL

### 全局 DB

```sql
-- sessions
CREATE TABLE IF NOT EXISTS sessions (
  id        TEXT PRIMARY KEY,
  sender    TEXT NOT NULL,
  mode      TEXT NOT NULL,
  summary   TEXT,
  created   TEXT NOT NULL,
  updated   TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_sessions_sender ON sessions(sender);
CREATE INDEX IF NOT EXISTS idx_sessions_updated ON sessions(updated DESC);

-- turns
CREATE TABLE IF NOT EXISTS turns (
  id             INTEGER PRIMARY KEY AUTOINCREMENT,
  session_id     TEXT NOT NULL REFERENCES sessions(id),
  user_message   TEXT NOT NULL,
  assistant_message TEXT,
  tool_trace     TEXT NOT NULL DEFAULT '[]',
  run_status     TEXT NOT NULL DEFAULT 'success',
  created        TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_turns_session ON turns(session_id, id);
```

### Vault DB

```sql
-- tasks_index
CREATE TABLE IF NOT EXISTS tasks_index (
  id        TEXT PRIMARY KEY,
  title     TEXT NOT NULL,
  status    TEXT NOT NULL CHECK(status IN ('todo','in_progress','done','cancelled')),
  priority  TEXT NOT NULL DEFAULT 'medium',
  due_date  TEXT,
  tags      TEXT NOT NULL DEFAULT '[]',
  file_path TEXT NOT NULL UNIQUE,
  created   TEXT NOT NULL,
  updated   TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks_index(status);
CREATE INDEX IF NOT EXISTS idx_tasks_due_date ON tasks_index(due_date);
CREATE INDEX IF NOT EXISTS idx_tasks_updated ON tasks_index(updated DESC);

-- notes_index
CREATE TABLE IF NOT EXISTS notes_index (
  id          TEXT PRIMARY KEY,
  title       TEXT NOT NULL,
  tags        TEXT NOT NULL DEFAULT '[]',
  file_path   TEXT NOT NULL UNIQUE,
  modified_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_notes_title ON notes_index(title);
CREATE INDEX IF NOT EXISTS idx_notes_modified ON notes_index(modified_at DESC);

-- memories_index
CREATE TABLE IF NOT EXISTS memories_index (
  id         TEXT PRIMARY KEY,
  key        TEXT NOT NULL UNIQUE,
  category   TEXT NOT NULL,
  importance REAL NOT NULL DEFAULT 0.5,
  file_path  TEXT NOT NULL UNIQUE,
  updated    TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_memories_category ON memories_index(category);
CREATE INDEX IF NOT EXISTS idx_memories_importance ON memories_index(importance DESC);
```

---

## 相关文件

| 文件 | 说明 |
|------|------|
| `src/storage/database/global.rs` | 全局 DB 打开与迁移 |
| `src/storage/database/vault.rs` | Vault DB 打开与迁移 |
| `src/storage/migrations/001_init.sql` | 全局 DB v1 迁移 |
| `src/storage/migrations/vault_001_init.sql` | Vault DB v1 迁移 |
