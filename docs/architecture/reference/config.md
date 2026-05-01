> **Status**: `active`
>
> 本文档描述 MindClaw 的配置边界、配置文件职责和不可配置策略。当前代码仍可能保留旧字段；本文档以目标设计为准，代码迁移另行处理。

# 配置说明

MindClaw 的配置设计遵循一个原则：**用户真正需要维护的设置应尽量随 Vault 打包带走；只与当前设备、密钥和最近打开状态相关的内容才留在用户级配置；产品运行规则进入代码策略，不暴露为用户配置项。**

因此配置分为三类：

| 配置层 | 存放位置 | 是否随 Vault 迁移 | 职责 |
|--------|----------|-------------------|------|
| UserConfig | `~/.config/mindclaw/config.json` | 否 | 当前设备上的 Vault 列表、最近打开状态、界面显示偏好和 Provider 连接元数据 |
| VaultConfig | `{vault}/.mindclaw/config.json` | 是 | 当前 Vault 的工作区偏好、默认模型选择、目录映射和索引刷新偏好 |
| CodePolicy | 代码常量 / 策略模块 | 否 | Core Docs 读取规则、置信度默认策略、Private 隔离、索引硬排除和安全边界 |

`AppConfig` 是运行时合并结果，不单独持久化。合并顺序为：代码默认值 → UserConfig → VaultConfig → 本次运行临时参数。

---

## 设计原则

1. **Vault 优先**：影响知识库体验、Agent 行为和工作台布局的配置默认写入 Vault，使 Vault 迁移到另一台设备后仍保持一致。
2. **用户级最小化**：UserConfig 只保存当前设备必须知道的信息，例如最近打开哪个 Vault、已登记哪些 Vault、界面语言和主题。
3. **密钥不进配置**：API Key 和敏感凭据只存 OS Keychain / Stronghold，配置文件最多保存 Provider 的非敏感连接元数据。
4. **策略不暴露成设置**：Core Docs 文件名、注入顺序、`confidence` 默认值、Private 硬隔离、ContextIndex 硬排除属于产品策略，不作为用户可编辑配置。
5. **文档是真相源**：知识、记忆、Inbox、演化记录、置信度和状态写在 Markdown + Frontmatter 中；配置不保存长期知识事实。

---

## UserConfig

UserConfig 是当前设备的本地启动配置，不代表 Vault 的产品设置。它可以被重建或重新登记，不应影响 Vault 的可迁移性。

```json
{
  "language": "zh-CN",
  "theme": "system",
  "active_vault_path": null,
  "vaults": [],
  "providers": [],
  "active_provider_id": null,
  "startup": {
    "open_last_vault": true
  }
}
```

| 名称 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `language` | String | `"zh-CN"` | 当前设备界面语言 |
| `theme` | String | `"system"` | 当前设备主题 |
| `active_vault_path` | Option<PathBuf> | `None` | 最近打开的 Vault 路径 |
| `vaults` | Vec<VaultEntry> | `[]` | 当前设备登记过的 Vault 列表 |
| `providers` | Vec<ProviderConfig> | 内置默认值 | Provider 非敏感连接元数据 |
| `active_provider_id` | Option<String> | `None` | 当前设备默认 Provider；VaultConfig 可覆盖 |
| `startup.open_last_vault` | bool | `true` | 启动时是否尝试打开上次 Vault |

### VaultEntry

| 名称 | 类型 | 说明 |
|------|------|------|
| `path` | PathBuf | Vault 本地路径 |
| `display_name` | String | 当前设备显示名称，可从 Vault metadata 推导 |
| `last_opened_at` | DateTime | 最近打开时间，用于启动页排序 |

### ProviderConfig

| 名称 | 类型 | 说明 |
|------|------|------|
| `id` | String | Provider 标识 |
| `display_name` | String | 显示名称 |
| `base_url` | String | API 基础 URL |
| `main_model` | Option<String> | 当前 provider 主模型；为空时使用 `default_model` |
| `light_model` | Option<String> | 当前 provider 轻量模型；为空时运行时回退主模型 |
| `default_model` | Option<String> | 兼容字段，作为主模型默认值 |
| `available_models` | Vec<String> | 可选模型列表 |

API Key 不写入 UserConfig，使用 OS Keychain / Stronghold，并通过 `provider_id` 关联。

### 不再放入 UserConfig 的内容

| 内容 | 归属 | 理由 |
|------|------|------|
| 工作区布局、面板宽度、最近打开 Tab | VaultConfig | 这些偏好属于具体 Vault 的工作方式，应随 Vault 迁移 |
| Agent 默认迭代次数、上下文预算、并发参数 | CodePolicy / AgentProfile | 属于运行策略，不应成为普通用户设置 |
| MessageBus 容量 | CodePolicy | 运行时基础设施参数，不是用户心智模型 |
| 记忆开关和提取阈值 | CodePolicy + Vault Core Docs + Markdown 状态 | 记忆是否可用由产品能力决定，具体记忆可信度由 Markdown 管理 |

