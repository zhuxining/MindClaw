> **Status**: `active`
>
> 本文档列出所有配置项的名称、类型、默认值和说明。随配置字段增减同步更新。

# 配置说明

MindClaw 使用**两级配置**系统：

- **UserConfig**：用户级配置，存储在 `~/.config/mindclaw/config.json`，跟随用户账号
- **VaultConfig**：Vault 级配置，存储在 `{vault}/.mindclaw/config.json`，跟随 Obsidian vault（可 git sync）
- **AppConfig**：运行时合并后的配置，由 `ConfigLoader::merge()` 生成

---

## 加载顺序

配置按以下优先级加载（后者覆盖前者）：

1. **硬编码默认值**（代码中 `Default` 实现）
2. **UserConfig**（`~/.config/mindclaw/config.json`，若存在）
3. **VaultConfig**（`{vault}/.mindclaw/config.json`，若存在）
4. **命令行覆盖**（`AppRuntimeBuilder` 的覆盖参数）

**合并规则**：VaultConfig 中 `Some` 值的字段覆盖 UserConfig 的对应值；`None` 表示继承全局配置。

---

## UserConfig 配置项

文件位置：`~/.config/mindclaw/config.json`

```json
{
  "language": "zh-CN",
  "theme": "system",
  "active_vault_path": "/Users/foo/Documents/Obsidian/MyVault",
  "vaults": [
    {
      "name": "我的知识库",
      "path": "/Users/foo/Documents/Obsidian/MyVault",
      "last_opened": "2026-04-07T14:30:00+08:00"
    }
  ],
  "active_provider_id": "deepseek",
  "providers": [
    {
      "id": "deepseek",
      "display_name": "DeepSeek",
      "base_url": "https://api.deepseek.com",
      "default_model": "deepseek-chat",
      "available_models": ["deepseek-chat", "deepseek-reasoner"]
    }
  ],
  "agent_defaults": {
    "max_iterations": 8,
    "temperature": null,
    "max_tokens": null,
    "context_token_limit": 128000,
    "tool_concurrency": 4,
    "llm_concurrency": 3,
    "enable_memory": true
  },
  "bus_capacity": 100,
  "startup": {
    "open_last_vault": true,
    "sync_index_on_open": true
  }
}
```

### 应用界面配置

| 名称 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `language` | String | `"zh-CN"` | 界面语言：`zh-CN` 或 `en-US` |
| `theme` | String | `"system"` | 主题：`light`、`dark`、`system` |

### Vault 管理配置

| 名称 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `active_vault_path` | Option<PathBuf> | `None` | 当前打开的 vault 路径 |
| `vaults` | Vec<VaultEntry> | `[]` | Vault 列表（最近打开过的记录） |

**VaultEntry 结构**：

| 名称 | 类型 | 说明 |
|------|------|------|
| `name` | String | 显示名称 |
| `path` | PathBuf | Vault 根目录绝对路径（作为唯一标识） |
| `last_opened` | Option<String> | 最后打开时间（ISO 8601） |

### Provider 配置

| 名称 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `active_provider_id` | String | `"deepseek"` | 默认 LLM Provider 标识 |
| `providers` | Vec<ProviderConfig> | 内置配置 | Provider 列表 |

**ProviderConfig 结构**：

| 名称 | 类型 | 说明 |
|------|------|------|
| `id` | String | Provider 标识：`deepseek`、`openai`、`claude` |
| `display_name` | String | 显示名称 |
| `base_url` | String | API 基础 URL |
| `default_model` | Option<String> | 默认模型 ID |
| `available_models` | Vec<String> | 可用模型列表 |

**API Key 存储**：不存于配置文件，使用 OS Keychain，Key 名为 `mindclaw-{id}-api-key`。

### Agent 默认配置

| 名称 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `agent_defaults.max_iterations` | usize | `8` | 最大迭代次数 |
| `agent_defaults.temperature` | Option<f32> | `None` | 采样温度，`None` 使用 Provider 默认值 |
| `agent_defaults.max_tokens` | Option<usize> | `None` | 最大生成 token 数 |
| `agent_defaults.context_token_limit` | usize | `128000` | 上下文窗口上限 |
| `agent_defaults.tool_concurrency` | usize | `4` | 工具并发数 |
| `agent_defaults.llm_concurrency` | usize | `3` | LLM 调用并发数 |
| `agent_defaults.enable_memory` | bool | `true` | 是否启用记忆提取 |

### 运行时配置

| 名称 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `bus_capacity` | usize | `100` | MessageBus 有界队列容量 |
| `startup.open_last_vault` | bool | `true` | 启动时自动打开上次 vault |
| `startup.sync_index_on_open` | bool | `true` | 打开 vault 时自动增量同步索引 |

---

## VaultConfig 配置项

文件位置：`{vault}/.mindclaw/config.json`

