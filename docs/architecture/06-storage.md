> **Status**: `active`

# Storage — 存储层

---

## § 职责定位

Storage 层负责三类存储介质（SQLite、Markdown 文件、OS Keychain）的读写操作，不负责任何业务逻辑判断、数据聚合或索引重建决策。

---

## § 核心原则

**真相源不可混淆**：

- **笔记**的真相源是 Markdown 文件（YAML Frontmatter），SQLite 只存储索引（允许过时，可重建）
- **会话历史**的真相源是 SQLite（全局 DB），不暴露在文件系统
- **可选任务**以 Markdown checklist 形式存在于 Daily Note 中，SQLite 仅建立派生索引
- 混淆两者的写入权会导致数据不一致无法恢复

**双 DB 架构**：

- **全局 DB**（`~/.config/mindclaw/mindclaw.db`）：存储跨 vault 的会话/回合/消息
- **Vault DB**（`{vault}/.mindclaw/mindclaw.db`）：存储当前 vault 的任务/笔记/记忆索引

---

## § 目录结构

```
~/.config/mindclaw/
├── config.json          ← UserConfig（providers、vault 列表）
└── mindclaw.db          ← 全局 DB：sessions/turns

{obsidian-vault}/
├── .obsidian/           ← Obsidian 配置（不动）
├── .mindclaw/
│   ├── config.json      ← VaultConfig（agent 偏好、folder 映射）
│   ├── mindclaw.db      ← Vault DB：checklist_index/notes/memories 索引
│   └── memory/          ← Memory Markdown 文件（Agent 内部数据）
└── daily/               ← 日记 Markdown 文件（含 checklist 任务）
```

---

## § 边界与实体

**输入**：来自 Services 层的读写请求，携带领域对象（Task、Note 内容、Memory 等）。

**输出**：存储操作结果（成功/失败）或检索结果（记录列表、文件内容），对上层屏蔽存储介质的差异。

**核心实体**：

**GlobalDatabase**：全局 SQLite 数据库，管理跨 vault 数据。

- 关键属性：数据库文件路径（`~/.config/mindclaw/mindclaw.db`）、WAL 模式
- 关系：由 AppRuntimeBuilder 初始化，以 `Arc<Mutex<Connection>>` 形式被 SessionManager 共享

**VaultDatabase**：Vault 级 SQLite 数据库，管理索引数据。

- 关键属性：数据库文件路径（`{vault}/.mindclaw/mindclaw.db`）、WAL 模式
- 关系：由 AppRuntimeBuilder 初始化，以 `Arc<Mutex<Connection>>` 形式被 Services 层共享

**MarkdownStorage**：Vault 目录的文件访问层，提供 Markdown 文件的读写接口。

- 关键属性：vault 根路径（`vault_path`，从 AppConfig 读取）
- 关系：TaskService/MemoryStore/NoteService 直接读写 Markdown 文件，同步更新 SQLite 索引

**KeychainStorage**：OS Keychain 的访问封装（不变）。

- 关键属性：服务名称（`mindclaw-{provider}-api-key`）
- 关系：Provider 初始化时读取 API Key

---

## § 存储职责分配

| 数据类型 | 存储位置 | 真相源 | 写入方 | 同步机制 |
|---------|---------|--------|-------|---------|
| 会话消息历史（Turn） | 全局 DB `sessions/turns` | SQLite | SessionManager | 单一写入方，无需同步 |
| Agent 记忆（Memory） | `{vault}/.mindclaw/memory/*.md` + Vault DB `memories_index` | **Markdown** | MemoryStore | 写文件后更新索引 |
| 可选任务（Task） | `{vault}/daily/*.md` 中的 checklist + Vault DB `checklist_index` | **Markdown** | Agent 工具 | 写文件后更新索引 |
| 笔记索引 | `{vault}/**/*.md` + Vault DB `notes_index` | **Markdown** | NoteService | 启动时 sync，运行时增量更新 |
| 日记 | `{vault}/daily/*.md` | Markdown | DailyService | 直接读写，无索引 |
| 私密笔记 | `{vault}/private/` | Markdown | 用户直接编辑 | Agent 不可访问 |
| API Key | OS Keychain | Keychain | 设置界面 | 独立存储 |

---

## § 可选：Checklist 索引

任务以 Markdown checklist（`- [ ] 内容`）形式存在于 Daily Note 中。

### Checklist 格式

```markdown
- [ ] 普通任务
- [ ] 优先级任务 !high
- [ ] 截止日任务 @2026-05-01
- [x] 已完成任务 ✅ 2026-04-28
```

### checklist_index 表结构

```sql
CREATE TABLE checklist_index (
    id INTEGER PRIMARY KEY,
    note_path TEXT NOT NULL,      -- 所属笔记路径
    line_number INTEGER,          -- 行号
    content TEXT NOT NULL,        -- 任务内容（去除标记）
    raw_line TEXT,                -- 原始行
    status TEXT,                  -- "todo" | "done"
    priority TEXT,                -- "high" | "medium" | "low"
    due_date TEXT,                -- YYYY-MM-DD
    completed_at TEXT,            -- ISO8601
    last_indexed TEXT
);
```

