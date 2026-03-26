# MindClaw 技术架构设计

> 拆分自 architecture.md，完整索引见 [README.md](./README.md)

## 五、存储架构

### 核心原则

**Markdown 是内容真相，SQLite 是查询索引。** SQLite 和向量索引都是 Markdown 的派生层，丢失可从 Markdown 完整重建。

### SQLite 表结构

```sql
-- Markdown 索引（派生，可从文件系统重建）
-- 三级索引：L0 Tags / L1 Overview / L2 Detail（全文在文件系统）
-- 笔记和目录统一存储，kind 区分类型，共享 L0/L1 检索路径
CREATE TABLE notes (
  id         TEXT PRIMARY KEY,
  path       TEXT UNIQUE NOT NULL,  -- 笔记: "knowledge/投资/价值投资.md"（有 .md 后缀）
                                    -- 目录: "knowledge/投资"（无后缀，从文件系统目录派生）
  title      TEXT,
  tags       TEXT,           -- JSON 数组（L0，~100 tokens）
                             --   笔记: 从 frontmatter 提取
                             --   目录: 聚合子笔记 tags（去重合并）
  overview   TEXT,           -- ~2k tokens 概要（L1）
                             --   笔记: 从 frontmatter 提取（Haiku 生成或人工编写）
                             --   目录: 聚合子笔记概要
  source     TEXT,           -- 来源标识（从 frontmatter 提取，仅笔记有）
                             --   NULL             — 用户手动创建
                             --   'https://...'    — 从 URL 解析
                             --   'file://...pdf'  — 从 PDF 解析
                             --   'session:abc123' — 对话沉淀（关联会话 ID）
                             --   'capture:xyz'    — 捕获路由（关联 capture ID）
  -- parent_dir 和 note_count 不需要：
  --   父目录从 path 推导（如 "knowledge/投资/价值投资.md" → "knowledge/投资"）
  --   子节点查询用 WHERE path LIKE 'knowledge/投资/%'
  --   子笔记计数用 COUNT(*) 实时计算
  created    TEXT NOT NULL,
  updated    TEXT NOT NULL,
  status     TEXT DEFAULT 'active',
  last_indexed TEXT
);
-- 笔记 vs 目录的判断：path LIKE '%.md' 即为笔记，否则为目录

-- L0 全文索引（FTS5，笔记和目录统一搜索）
CREATE VIRTUAL TABLE notes_fts USING fts5(
  title, tags,
  content='notes', content_rowid='rowid'
);

-- 子节点查询直接用 path LIKE 'dir/%'，path 的 UNIQUE 索引已覆盖前缀匹配

-- 任务（一等公民，独立结构）
CREATE TABLE tasks (
  id        TEXT PRIMARY KEY,
  content   TEXT NOT NULL,
  status    TEXT DEFAULT 'pending',  -- pending | in_progress | done | cancelled
  due       TEXT,
  note_path TEXT,
  context   TEXT,
  created   TEXT NOT NULL,
  completed TEXT
);

-- 笔记链接关系（从 wikilinks 提取，派生）
CREATE TABLE links (
  source_path TEXT NOT NULL,
  target_path TEXT NOT NULL,
  context     TEXT,
  PRIMARY KEY (source_path, target_path)
);

-- Memory Layer: Agent 私有记忆（单表统一，不进 Markdown）
CREATE TABLE memories (
  id             TEXT PRIMARY KEY,
  key            TEXT UNIQUE NOT NULL,   -- 去重键，同一认知 upsert 而非 insert
  content        TEXT NOT NULL,          -- 记忆内容
  category       TEXT NOT NULL,          -- observation | preference | pattern
  type           TEXT,                   -- 子类型：insight/blindspot/emotion | communication_style | emotion_trend
  namespace      TEXT DEFAULT 'default', -- 上下文隔离（不同角色/模式下的记忆）
  importance     REAL DEFAULT 0.5,       -- 重要度（recall 排序、衰减基准）
  session_id     TEXT,                   -- 关联会话（溯源）
  related_path   TEXT,                   -- 关联笔记路径
  embedding      BLOB,                   -- 向量（Phase 2 语义检索）
  surfaced       INTEGER DEFAULT 0,      -- 是否已浮出给用户
  superseded_by  TEXT,                   -- 被哪条新记忆替代（认知演进链）
  created_at     TEXT NOT NULL,
  updated_at     TEXT NOT NULL
);

-- 索引：按 category 筛选 + importance 排序
CREATE INDEX idx_memories_category ON memories(category, importance DESC);
-- 索引：按 namespace 隔离
CREATE INDEX idx_memories_namespace ON memories(namespace);
-- 索引：未浮出的记忆（ContextBuilder 注入用）
CREATE INDEX idx_memories_unsurfaced ON memories(surfaced, importance DESC)
  WHERE surfaced = 0 AND superseded_by IS NULL;

-- 捕获队列
CREATE TABLE capture_queue (
  id         TEXT PRIMARY KEY,
  raw        TEXT NOT NULL,
  type       TEXT,  -- task | thought | feeling | link
  source     TEXT DEFAULT 'desktop',
  created    TEXT NOT NULL,
  processed  INTEGER DEFAULT 0,
  routed_to  TEXT
);

-- 对话会话
CREATE TABLE sessions (
  id      TEXT PRIMARY KEY,
  sender  TEXT NOT NULL,  -- canonical user ID（经 UserIdentityResolver 统一后）
  mode    TEXT NOT NULL,  -- companion | reflect | challenge | knowledge | treehole
  created TEXT NOT NULL,
  updated TEXT NOT NULL,
  summary TEXT
);

-- 索引：按 sender + mode 查找活跃会话
CREATE INDEX idx_sessions_sender_mode ON sessions(sender, mode);

-- 对话消息（热存，90 天后转冷归档）
CREATE TABLE messages (
  id         TEXT PRIMARY KEY,
  session_id TEXT NOT NULL REFERENCES sessions(id),
  role       TEXT NOT NULL,  -- user | assistant
  content    TEXT NOT NULL,
  created    TEXT NOT NULL
);

-- 用户角色
CREATE TABLE user_roles (
  id         TEXT PRIMARY KEY,
  name       TEXT NOT NULL,
  priority   INTEGER DEFAULT 0,
  weak_point TEXT,
  created    TEXT NOT NULL
);

-- Agent 记忆偏好等在 memories 表中（category='preference'）
```