```json
{
  "agent": {
    "provider_id": null,
    "model_id": null,
    "system_prompt": null,
    "max_iterations": null,
    "temperature": null,
    "max_tokens": null,
    "enable_memory": true,
    "memory": {
      "auto_extract": true,
      "min_importance": 0.5,
      "max_entries": 500
    }
  },
  "folder_mappings": {
    "tasks": "tasks",
    "memory": "memory",
    "daily": "daily",
    "index_exclude": [".obsidian", ".mindclaw", "templates", "attachments"]
  },
  "task_defaults": {
    "priority": "medium",
    "tags": []
  },
  "daily": {
    "date_format": "YYYY-MM-DD",
    "template": null
  },
  "index": {
    "auto_sync_interval_secs": 300
  }
}
```

### Agent 配置（覆盖层）

| 名称 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `agent.provider_id` | Option<String> | `None` | 覆盖全局 Provider，`None` 表示继承 |
| `agent.model_id` | Option<String> | `None` | 覆盖全局模型 |
| `agent.system_prompt` | Option<String> | `None` | 自定义系统提示词 |
| `agent.max_iterations` | Option<usize> | `None` | 覆盖全局最大迭代次数 |
| `agent.temperature` | Option<f32> | `None` | 覆盖全局温度 |
| `agent.max_tokens` | Option<usize> | `None` | 覆盖全局最大 token 数 |
| `agent.enable_memory` | bool | `true` | 是否为此 vault 开启记忆 |
| `agent.memory.auto_extract` | bool | `true` | Turn 结束后自动提取记忆 |
| `agent.memory.min_importance` | f32 | `0.5` | 低于此值不写入记忆 |
| `agent.memory.max_entries` | usize | `500` | 超出时按 importance 淘汰 |

### 文件夹映射

| 名称 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `folder_mappings.tasks` | String | `"tasks"` | 任务文件夹（相对 vault 根） |
| `folder_mappings.memory` | String | `"memory"` | 记忆文件夹 |
| `folder_mappings.daily` | String | `"daily"` | 日记文件夹 |
| `folder_mappings.index_exclude` | Vec<String> | 见示例 | 索引扫描排除目录 |

### 任务默认值

| 名称 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `task_defaults.priority` | String | `"medium"` | 新建任务默认优先级 |
| `task_defaults.tags` | Vec<String> | `[]` | 新建任务自动添加的标签 |

### 日记配置

| 名称 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `daily.date_format` | String | `"YYYY-MM-DD"` | 日记文件名格式 |
| `daily.template` | Option<String> | `None` | 日记模板内容，`None` 表示无模板 |

### 索引配置

| 名称 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `index.auto_sync_interval_secs` | u64 | `300` | 后台增量同步间隔（秒），`0` 表示禁用 |

---

## AppConfig 运行时配置

AppConfig 由 `ConfigLoader::merge()` 生成，包含合并后的最终配置值和运行时路径。

### 路径字段

| 名称 | 类型 | 说明 |
|------|------|------|
| `user_data_dir` | PathBuf | `~/.config/mindclaw/` |
| `global_db_path` | PathBuf | `~/.config/mindclaw/mindclaw.db` |
| `vault_path` | PathBuf | 当前 vault 根目录 |
| `vault_config_dir` | PathBuf | `{vault}/.mindclaw/` |
| `vault_db_path` | PathBuf | `{vault}/.mindclaw/mindclaw.db` |

### 运行时参数字段（合并后）

| 名称 | 类型 | 来源 |
|------|------|------|
| `provider_id` | String | VaultConfig.agent.provider_id → UserConfig.active_provider_id |
| `model_id` | Option<String> | VaultConfig.agent.model_id → UserConfig.agent_defaults |
| `system_prompt` | String | VaultConfig.agent.system_prompt → 内置默认值 |
| `agent_max_iterations` | usize | VaultConfig.agent.max_iterations → UserConfig.agent_defaults |
| `agent_temperature` | Option<f32> | VaultConfig.agent.temperature → UserConfig.agent_defaults |
| `agent_max_tokens` | Option<usize> | VaultConfig.agent.max_tokens → UserConfig.agent_defaults |
| `enable_memory` | bool | VaultConfig.agent.enable_memory |
| `memory` | MemoryConfig | VaultConfig.agent.memory |
| `folder_mappings` | FolderMappings | VaultConfig.folder_mappings |
| `task_defaults` | TaskDefaults | VaultConfig.task_defaults |
| `daily` | DailyConfig | VaultConfig.daily |
| `index` | IndexConfig | VaultConfig.index |
| `startup` | StartupConfig | UserConfig.startup |
| `bus_capacity` | usize | UserConfig.bus_capacity |

---

## 向前兼容

所有配置结构均使用 `#[serde(default)]`，缺失字段自动使用默认值，不会导致旧配置文件解析失败。

---

## 相关文件

| 文件 | 说明 |
|------|------|
| `src/runtime/config/mod.rs` | AppConfig 定义 |
| `src/runtime/config/user.rs` | UserConfig、ProviderConfig、VaultEntry |
| `src/runtime/config/vault.rs` | VaultConfig、FolderMappings、MemoryConfig |
| `src/runtime/config/loader.rs` | ConfigLoader（加载/保存/合并） |
