> **Status**: `active`

# Storage — 存储层

---

## § 职责定位

Storage 层负责 SQLite、Markdown 文件和 OS Keychain 的读写封装；不负责任何业务判断、候选审核、记忆召回排序或知识提炼。

---

## § 核心原则

**真相源不可混淆**：

- 已确认知识正文的真相源是 Markdown 文件和 YAML Frontmatter。
- Checklist 任务的真相源是 Markdown checklist。
- Agent 记忆、观察候选、审核状态和演化记录的真相源是 SQLite。
- API Key 的真相源是 OS Keychain。

**索引可重建，运行状态不可重建**：`notes_index`、`checklist_index` 可从 Markdown 重建；Memory、Review、Evolution 表不可从 Markdown 完整重建。

**Private 后端强隔离**：任何 Storage 读取、索引、召回和写入入口都必须拒绝 Agent 访问 `private/` 路径。

---

## § 目录结构

```text
~/.config/mindclaw/
├── config.json          ← UserConfig（providers、vault 列表）
└── mindclaw.db          ← 全局 DB：sessions / turns

{vault}/
├── .obsidian/           ← Obsidian 配置（不动）
├── .mindclaw/
│   ├── config.json      ← VaultConfig（agent 偏好、folder 映射）
│   └── mindclaw.db      ← Vault DB：索引、记忆、候选、演化记录
├── daily/               ← Daily Markdown 文件
├── private/             ← Agent 不可见内容
└── **/*.md              ← 共有知识、项目笔记、Inbox 等 Markdown 内容
```

---

## § 存储介质

**GlobalDatabase**：全局 SQLite 数据库，管理跨 vault 的会话和 turn。

**VaultDatabase**：Vault 级 SQLite 数据库，管理 notes/checklist 索引、Agent 记忆、审核候选和演化记录。

**MarkdownStorage**：Vault 目录的文件访问层，提供 Markdown 文件读写、Frontmatter 解析和原子写入。

**KeychainStorage**：OS Keychain 封装，保存 Provider API Key 等敏感信息。

---

## § 存储职责分配

| 数据类型 | 存储位置 | 真相源 | 写入方 | 可重建 |
|---------|---------|--------|--------|--------|
| 会话消息历史 | 全局 DB `sessions` / `turns` | SQLite | SessionManager | 否 |
| 知识笔记正文 | Vault Markdown | Markdown | NoteService / 用户编辑 | 是 |
| 笔记索引 | Vault DB `notes_index` | Markdown 派生 | NoteService | 是 |
| Daily | `daily/*.md` | Markdown | DailyService / 用户编辑 | 是 |
| Checklist | Markdown checklist + `checklist_index` | Markdown | ChecklistService | 是 |
| Agent 记忆 | Vault DB `memories` | SQLite | MemoryService | 否 |
| 记忆来源 | Vault DB `memory_sources` | SQLite | MemoryService | 否 |
| 知识引用 | Vault DB `memory_knowledge_refs` | SQLite + Markdown 路径 | MemoryService | 可部分校验 |
| 回顾队列 | Vault DB `review_items` | SQLite | ReviewService | 否 |
| 观察候选 | Vault DB `observation_candidates` | SQLite | ReviewService | 否 |
| 记忆更新建议 | Vault DB `memory_update_proposals` | SQLite | ReviewService | 否 |
| 经验教训候选 | Vault DB `lesson_candidates` | SQLite | ReviewService | 否 |
| 演化记录 | Vault DB `evolution_logs` | SQLite | EvolutionService | 否 |
| API Key | OS Keychain | Keychain | Settings / Provider | 否 |

---

## § Frontmatter 知识索引

知识笔记使用 Markdown Frontmatter 作为人类和 Agent 共用的轻量索引。

```yaml
---
title: 笔记标题
tags: [agent-memory, knowledge-design]
overview: 一句话到一小段，说明这篇笔记解决什么问题、核心判断是什么。
---
```

字段规则：

- `title` 用于人类浏览和搜索结果展示。
- `tags` 用于轻量路由和过滤。
- `overview` 用于 Agent 预读，判断是否需要加载正文。
- 正文承载完整论证、案例、方法和反例。

---

## § Checklist 索引

任务以 Markdown checklist 表达，不作为独立一等业务对象。

```markdown
- [ ] 普通任务
- [ ] 优先级任务 !high
- [ ] 截止日任务 @2026-05-01
- [x] 已完成任务 ✅ 2026-04-28
```

`checklist_index` 是派生索引，用于快速筛选和 UI 展示。启动或文件变化时可从 Markdown 重建。

---

## § 关键流程

### 笔记保存

1. NoteService 构造 Markdown 正文和 Frontmatter。
2. MarkdownStorage 使用 temp + rename 原子写入。
3. NoteService 更新 `notes_index`。
4. 若来源为经验教训候选，ReviewService 和 MemoryService 更新候选状态与知识引用。

### 记忆更新

1. ReviewService 或用户操作请求 MemoryService 更新记忆。
2. MemoryService 写入 `memories`、`memory_sources` 和可选 `memory_knowledge_refs`。
3. EvolutionService 写入 `evolution_logs`。
4. ContextPipeline 后续召回读取 MemoryService 输出，不直接读表。

### 索引重建

1. 扫描 Vault 下允许索引的 Markdown 文件。
2. 排除 `.obsidian/`、`.mindclaw/`、`private/` 等目录。
3. 提取 Frontmatter 和 checklist。
4. 重建 `notes_index` 与 `checklist_index`。

---

## § 设计决策与权衡

| 决策问题 | 选择 | 放弃的替代方案 | 理由 |
|---------|------|--------------|------|
| 知识笔记的真相源是哪里？ | Markdown 文件 | SQLite 为真相源，Markdown 为导出格式 | 文件对用户直接可见可编辑，损坏后可重建索引 |
| Agent 记忆的真相源是哪里？ | SQLite | Markdown memory 文件 | 记忆是结构化运行状态，需要审核、降权、来源和删除语义 |
| 观察候选和经验候选是否写 Markdown？ | 否，SQLite 保存审核状态 | 每个候选生成 Markdown 文件 | 候选不是确认知识，写 Markdown 会污染知识空间 |
| 向量嵌入存储在哪里？ | 暂不作为必需存储层 | SQLite BLOB / 独立向量库 | MVP 先依赖 Frontmatter、LIKE 和按需正文加载 |
| 双 DB 如何划分？ | 全局 DB 存会话，Vault DB 存 vault 相关索引和运行状态 | 单 DB 存所有数据 | Vault 数据随 vault 迁移，全局会话保持本地 |
| 私密内容如何与 Agent 隔离？ | PathGuard 在 Rust 层拒绝 `private/` 路径 | 文件系统权限或前端隐藏 | Agent 不可见边界必须由后端强制 |
| 文件写入如何保证原子性？ | temp + rename | 直接写入 | rename 原子性由 OS 保证，崩溃后避免半写文件 |

---

## § 相关文件

| 文件 | 说明 |
|------|------|
| `src-tauri/src/storage/database/global.rs` | 全局 DB 打开与迁移 |
| `src-tauri/src/storage/database/vault.rs` | Vault DB 打开与迁移 |
| `src-tauri/src/storage/markdown.rs` | Frontmatter 解析与原子写入 |
| `src-tauri/src/storage/migrations/` | SQL 迁移文件 |