---

## VaultConfig

VaultConfig 是可随 Vault 打包迁移的配置，存储当前知识库的工作偏好和轻量目录约定。

```json
{
  "workspace": {
    "left_panel_width": 280,
    "right_panel_width": 360,
    "last_ribbon": "daily",
    "open_tabs": [],
    "pinned_paths": []
  },
  "agent": {
    "provider_id": null,
    "model_id": null,
    "light_model_id": null
  },
  "folder_mappings": {
    "daily": "daily",
    "inbox": "inbox",
    "resources": "resources",
    "agent_memory": "agent/memory",
    "agent_evolution": "agent/evolution",
    "private": "private"
  },
  "daily": {
    "date_format": "YYYY-MM-DD",
    "template_path": null
  },
  "index": {
    "auto_sync_on_open": true,
    "auto_sync_interval_secs": 300
  }
}
```

### WorkspacePrefs

| 名称 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `left_panel_width` | usize | `280` | 左侧面板宽度 |
| `right_panel_width` | usize | `360` | 右侧上下文面板宽度 |
| `last_ribbon` | String | `"daily"` | 上次使用的 Ribbon 工作域 |
| `open_tabs` | Vec<TabRef> | `[]` | 可恢复的中央内容区 Tab |
| `pinned_paths` | Vec<String> | `[]` | 当前 Vault 内固定的常用路径 |

WorkspacePrefs 属于 VaultConfig，而不是 UserConfig。原因是同一个用户在不同 Vault 中会形成不同的工作区布局、固定路径和打开习惯。

### VaultAgentConfig

| 名称 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `provider_id` | Option<String> | `None` | 当前 Vault 首选 Provider；为空时使用 UserConfig 默认值 |
| `model_id` | Option<String> | `None` | 当前 Vault 主模型；为空时使用 Provider 主模型或默认模型 |
| `light_model_id` | Option<String> | `None` | 当前 Vault 轻量模型；为空时运行时回退主模型 |

VaultConfig 不保存 `system_prompt`。稳定提示词由 `{vault}/.mindclaw/AGENTS.md`、`SOUL.md`、`TOOLS.md`、`USER.md`、`MEMORY.md` 承载。

### FolderMappings

| 名称 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `daily` | String | `"daily"` | Daily 目录 |
| `inbox` | String | `"inbox"` | Intake & Review Queue 目录 |
| `resources` | String | `"resources"` | 外部原始资源目录 |
| `agent_memory` | String | `"agent/memory"` | 已确认 Agent 记忆目录 |
| `agent_evolution` | String | `"agent/evolution"` | Agent 演化记录目录 |
| `private` | String | `"private"` | Private 文件夹路径 |

目录映射用于 Vault 初始化、迁移和 UI 定位。即使 `private` 路径可配置，它仍然受 CodePolicy 的硬隔离规则约束，不进入 Agent 召回和 ContextIndex。

### DailyConfig

| 名称 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `date_format` | String | `"YYYY-MM-DD"` | Daily 文件名日期格式 |
| `template_path` | Option<String> | `None` | Daily 模板文件路径，指向 Vault 内 Markdown |

### IndexConfig

| 名称 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `auto_sync_on_open` | bool | `true` | 打开 Vault 时自动同步索引 |
| `auto_sync_interval_secs` | u64 | `300` | 后台增量同步间隔，`0` 表示禁用定时同步 |

---

## CodePolicy

CodePolicy 表示产品固定策略或工程默认值，不写入用户可编辑配置文件。只有代码版本升级、产品决策调整或开发者构建配置改变时才修改。

### CoreDocsPolicy

Vault Core Docs 固定存储在 `{vault}/.mindclaw/`，由 ContextPipeline 作为 Core Layer 读取，不进入 `context_index`，也不作为 `agent/memory/` 的结构化记忆资产。

| 固定文件 | 标题 | 用途 |
|----------|------|------|
| `AGENTS.md` | Your Workspace | 当前 Vault 的工作规范、知识库约定和协作规则 |
| `SOUL.md` | Who You Are | Agent 稳定性格与表达边界 |
| `TOOLS.md` | Tool Usage Notes | 工具使用指引，具体工具细节按需搜索 |
| `USER.md` | About Your Human | 用户摘要，建议控制在 500 字以内 |
| `MEMORY.md` | Long-term Memory | 长期关键记忆摘要，建议控制在 2000 字以内 |

固定注入顺序：