### Markdown 与 SQLite 同步

- **写入时**：Markdown 先写，然后更新 SQLite 索引（frontmatter tags/overview → notes 表 L0/L1）
- **写入失败恢复**：如果 SQLite 索引更新失败，写入 `data/.index_dirty` 脏标记文件，下次启动时立即触发全量重建
- **冲突时**：Markdown frontmatter 为权威，SQLite 索引可随时从文件系统重建
- **重建索引**：启动时先检查 `.index_dirty` 标记，再检查 `last_indexed` 与文件 mtime，仅增量更新

### 知识笔记三级索引（L0 / L1 / L2）

知识笔记采用渐进式加载策略，用最少的 token 做最精准的检索：

| Level | Name | Token 限制 | 存储位置 | 用途 |
|-------|------|-----------|---------|------|
| **L0** | Tags | ~100 tokens | SQLite `notes.tags`（JSON 数组） | 向量搜索、FTS5 过滤、分类筛选、快速扫描 |
| **L1** | Overview | ~2k tokens | SQLite `notes.overview` | 重排序、内容导航、RAG 上下文注入 |
| **L2** | Detail | 无限制 | 文件系统 `vault/knowledge/*.md` | 完整内容，Agent 按需加载 |

**L0 就是 tags**——精心设计的标签本身就是最好的语义摘要，天然适合向量化和精确匹配，不需要额外的 abstract 字段。

#### Markdown 格式规范

