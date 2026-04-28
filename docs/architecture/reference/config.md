> **Status**: `active`
>
> 本文档列出当前代码中的配置项名称、类型、默认值和说明。随配置字段增减同步更新。

# 配置说明

MindClaw 使用两级配置系统：

- **UserConfig**：用户级配置，存储在 `~/.config/mindclaw/config.json`。
- **VaultConfig**：Vault 级配置，存储在 `{vault}/.mindclaw/config.json`。
- **AppConfig**：运行时合并后的配置，由 `ConfigLoader::merge()` 生成。

---

## UserConfig 配置项

```json
{
  "language": "zh-CN",
  "theme": "system",
  "active_vault_path": null,
  "vaults": [],
  "active_provider_id": "deepseek",
  "providers": [],
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
  },
  "workspace": {}
}
```

| 名称 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `language` | String | `"zh-CN"` | 界面语言 |
| `theme` | String | `"system"` | 主题 |
| `active_vault_path` | Option<PathBuf> | `None` | 当前打开的 vault 路径 |
| `vaults` | Vec<VaultEntry> | `[]` | Vault 列表 |
| `active_provider_id` | String | `"deepseek"` | 默认 LLM Provider 标识 |
| `providers` | Vec<ProviderConfig> | 内置配置 | Provider 列表 |
| `agent_defaults` | GlobalAgentDefaults | 见下表 | 全局 Agent 默认参数 |
| `bus_capacity` | usize | `100` | MessageBus 有界队列容量 |
| `startup` | StartupConfig | 见下表 | 启动行为配置 |
| `workspace` | WorkspacePrefs | `Default` | 桌面工作区偏好 |

### GlobalAgentDefaults

| 名称 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `max_iterations` | usize | `8` | 最大迭代次数 |
| `temperature` | Option<f32> | `None` | 采样温度 |
| `max_tokens` | Option<usize> | `None` | 最大生成 token 数 |
| `context_token_limit` | usize | `128000` | 上下文窗口上限 |
| `tool_concurrency` | usize | `4` | 工具并发数 |
| `llm_concurrency` | usize | `3` | LLM 调用并发数 |
| `enable_memory` | bool | `true` | 是否启用当前实现中的记忆提取与召回 |

### ProviderConfig

| 名称 | 类型 | 说明 |
|------|------|------|
| `id` | String | Provider 标识 |
| `display_name` | String | 显示名称 |
| `base_url` | String | API 基础 URL |
| `default_model` | Option<String> | 默认模型 ID |
| `available_models` | Vec<String> | 可用模型列表 |

API Key 不存于配置文件，使用 OS Keychain。

### StartupConfig

| 名称 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `open_last_vault` | bool | `true` | 启动时自动打开上次 vault |
| `sync_index_on_open` | bool | `true` | 打开 vault 时自动增量同步索引 |

---

## VaultConfig 配置项

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
    "memory": ".mindclaw/memory",
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

### VaultAgentConfig

| 名称 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `provider_id` | Option<String> | `None` | 覆盖全局 Provider |
| `model_id` | Option<String> | `None` | 覆盖全局模型 |
| `system_prompt` | Option<String> | `None` | 自定义系统提示词 |
| `max_iterations` | Option<usize> | `None` | 覆盖全局最大迭代次数 |
| `temperature` | Option<f32> | `None` | 覆盖全局温度 |
| `max_tokens` | Option<usize> | `None` | 覆盖全局最大 token 数 |
| `enable_memory` | bool | `true` | 是否为此 vault 开启当前实现中的记忆提取 |
| `memory` | MemoryConfig | 见下表 | 记忆提取设置 |

### MemoryConfig

| 名称 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `auto_extract` | bool | `true` | Turn 结束后自动后台提取 |
| `min_importance` | f32 | `0.5` | 低于此重要性的记忆不写入 |
| `max_entries` | usize | `500` | 超出时按 importance 淘汰旧条目 |

### FolderMappings

| 名称 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `tasks` | String | `"tasks"` | 任务文件夹 |
| `memory` | String | `".mindclaw/memory"` | 当前实现中的记忆文件夹 |
| `daily` | String | `"daily"` | 日记文件夹 |
| `index_exclude` | Vec<String> | 见示例 | 索引扫描排除目录 |

### TaskDefaults

| 名称 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `priority` | TaskPriority | `Medium` | 新建任务默认优先级 |
| `tags` | Vec<String> | `[]` | 新建任务自动添加标签 |

### DailyConfig

| 名称 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `date_format` | String | `"YYYY-MM-DD"` | 日记文件名格式 |
| `template` | Option<String> | `None` | 日记模板内容 |

### IndexConfig

| 名称 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `auto_sync_interval_secs` | u64 | `300` | 后台增量 sync 间隔，`0` 表示禁用 |

---

## 相关文件

| 文件 | 说明 |
|------|------|
| `src-tauri/src/runtime/config/mod.rs` | AppConfig 定义 |
| `src-tauri/src/runtime/config/user.rs` | UserConfig、ProviderConfig、VaultEntry |
| `src-tauri/src/runtime/config/vault.rs` | VaultConfig、FolderMappings、MemoryConfig |
| `src-tauri/src/runtime/config/loader.rs` | ConfigLoader |
