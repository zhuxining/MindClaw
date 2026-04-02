# MindClaw 技术架构设计 — 命令架构

> 完整架构文档索引见 [README.md](./README.md)

## 三层命令架构

系统提供三种命令入口，底层共享 Services 层：

```
React Frontend ── invoke() ──► Web Commands ──► Services ──► Storage
对话中 /xxx ─────────────────► Agent Commands ─► Agent 生命周期控制
终端 mindclaw ─────────────► CLI Commands ──► Services ──► Storage
```

### 跨层命令矩阵

| 维度       | Web Commands       | Agent Commands            | CLI Commands          |
| ---------- | ------------------ | ------------------------- | --------------------- |
| 入口       | React `invoke()`   | 对话消息 `/xxx`           | 终端 `mindclaw`       |
| 职责       | 业务 CRUD          | Agent 生命周期控制        | 自动化/脚本           |
| 数量       | ~25 个             | 4 个                      | ~12 个                |
| 调用链     | Command → Services | AgentLoop 拦截            | AppRuntime → Services |
| 需要 Tauri | 是                 | 是（运行在 AgentLoop 内） | 否                    |
| 需要 LLM   | 否                 | 否（纯控制）              | 仅 agent 子命令       |

**Session Commands 说明**：在 `AgentLoop.dispatch()` 中拦截，位于 Session 加载之后、`agent.run()` 调用之前。拦截后直接返回确定性结果，不调用 Provider。

---

## Web Commands — Tauri IPC

前后端通过 `invoke()` 通信，命令返回 `Result<T, AppError>`。

### 命令清单

| 分类             | 命令                  | 说明                                               |
| ---------------- | --------------------- | -------------------------------------------------- |
| **Conversation** | `send_message`        | 发送消息，返回 request_id，响应通过 Event 流式推送 |
|                  | `get_session_history` | 获取会话历史                                       |
| **Daily**        | `get_daily`           | 获取/创建当日日记                                  |
|                  | `save_daily`          | 保存日记                                           |
| **Tasks**        | `list_tasks`          | 任务列表                                           |
|                  | `create_task`         | 创建任务                                           |
|                  | `update_task_status`  | 更新任务状态                                       |
| **Knowledge**    | `search_knowledge`    | 搜索知识条目                                       |
|                  | `get_knowledge`       | 获取知识笔记                                       |
| **Settings**     | `get_settings`        | 读取设置                                           |
|                  | `save_settings`       | 保存设置                                           |
|                  | `set_api_key`         | 存入 OS Keychain                                   |
| **System**       | `get_system_status`   | 系统健康状态                                       |

### 流式响应

通过 Tauri Event `conversation_event` 推送：

```
{ session_id, request_id, payload }

payload =
  { type: "chunk", content }
  | { type: "done" }
  | { type: "error", message, retryable }
  | { type: "status", phase }
```

前端通过 `listen("conversation_event", callback)` 接收。

### State 注入

通过 `AppRuntime` 统一注入（见 [10-runtime.md](./10-runtime.md)）：

```rust
#[tauri::command]
pub async fn list_tasks(
    runtime: State<'_, Arc<AppRuntime>>,
) -> AppResult<Vec<Task>>
```

---

## Session Commands — 控制指令

对话中输入 `/xxx` 控制会话行为，不触发 LLM。

| 指令       | 说明     | 行为                             |
| ---------- | -------- | -------------------------------- |
| `/new`     | 新建会话 | 关闭当前 Session，创建新 Session |
| `/stop`    | 停止操作 | 取消进行中的任务，中断流式响应   |
| `/restart` | 重启服务 | 重新初始化 AgentLoop             |
| `/status`  | 查看状态 | 返回运行状态、活跃 Session 数等  |

### 核心设计

- **拦截点**：`AgentLoop.dispatch()`，Session 加载后、`agent.run()` 调用前
- **注册表**：`SessionCommandRegistry`，支持自定义指令
- **上下文**：`SessionCommandContext` 提供 session、cancel_token 等
- **返回**：`SessionCommandResult { response, action }`，action 指示后续行为

---

## CLI Commands — 终端命令

独立二进制，不启动 Tauri/UI。

### 命令清单

| 子命令                          | 说明                |
| ------------------------------- | ------------------- |
| `mindclaw agent`                | 交互式对话（REPL）  |
| `mindclaw agent -m <msg>`       | 单轮对话后退出      |
| `mindclaw agent --session <id>` | 继续指定会话        |
| `mindclaw agent -p <provider>`  | 临时指定 Provider   |
| `mindclaw status`               | 系统状态摘要        |
| `mindclaw session list`         | 列出会话            |
| `mindclaw session export <id>`  | 导出会话            |
| `mindclaw session delete <id>`  | 删除会话            |
| `mindclaw config init`          | 初始化配置          |
| `mindclaw config show`          | 显示配置            |
| `mindclaw config set <k> <v>`   | 设置配置项          |
| `mindclaw completions <shell>`  | 生成 Shell 补全脚本 |

### 运行时

CLI 直接使用 `AppRuntime`：

```rust
let rt = AppRuntime::builder().build().await?;
rt.start().await?;
// CLI 特有逻辑（REPL、终端输出）
rt.shutdown().await;
```

---

## 层级关系

```
Web Commands (Tauri IPC)
Agent Commands (AgentLoop 拦截)
CLI Commands (独立二进制)
         │
         ▼
    AppRuntime
         │
    ┌────┴────┐
Services   Agent
    │        │
Storage   Provider
```