每篇知识笔记的 frontmatter 包含 `tags`（L0）和 `overview`（L1），正文即完整内容（L2）：

```markdown
---
title: 价值投资的核心原则
tags: [投资, 价值投资, 巴菲特, 安全边际, 内在价值, 能力圈, 长期持有]
overview: |
  价值投资由格雷厄姆创立，核心三原则：安全边际（内在价值与市场价格的差距）、
  能力圈（只投资自己理解的领域）、长期持有（利用复利效应）。
  关键指标包括 PE/PB 估值、自由现金流、护城河宽度。
  与成长投资的本质区别在于对"确定性"的定价方式不同。
source: https://example.com/value-investing-guide
created: 2026-03-15
updated: 2026-03-20
---

价值投资由本杰明·格雷厄姆创立……


## 安全边际

……完整内容……
```

**`source` 字段**——单一字段，类型从值自身推断：

| 值 | 含义 | 示例 |
|----|------|------|
| NULL | 用户手动创建 | — |
| `https://...` | 从 URL 网页解析 | `https://example.com/article` |
| `file://...` | 从本地 PDF/文件解析 | `file:///Users/.../paper.pdf` |
| `session:ID` | 对话沉淀（SubAgent 提炼） | `session:abc123` |
| `capture:ID` | 捕获路由（Inbox → Knowledge） | `capture:xyz789` |

Agent 解析 URL/PDF 的流程：用户发送链接或文件 → Agent 提取内容 → Haiku 生成 tags（L0）+ overview（L1）→ Sonnet 提炼正文为结构化知识笔记（L2）→ 写入 frontmatter + vault，等待人类审核确认。

- `tags` = **L0**（~100 tokens，从 frontmatter 提取，存入 SQLite + FTS5）
  - tags 是 Agent 的第一视角——扫描 tags 就能判断这篇笔记"关于什么"
  - tags 设计原则：覆盖核心概念 + 关联领域 + 关键人名/术语，总量控制在 ~100 tokens
- `overview` = **L1**（~2k tokens，从 frontmatter 提取，缓存到 SQLite `notes.overview`）
  - overview 是知识的结构化概要，Agent 读它即可理解核心内容，无需加载全文
  - 首次创建时由 SubAgent Haiku 从正文生成，写回 frontmatter 持久化
  - 人类可手动编辑 overview 提高精度（frontmatter 是真相源，SQLite 是缓存）
  - 笔记正文更新时，SubAgent 异步重新生成 overview 并写回 frontmatter
- 完整 Markdown 正文 = **L2**（仅在 Agent 明确需要时从文件系统读取）

**Markdown 文件即完整真相**：L0（tags）+ L1（overview）+ L2（正文）全部在一个 `.md` 文件中。SQLite 中的 `tags` 和 `overview` 列是 frontmatter 的派生缓存，丢失可从文件系统重建。

#### 目录级聚合索引

知识按主题组织为目录（如 `knowledge/投资/`、`knowledge/教育/`）。每个目录自动维护聚合索引：

```
vault/knowledge/投资/
  ├── 价值投资.md                  # 单篇笔记 (L2)
  ├── 风险管理.md
  └── 量化策略.md

SQLite notes 表（目录也是一条记录，path 无 .md 后缀）：
  path: "knowledge/投资"
  tags: ["投资", "价值投资", "风险管理", "量化", "巴菲特", ...]  (聚合 L0)
  overview: "3 篇笔记：价值投资核心原则、风险管理框架、量化策略入门..."  (聚合 L1)
```

目录和笔记统一在 `notes` 表中，通过 `kind` 区分。L0 搜索只查一张表，检索路径统一。目录 L0（tags）在子笔记 CRUD 时自动聚合——合并去重子笔记的所有 tags。目录 L1 由 Haiku 从子笔记 L1 聚合生成。

#### RAG 检索流程（渐进式加载）