### 索引重建

扫描所有 `.md` 文件，正则匹配 `^- \[([ x])\] (.+)$`，提取内容并解析标签。

---

## § Frontmatter 格式规范（已废弃）

### Task Frontmatter

```yaml
---
id: "uuid"
title: "任务标题"
status: todo          # todo | in_progress | done | cancelled
priority: medium      # low | medium | high
due_date: 2026-04-10  # YYYY-MM-DD，可选
tags: [work, urgent]
created: 2026-04-07T14:30:00+08:00
updated: 2026-04-07T14:30:00+08:00
---

任务正文内容（Markdown 格式）
```

**文件名规则**：`{YYYY-MM-DD}-{slug}.md`，如 `2026-04-07-写季度报告.md`

### Memory Frontmatter

```yaml
---
id: "uuid"
key: "user-preference-theme"
category: preference  # user_fact | preference | work_context | relationship | goal
importance: 0.85
created: 2026-04-07T14:30:00+08:00
updated: 2026-04-07T14:30:00+08:00
---

记忆内容（Markdown 格式）
```

**文件名规则**：`{category}-{key-slug}.md`，如 `preference-theme.md`

---

## § 关键流程

### Checklist 索引更新

1. Agent 工具修改 Markdown 文件（追加 `- [ ] 内容` 或更新 `- [x]`）
2. 文件系统事件触发索引更新
3. 解析变更行，提取 content/status/priority/due_date
4. UPSERT 到 `checklist_index`

### 笔记索引同步（sync_index）

1. 遍历 vault 下所有 `.md` 文件（排除 `.obsidian/`, `.mindclaw/` 等）
2. 对比文件 `mtime` 与 `notes_index.modified_at`
3. 仅更新有变化的文件：提取 title/tags，计算 path hash
4. UPSERT 到 `notes_index`

### 笔记检索（RAG 简化版）

1. NoteService 在 `notes_index` 中对 title/tags 进行 LIKE 查询
2. 返回 `NoteIndex` 列表（轻量，不含正文）
3. 调用方按需调用 `read(file_path)` 获取完整内容

---

## § 设计决策与权衡

| 决策问题 | 选择 | 放弃的替代方案 | 理由 |
|---------|------|--------------|------|
| 知识笔记的真相源是哪里？ | **Markdown 文件**（`{vault}/`） | SQLite 为真相源，Markdown 为导出格式 | 文件对用户直接可见可编辑，SQLite 损坏后可从 Markdown 重建索引；Obsidian 可直接打开 |
| 向量嵌入存储在哪里？ | **不存储**（先不做向量嵌入） | SQLite BLOB / 独立向量数据库 | 个人应用规模小，LIKE 查询足够；向量增加复杂度 |
| 双 DB 如何划分？ | **全局 DB**（会话）+ **Vault DB**（索引） | 单 DB 存储所有数据 | Vault DB 随 vault 迁移，会话历史保留在本地；多 vault 场景下数据隔离清晰 |
| 配置格式是什么？ | **JSON**（`serde_json`） | TOML | JSON 与前端天然兼容，无需额外转换；serde_json 性能更好 |
| 配置层级如何设计？ | **两级配置**（UserConfig + VaultConfig → AppConfig） | 单配置 | 用户级配置跟随用户账号，Vault 级配置跟随 vault（可 git sync） |
| 会话历史是否永久保留在 SQLite？ | 永久保留（全局 DB） | 90 天后归档 | 全局 DB 只存会话，数据量可控；vault 级数据在 Markdown 中 |
| 私密内容如何与 Agent 隔离？ | PathGuard 在 Rust 层拒绝 `private/` 路径 | 文件系统权限 | Rust 层强制比文件系统权限更可靠 |
| SQLite 并发访问如何处理？ | WAL 模式 + `Arc<Mutex<Connection>>` | 连接池 | WAL 模式支持并发读；Mutex 序列化写，简单可靠 |
| 文件写入如何保证原子性？ | **temp + rename** 模式 | 直接写入 | rename 原子性由 OS 保证，崩溃后不会留下半写文件 |
| vault 与 SQLite 如何保持一致？ | **文件先写，后更新索引**；启动时支持重建 | 单事务覆盖两者 | SQLite 不支持文件系统事务；索引可重建使恢复简单 |
| 崩溃后如何恢复？ | 启动时扫描 vault，重建 SQLite 索引 | 依赖 SQLite 日志回滚 | 索引可重建使恢复简单可靠；Markdown 文件是真相源 |
| 如何处理存储空间不足？ | 返回错误，调用方处理 | 自动清理旧数据 | 自动清理可能导致数据丢失；返回错误让用户决定 |

---

## § 相关文件

| 文件 | 说明 |
|------|------|
| `src/storage/database/global.rs` | 全局 DB 打开与迁移 |
| `src/storage/database/vault.rs` | Vault DB 打开与迁移 |
| `src/storage/markdown.rs` | Frontmatter 解析与原子写入 |
| `src/storage/migrations/` | SQL 迁移文件 |
