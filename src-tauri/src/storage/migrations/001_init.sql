-- 001: 全局 DB 初始化（sessions/turns）
-- 注意：messages 表已删除，消息数据从 turns 表解析

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

PRAGMA user_version = 1;