```
用户消息 "如何控制投资风险？"
  │
  ├── Step 1: L0 粗筛（tags 匹配，低成本，高召回）
  │   FTS5 搜索 notes_fts(title, tags)，笔记和目录统一命中
  │   → 命中目录 "knowledge/投资"（tags 含 "投资", "风险管理"）
  │   → 命中笔记 "knowledge/投资/风险管理.md", "knowledge/投资/价值投资.md" 等
  │   → 候选集 ~20 条 L0 tags（~2000 tokens）
  │
  ├── Step 2: L1 重排序 + 目录递归
  │   对候选集加载 L1 overview（从 SQLite 读，无磁盘 IO）
  │   按关键词重叠度 + tags 匹配度排序
  │   高分目录内递归：检查 "knowledge/投资/" 下所有子笔记
  │   → Top 3-5 条 L1（~6k-10k tokens）
  │
  ├── Step 3: L1 注入上下文
  │   ContextBuilder 将 Top L1 注入 System Prompt
  │   Agent 基于 L1 概要理解知识全貌
  │
  └── Step 4: L2 按需加载（Agent 主动请求）
      Agent 判断需要某篇完整内容时：
      tool_call("operations", {action: "call",
        name: "knowledge_get", args: {path: "knowledge/投资/风险管理.md"}})
      → 从文件系统读取完整 Markdown 返回
```

**与传统 RAG 的区别**：传统方案将全文切片后向量检索，返回碎片化的 snippet。MindClaw 的三级方案保持知识的完整性——L1 是结构化概要而非随机切片，Agent 始终能看到知识的完整轮廓，需要细节时再加载 L2。

#### L1 生成策略

| 策略 | 方式 | 写入位置 | 适用场景 |
|------|------|---------|---------|
| **Haiku 生成** | SubAgent 从正文生成结构化概要 | 写入 frontmatter `overview` 字段 | 默认策略，笔记创建/更新时异步触发 |
| **人工编写** | 用户直接编辑 frontmatter overview | frontmatter（真相源） | 高价值笔记需精确概要 |
| **截断兜底** | 取正文前 ~2k tokens | 仅缓存到 SQLite（不写 frontmatter） | Haiku 调用失败时的降级策略 |

overview 的生命周期：笔记创建 → SubAgent 异步生成 overview → 写回 frontmatter → 同步到 SQLite 缓存。人类编辑 frontmatter 中的 overview 后，下次索引时以 frontmatter 为准覆盖 SQLite。

#### 索引更新触发

| 触发事件 | 更新内容 |
|---------|---------|
| 笔记创建/更新 | 提取 frontmatter tags → L0；提取 frontmatter overview → L1（无则 Haiku 生成写回）；更新 FTS5；聚合 parent_dir 目录的 L0/L1 |
| 笔记删除 | 移除 notes/notes_fts 记录；重新聚合所属目录 |
| 新目录出现 | 自动插入 kind='dir' 记录，聚合子笔记 tags → L0，Haiku 生成 L1 |
| 定时任务 index_rebuild | 增量对比 mtime，修复不一致，补全缺失的目录记录 |

### 对话历史分层

| 层 | 内容 | 存储 | 保留 |
|---|------|------|------|
| 原始消息 | 每句对话 | SQLite messages 表 | 90 天 |
| 会话摘要 | 每次会话精华 | SQLite sessions.summary | 永久 |
| 蒸馏知识 | 提炼的洞见 | vault/knowledge/ Markdown | 永久 |

90 天后原始消息导出为 JSONL 冷归档（`data/archive/YYYY-MM.jsonl`），SQLite 中删除。

### 设置存储分工

```
settings.json              OS Keychain                 SQLite
─────────────              ─────────────               ──────
LLM 模型选择               API Key（加密）              角色模版
主题 / 语言                Gateway Bearer Token        Agent 学习偏好
Vault 路径                                              使用统计
同步配置
Token 预算（可选覆盖）
```

API Key 和 Gateway Bearer Token 必须存入 OS Keychain，绝不能存在任何明文文件中。

---
