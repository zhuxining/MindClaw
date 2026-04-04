> **Status**: `active`
>
> 本文档描述 SQLite 数据库表结构、索引和并发模型。随表结构变更同步更新。

# 数据库说明

## 并发模型

- **WAL 模式**：`PRAGMA journal_mode = WAL`，支持读写并发
- **忙等待超时**：`PRAGMA busy_timeout = 5000`，5 秒超时后返回错误
- **外键约束**：`PRAGMA foreign_keys = ON`，级联操作由应用层控制
- **连接模型**：单连接 + `Arc<Mutex<Connection>>`，Tokio 异步运行时通过 Mutex 序列化访问

---

## 表清单

| 表名 | 写入方 | 说明 |
|------|--------|------|
| `sessions` | AgentLoop | 对话会话元数据 |
| `turns` | AgentLoop | 完整对话回合（用户输入 + Agent 响应） |
| `messages` | AgentLoop | 扁平化消息索引，供前端查询 |

---

## 表结构与索引

### sessions

会话元数据表。

| 列名 | 类型 | 约束 | 说明 |
|------|------|------|------|
| `id` | TEXT | PRIMARY KEY | 会话唯一标识（即 `session_key`） |
| `sender` | TEXT | NOT NULL | 发送者标识，格式 `{channel}:{chat_id}` |
| `mode` | TEXT | NOT NULL | 会话模式（如 `default`、`task`、`knowledge`） |
| `summary` | TEXT | | 会话摘要，由记忆整合生成 |
| `created` | TEXT | NOT NULL | 创建时间（ISO 8601） |
| `updated` | TEXT | NOT NULL | 最后更新时间（ISO 8601） |

**索引**：

- `idx_sessions_sender`：按发送者查询
- `idx_sessions_updated`：按更新时间倒序查询

### turns

对话回合表，记录一次完整的"用户输入 + Agent 响应"交互。

| 列名 | 类型 | 约束 | 说明 |
|------|------|------|------|
| `id` | INTEGER | PRIMARY KEY AUTOINCREMENT | 回合自增 ID |
| `session_id` | TEXT | NOT NULL, FK → sessions(id) | 所属会话 |
| `user_message` | TEXT | NOT NULL | 用户消息 JSON（`ChatMessage` 序列化） |
| `assistant_message` | TEXT | | Agent 响应 JSON，失败/取消时为 NULL |
| `tool_trace` | TEXT | NOT NULL DEFAULT '[]' | 工具执行轨迹 JSON 数组 |
| `run_status` | TEXT | NOT NULL DEFAULT 'success' | 执行状态：`success` / `failed:reason` / `cancelled` |
| `created` | TEXT | NOT NULL | 创建时间（ISO 8601） |

**索引**：

- `idx_turns_session`：按会话 ID 查询回合列表

### messages

扁平化消息索引表，供前端快速查询消息列表，不存储完整工具轨迹。

| 列名 | 类型 | 约束 | 说明 |
|------|------|------|------|
| `id` | TEXT | PRIMARY KEY | 消息唯一标识（UUID） |
| `session_id` | TEXT | NOT NULL, FK → sessions(id) | 所属会话 |
| `role` | TEXT | NOT NULL | 消息角色：`user` / `assistant` / `system` |
| `content` | TEXT | NOT NULL | 消息内容文本 |
| `created` | TEXT | NOT NULL | 创建时间（ISO 8601） |

**索引**：

- `idx_messages_session`：按会话 ID 和时间查询消息列表

---

## 归档策略

| 数据类型 | 保留策略 | 清理触发条件 |
|----------|----------|--------------|
| Sessions | 永久保留 | 用户主动删除 |
| Turns | 永久保留 | 用户主动删除 |
| Messages | 与 Turns 同步 | Turns 删除时级联删除 |

---

## 完整 DDL

```sql
-- 会话表
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

-- 回合表
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

-- 消息索引表
CREATE TABLE IF NOT EXISTS messages (
  id         TEXT PRIMARY KEY,
  session_id TEXT NOT NULL REFERENCES sessions(id),
  role       TEXT NOT NULL,
  content    TEXT NOT NULL,
  created    TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_messages_session ON messages(session_id, created);
```

---

## Schema 版本

当前版本：`1`

迁移文件位置：`src/storage/migrations/001_init.sql`

版本升级逻辑见 `src/storage/database.rs` 中的 `migrate()` 函数。
