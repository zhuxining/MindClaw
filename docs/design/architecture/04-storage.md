# MindClaw 技术架构设计 — 存储架构

> 完整架构文档索引见 [README.md](./README.md)

## 五、存储架构

### 核心原则

**Markdown 优先，SQLite 分治。** 知识笔记以 Markdown 为真相源，SQLite 是派生索引，可从 Markdown 重建。任务、记忆、会话等结构化状态以 SQLite 为真相源，必要时同步到 Markdown 作为人类友好视图。

### SQLite 表结构

```sql
-- Markdown 索引（派生，可从文件系统重建）
-- 三级索引：L0 Tags / L1 Overview / L2 Detail（全文在文件系统）
-- 笔记和目录统一存储，path 后缀区分类型（%.md 为笔记，否则为目录）
CREATE TABLE notes (
  id         TEXT PRIMARY KEY,
  path       TEXT UNIQUE NOT NULL,  -- 笔记: "knowledge/投资/价值投资.md"（有 .md 后缀）
                                    -- 目录: "knowledge/投资"（无后缀，从文件系统目录派生）
  title      TEXT,
  tags       TEXT,           -- JSON 数组（L0，~100 tokens）
                             --   笔记: 从 frontmatter 提取
                             --   目录: 聚合子笔记 tags（去重合并）
  overview   TEXT,           -- ~2k tokens 概要（L1）
                             --   笔记: 从 frontmatter 提取（自动生成或人工编写）
                             --   目录: 聚合子笔记概要
  source     TEXT,           -- 创建方式（从 frontmatter 提取，仅笔记有）
                             --   NULL         — 用户手动创建
                             --   'resource'   — 从资源结晶（URL/PDF，详见 resources 表）
                             --   'session:ID' — 对话蒸馏（关联会话 ID，非资源）
  -- parent_dir / note_count 不需要：父目录从 path 推导，子节点用 LIKE 'dir/%.md' 查询
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

-- 任务（SQLite 是真相源，Daily Markdown 是人类友好视图）
-- Daily 中格式：- [ ] 买菜 <!--task:abc123-->
-- 写入方向：SQLite → Markdown；用户在 Obsidian 手写无 ID 的 checkbox 时反向创建
CREATE TABLE tasks (
  id        TEXT PRIMARY KEY,
  content   TEXT NOT NULL,
  status    TEXT DEFAULT 'pending',  -- pending | in_progress | done | cancelled
  due       TEXT,
  note_path TEXT,                    -- 关联笔记路径（daily 或 knowledge，created 日期关联当天 daily）
  context   TEXT,
  created   TEXT NOT NULL,
  completed TEXT
);

-- 资源（外部文件/URL，结晶前的原始材料）
-- 生命周期：pending → parsing → done | failed
-- 与知识笔记 1:1：一个资源结晶为一篇知识笔记
CREATE TABLE resources (
  id          TEXT PRIMARY KEY,
  uri         TEXT UNIQUE NOT NULL,   -- 统一资源标识（https://... | file:///... | skill://name）
  title       TEXT,                   -- 资源标题（从元数据提取）
  type        TEXT NOT NULL,          -- url | pdf | epub | file | skill
  status      TEXT DEFAULT 'pending', -- pending | parsing | done | failed
  note_path   TEXT,                   -- 结晶后指向 notes.path（未结晶时为 NULL）
  created     TEXT NOT NULL,
  updated     TEXT NOT NULL
);
CREATE INDEX idx_resources_status ON resources(status);

-- 笔记链接关系（从 wikilinks 提取，派生）
CREATE TABLE links (
  source_path TEXT NOT NULL,
  target_path TEXT NOT NULL,
  context     TEXT,
  PRIMARY KEY (source_path, target_path)
);

-- Memory Layer: 记忆系统（单表统一，不进 Markdown）
-- category 隐含 owner：profile/preferences/entities/events 归 user，cases/patterns 归 agent
CREATE TABLE memories (
  id             TEXT PRIMARY KEY,
  key            TEXT UNIQUE NOT NULL,   -- 去重键，同一认知 upsert 而非 insert
  content        TEXT NOT NULL,          -- 记忆内容
  category       TEXT NOT NULL,          -- profile | preferences | entities | events | cases | patterns
                                         --   profile:     用户基本信息（角色、背景、目标）
                                         --   preferences: 用户偏好（沟通风格、主题偏好）
                                         --   entities:    实体记忆（人物、项目、组织）
                                         --   events:      事件记录（决策、里程碑、事故）
                                         --   cases:       Agent 学到的案例（成功方案、调试经验）
                                         --   patterns:    Agent 学到的模式（行为规律、偏好趋势）
  importance     REAL DEFAULT 0.5,       -- 重要度（recall 排序、衰减基准）
  session_id     TEXT,                   -- 关联会话（溯源）
  related_path   TEXT,                   -- 关联笔记路径
  embedding      BLOB,                   -- 向量（Phase 2 语义检索）
  surfaced       INTEGER DEFAULT 0,      -- 是否已浮出给用户
  superseded_by  TEXT,                   -- 被哪条新记忆替代（认知演进链）
  created        TEXT NOT NULL,
  updated        TEXT NOT NULL
);

CREATE INDEX idx_memories_category ON memories(category, importance DESC);
CREATE INDEX idx_memories_unsurfaced ON memories(surfaced, importance DESC)
  WHERE surfaced = 0 AND superseded_by IS NULL;

-- 输入路由不需要中间表：Agent 对话中理解意图后直接写入目标表
-- （tasks / resources / daily notes / knowledge notes）

-- 对话会话
CREATE TABLE sessions (
  id        TEXT PRIMARY KEY,
  initiator TEXT NOT NULL,  -- user | system | agent
  created   TEXT NOT NULL,
  updated   TEXT NOT NULL,
  summary   TEXT
);

CREATE INDEX idx_sessions_initiator ON sessions(initiator);

-- 对话消息（热存，90 天后转冷归档）
CREATE TABLE messages (
  id         TEXT PRIMARY KEY,
  session_id TEXT NOT NULL REFERENCES sessions(id),
  role       TEXT NOT NULL,  -- user | assistant
  content    TEXT NOT NULL,
  created    TEXT NOT NULL
);

-- 用户角色信息已归入 memories 表（category='profile'），不需要独立表
```