```text
AGENTS.md -> SOUL.md -> TOOLS.md -> USER.md -> MEMORY.md
```

这些文件的路径、顺序和建议预算不是用户配置项。用户编辑的是文件内容本身。

### ConfidencePolicy

`confidence` 是 Markdown Frontmatter 字段，不是配置项。用户在创建、修改和审核文件时直接维护每个文件的置信度；ContextIndex 从 Frontmatter 派生该值用于召回排序。

代码策略只定义默认值和阈值：

| 场景 | 默认策略 |
|------|----------|
| 用户创建内容 | 默认中高置信度 |
| 外部剪藏或解析产物 | 默认中等置信度，等待审核 |
| Agent 生成候选 | 默认较低置信度，必须经过审核 |
| 用户审核确认 | 提升到高置信度 |
| 普通召回 | 低置信内容默认降低排序或不主动注入 |

默认数值属于代码策略，不在 `{vault}/.mindclaw/config.json` 中暴露。需要审计和迁移的是每个 Markdown 文件自己的 `confidence`。

### IndexPolicy

以下索引规则固定在代码策略中：

- `.mindclaw/`、`.obsidian/`、缓存目录和系统临时目录不进入普通知识索引。
- `private/` 不进入 ContextIndex，不进入 Agent ContextPipeline。
- `resources/` 只索引 manifest 和可检索元数据，不把原始资源正文当作 Markdown 知识。
- `inbox/` 是独立 `space`，待处理项默认不等同于已确认知识。

### RuntimePolicy

以下运行参数不暴露给普通用户配置：

- Agent 最大迭代次数、工具并发、LLM 并发和上下文预算。
- MessageBus 容量、后台任务锁超时和数据库连接池参数。
- Memory 提取阈值、反思触发阈值和默认召回阈值。

这些参数影响系统稳定性和成本，应由代码默认值、开发者构建配置或未来的高级实验开关管理。

---

## 配置合并

运行时读取配置的顺序：

1. 加载代码默认值和 CodePolicy。
2. 读取 UserConfig，获得当前设备语言、主题、Vault 列表、Provider 元数据和最近打开路径。
3. 打开 Vault 后读取 VaultConfig，覆盖与当前 Vault 相关的工作区、模型和目录偏好。
4. 读取 Vault Core Docs，作为 ContextPipeline 的 Core Layer 内容。
5. 根据本次 Session 的显式参数做临时覆盖；临时覆盖不写回配置文件。

冲突处理规则：

| 冲突 | 处理方式 |
|------|----------|
| VaultConfig 指定的 Provider 在当前设备不存在 | UI 提示用户绑定本机 Provider，不自动改写 VaultConfig |
| VaultConfig 指定目录不存在 | 创建目录或提示用户修复；不静默改到其他路径 |
| Core Docs 缺失 | 使用空内容并提示初始化；不写入配置替代 |
| Markdown Frontmatter 缺少 `confidence` | 使用 ConfidencePolicy 默认值建立索引，但不强制改写原文件 |

---

## 不属于配置的内容

| 内容 | 存放位置 | 原因 |
|------|----------|------|
| 知识正文、Daily、Inbox、Agent Memory、EvolutionLog | Markdown + Frontmatter | 可审阅、可迁移、可纠偏 |
| `tags`、`overview`、`confidence`、`origin`、`status`、`refs` | Markdown Frontmatter | 属于内容索引语义，不属于全局配置 |
| Agent 会话完整记录 | Global DB `sessions` / `turns` | 用于活跃会话恢复和工具记录审计 |
| ContextIndex / FTS / ChecklistIndex | Vault DB | 可从 Markdown 重建，是索引不是配置 |
| API Key | OS Keychain / Stronghold | 敏感信息不能写入明文配置 |

---

## 当前代码状态

当前实现中的 `src-tauri/src/runtime/config/*` 仍可能包含旧字段，例如 `agent_defaults`、`task_defaults`、`workspace` 位于 UserConfig、或 `memory` 阈值位于 VaultConfig。本轮文档定义的是目标配置边界；代码迁移需要单独计划。

## 相关文件

| 文件 | 说明 |
|------|------|
| `src-tauri/src/runtime/config/mod.rs` | AppConfig 定义 |
| `src-tauri/src/runtime/config/user.rs` | UserConfig、ProviderConfig、VaultEntry |
| `src-tauri/src/runtime/config/vault.rs` | VaultConfig、FolderMappings |
| `src-tauri/src/runtime/config/loader.rs` | ConfigLoader |
| `docs/architecture/03.06-context-pipeline.md` | ContextPipeline 与 Vault Core Docs 注入顺序 |
| `docs/architecture/06-storage.md` | Vault 目录结构、Frontmatter 和索引职责 |
