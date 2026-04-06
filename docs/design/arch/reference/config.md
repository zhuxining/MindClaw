> **Status**: `active`
>
> 本文档列出所有配置项的名称、类型、默认值和说明。随配置字段增减同步更新。

# 配置说明

## 加载顺序

配置按以下优先级加载（后者覆盖前者）：

1. **硬编码默认值**（代码中 `Default` 实现）
2. **配置文件**（`~/.config/mindclaw/config.toml`，若存在）
3. **环境变量**（`MINDCLAW_*` 前缀）

---

## 配置项清单

### 路径配置

| 名称 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `data_dir` | PathBuf | `~/.config/mindclaw`（macOS/Linux）或 `%APPDATA%\mindclaw`（Windows） | 应用数据根目录，存储数据库、配置、vault |
| `db_path` | PathBuf | `{data_dir}/mindclaw.db` | SQLite 数据库文件路径 |
| `vault_path` | PathBuf | `{data_dir}/vault` | 知识库 Markdown 文件存储目录 |

### Provider 配置

| 名称 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `provider_id` | String | `"deepseek"` | 默认 LLM Provider 标识，可选：`deepseek`、`openai`、`claude` |
| `model_id` | Option<String> | `None` | 可选的模型 ID 覆盖，为空时使用 Provider 默认模型 |

### 运行时配置

| 名称 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `bus_capacity` | usize | `100` | MessageBus 有界队列容量，Inbound 和 Outbound 各此容量 |
| `context_token_limit` | usize | `128000` | 上下文窗口 token 上限，用于历史消息压缩触发判断 |
| `system_prompt` | String | `"你是一个智能助手..."` | Agent 系统提示词，作为 Core 层上下文的一部分 |

---

## 配置文件格式

配置文件位置：`~/.config/mindclaw/config.toml`

```toml
# 示例配置
data_dir = "/Users/username/.config/mindclaw"
provider_id = "deepseek"
model_id = "deepseek-chat"
bus_capacity = 100
context_token_limit = 128000
system_prompt = """你是一个智能助手，可以帮助用户完成各种任务。"""
```

---

## 环境变量

支持通过环境变量覆盖配置（`MINDCLAW_` 前缀 + 大写下划线格式）：

| 环境变量 | 对应配置项 | 示例值 |
|----------|-----------|--------|
| `MINDCLAW_DATA_DIR` | `data_dir` | `/path/to/data` |
| `MINDCLAW_PROVIDER_ID` | `provider_id` | `claude` |
| `MINDCLAW_MODEL_ID` | `model_id` | `claude-opus-4` |
| `MINDCLAW_BUS_CAPACITY` | `bus_capacity` | `200` |
| `MINDCLAW_CONTEXT_TOKEN_LIMIT` | `context_token_limit` | `64000` |

---

## 密钥配置

API Key **不**存储在配置文件中，而是存储在 OS Keychain：

| Provider | Keychain 服务名 | 说明 |
|----------|----------------|------|
| DeepSeek | `mindclaw-deepseek-api-key` | DeepSeek API 密钥 |
| OpenAI | `mindclaw-openai-api-key` | OpenAI API 密钥 |
| Claude | `mindclaw-anthropic-api-key` | Anthropic API 密钥 |

密钥通过 Tauri 命令层写入/读取，Services 层不直接访问 Keychain。
