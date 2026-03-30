-- 001: 初始 Schema

-- 对话会话
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

-- 对话回合
CREATE TABLE IF NOT EXISTS turns (
  id             INTEGER PRIMARY KEY AUTOINCREMENT,
  session_id     TEXT NOT NULL REFERENCES sessions(id),
  user_message   TEXT NOT NULL,       -- JSON: ChatMessage
  assistant_message TEXT,             -- JSON: ChatMessage (nullable: failed/cancelled)
  tool_trace     TEXT NOT NULL DEFAULT '[]',  -- JSON: Vec<ToolTrace>
  run_status     TEXT NOT NULL DEFAULT 'success',  -- success | failed:reason | cancelled
  created        TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_turns_session ON turns(session_id, id);

-- 对话消息（扁平化索引，供前端查询用）
CREATE TABLE IF NOT EXISTS messages (
  id         TEXT PRIMARY KEY,
  session_id TEXT NOT NULL REFERENCES sessions(id),
  role       TEXT NOT NULL,
  content    TEXT NOT NULL,
  created    TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_messages_session ON messages(session_id, created);