### Markdown 与 SQLite 同步

**Knowledge（知识笔记）— Markdown 是真相源：**

- **写入时**：Markdown 先写，然后更新 SQLite 索引（frontmatter tags/overview → notes 表 L0/L1）
- **写入失败恢复**：如果 SQLite 索引更新失败，写入 `data/.index_dirty` 脏标记文件，下次启动时立即触发全量重建
- **冲突时**：Markdown frontmatter 为权威，SQLite 索引可随时从文件系统重建
- **重建索引**：启动时先检查 `.index_dirty` 标记，再检查 `last_indexed` 与文件 mtime，仅增量更新

**Task（任务）— SQLite 是真相源：**

- **正向同步（SQLite → Markdown）**：任务创建/状态变更时，更新关联笔记中的 checkbox
  - 用户通过 UI 或对话创建/完成任务 → SQLite 更新 → 异步同步到 Markdown
  - Markdown 中渲染为：`- [ ] 买菜 <!--task:abc123-->`（pending）或 `- [x] 写周报 <!--task:def456-->`（done）

- **反向识别（Markdown → SQLite）**：解析 daily 时发现无 ID 的 checkbox（`- [ ] 新任务`），自动创建 task 记录并回写 `<!--task:id-->` 标记
  - 仅在新日记创建或主动触发"同步任务"时执行
  - 识别后任务纳入 SQLite 管理

- **冲突处理**：
  - 用户在 Obsidian 中手动勾选 checkbox：**不会**自动同步到 SQLite（避免文件监听复杂度）
  - 下次 App 启动或日记加载时，SQLite 中的任务状态会覆盖 Markdown 中的 checkbox 状态
  - 设计原因：保持单一真相源，避免双向同步的竞态条件

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
source: resource
created: 2026-03-15
updated: 2026-03-20
---

价值投资由本杰明·格雷厄姆创立……


## 安全边际

