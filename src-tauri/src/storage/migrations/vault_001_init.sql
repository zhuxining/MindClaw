-- vault_001: Vault 级索引初始化
-- 真相源是 Markdown 文件，SQLite 只存索引（可丢弃，可重建）

-- Task 索引
CREATE TABLE IF NOT EXISTS tasks_index (
  id        TEXT PRIMARY KEY,
  title     TEXT NOT NULL,
  status    TEXT NOT NULL CHECK(status IN ('todo','in_progress','done','cancelled')),
  priority  TEXT NOT NULL DEFAULT 'medium' CHECK(priority IN ('low','medium','high')),
  due_date  TEXT,                     -- YYYY-MM-DD，可 NULL
  tags      TEXT NOT NULL DEFAULT '[]', -- JSON array of strings
  file_path TEXT NOT NULL UNIQUE,     -- vault 内相对路径，如 "tasks/2026-04-07-foo.md"
  created   TEXT NOT NULL,            -- ISO8601
  updated   TEXT NOT NULL             -- ISO8601
);

CREATE INDEX IF NOT EXISTS idx_tasks_status   ON tasks_index(status);
CREATE INDEX IF NOT EXISTS idx_tasks_due_date ON tasks_index(due_date);
CREATE INDEX IF NOT EXISTS idx_tasks_updated  ON tasks_index(updated DESC);

-- Note 索引（Obsidian 普通笔记，MindClaw 扫描建立）
CREATE TABLE IF NOT EXISTS notes_index (
  id          TEXT PRIMARY KEY,       -- file_path 的 sha256 短 hash，幂等
  title       TEXT NOT NULL,
  tags        TEXT NOT NULL DEFAULT '[]',
  file_path   TEXT NOT NULL UNIQUE,   -- vault 内相对路径
  modified_at TEXT NOT NULL           -- 文件 mtime（ISO8601），用于增量 sync
);

CREATE INDEX IF NOT EXISTS idx_notes_title    ON notes_index(title);
CREATE INDEX IF NOT EXISTS idx_notes_modified ON notes_index(modified_at DESC);

-- Memory 索引
CREATE TABLE IF NOT EXISTS memories_index (
  id         TEXT PRIMARY KEY,
  key        TEXT NOT NULL UNIQUE,
  category   TEXT NOT NULL,
  importance REAL NOT NULL DEFAULT 0.5,
  file_path  TEXT NOT NULL UNIQUE,    -- vault 内相对路径
  updated    TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_memories_category   ON memories_index(category);
CREATE INDEX IF NOT EXISTS idx_memories_importance ON memories_index(importance DESC);
CREATE INDEX IF NOT EXISTS idx_memories_key        ON memories_index(key);

PRAGMA user_version = 1;