……完整内容……
```

**`source` 字段**——标识知识笔记的创建方式：

| 值 | 含义 | 说明 |
|----|------|------|
| NULL | 用户手动创建 | 直接在 vault 中编写 |
| `resource` | 从资源结晶 | URL/PDF 解析产出，原始资源详见 `resources` 表 |
| `session:ID` | 对话蒸馏 | SubAgent 从会话中提炼的洞见，非资源 |

**资源结晶流程**：

1. 用户提交 URL/PDF → 写入 `resources` 表（`status=pending`）
2. Agent 提取内容 → 更新 `status=parsing`
3. LLM 生成 tags（L0）+ overview（L1）
4. 提炼正文为结构化知识笔记（L2）→ 写入 `vault/knowledge/` + frontmatter
5. 更新 `resources.note_path` + `status=done` → 知识笔记进入"待确认"状态
6. 人类审核确认 → 知识笔记正式发布（可选：确认后更新 `status=confirmed`）

**注意**：`status=done` 表示 Agent 已完成结晶，但知识笔记需经人类确认后才正式发布。确认前笔记可标记为草稿状态（如 frontmatter 中 `draft: true`）。

- `tags` = **L0**（~100 tokens，从 frontmatter 提取，存入 SQLite + FTS5）
  - tags 是 Agent 的第一视角——扫描 tags 就能判断这篇笔记"关于什么"
  - tags 设计原则：覆盖核心概念 + 关联领域 + 关键人名/术语，总量控制在 ~100 tokens
- `overview` = **L1**（~2k tokens，从 frontmatter 提取，缓存到 SQLite `notes.overview`）
  - overview 是知识的结构化概要，Agent 读它即可理解核心内容，无需加载全文
  - 首次创建时由 SubAgent 从正文生成，写回 frontmatter 持久化
  - 人类可手动编辑 overview 提高精度（frontmatter 是真相源，SQLite 是缓存）
  - 笔记正文更新时，SubAgent 异步重新生成 overview 并写回 frontmatter
- 完整 Markdown 正文 = **L2**（仅在 Agent 明确需要时从文件系统读取）

**Markdown 文件即完整真相**：L0（tags）+ L1（overview）+ L2（正文）全部在一个 `.md` 文件中。SQLite 中的 `tags` 和 `overview` 列是 frontmatter 的派生缓存，丢失可从文件系统重建。

#### 目录级聚合索引

知识按主题组织为目录（如 `knowledge/投资/`、`knowledge/教育/`）。每个目录自动维护聚合索引：

- **目录记录创建时机**：索引重建时扫描 `vault/knowledge/` 文件系统目录结构，为每个存在的目录创建或更新 notes 表记录
- **目录记录判断**：`path LIKE '%.md'` 为笔记，否则为目录

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

目录和笔记统一在 `notes` 表中，通过 path 后缀区分（`.md` 为笔记，否则为目录）。L0 搜索只查一张表，检索路径统一。目录 L0（tags）在子笔记 CRUD 时自动聚合——合并去重子笔记的所有 tags。目录 L1 由 SubAgent 从子笔记 L1 聚合生成。

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
| **LLM 生成** | SubAgent 从正文生成结构化概要 | 写入 frontmatter `overview` 字段 | 默认策略，笔记创建/更新时异步触发 |
| **人工编写** | 用户直接编辑 frontmatter overview | frontmatter（真相源） | 高价值笔记需精确概要 |
| **截断兜底** | 取正文前 ~2k tokens | 仅缓存到 SQLite（不写 frontmatter） | LLM 调用失败时的降级策略 |

overview 的生命周期：

1. 笔记创建 → SubAgent 异步生成 overview → 写回 frontmatter → 同步到 SQLite 缓存
2. 人类编辑 frontmatter 中的 overview 后，下次索引时以 frontmatter 为准覆盖 SQLite
3. LLM 调用失败时，使用截断兜底策略缓存到 SQLite，frontmatter 中的 overview 保持为空或保留旧值

#### 索引更新触发

| 触发事件 | 更新内容 |
|---------|---------|
| 笔记创建/更新 | 提取 frontmatter tags → L0；提取 frontmatter overview → L1（无则 LLM 生成写回）；更新 FTS5；聚合 parent_dir 目录的 L0/L1 |
| 笔记删除 | 移除 notes/notes_fts 记录；重新聚合所属目录 |
| 新目录出现 | 自动插入目录记录（path 无 .md 后缀），聚合子笔记 tags → L0，SubAgent 生成 L1 |
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
默认模型配置               API Key（加密）              记忆（memories 表）
主题 / 语言                Gateway Bearer Token        使用统计
Vault 路径
同步配置
Token 预算（可选覆盖）
```

API Key 和 Gateway Bearer Token 必须存入 OS Keychain，绝不能存在任何明文文件中。

---
