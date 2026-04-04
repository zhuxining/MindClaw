> **Status**: `active`
>
> 本文档描述跨模块接口契约（trait）和核心数据结构的位置与用途。随接口契约变更同步更新。

# 类型注册表

## 跨模块接口契约（Traits）

### Agent 核心契约

| Trait | 位置 | 关键约束 | 用途 |
|-------|------|---------|------|
| `AgentHook` | `src/agent/hook.rs` | 七个扩展点方法，`finalize_content` 返回处理后内容 | AgentRunner 与业务层之间的生命周期桥梁 |
| `Tool` | `src/agent/tools/traits.rs` | `execute(&self, input: Value) -> Result<Value, ToolError>`，必须声明 `name()` 和 `schema()` | 内置工具实现统一接口 |
| `McpTransport` | `src/agent/tools/mcp.rs` | `start()` / `stop()` / `send_request()`，支持 stdio 和 streamable-http | MCP 客户端传输层抽象 |

### Provider 契约

| Trait | 位置 | 关键约束 | 用途 |
|-------|------|---------|------|
| `LlmProvider` | `src/providers/traits.rs` | `chat()` / `chat_stream()` 两种调用模式，返回标准 `Message` 类型 | 统一 Claude / OpenAI 兼容 API 调用 |

### Channel 契约

| Trait | 位置 | 关键约束 | 用途 |
|-------|------|---------|------|
| `Channel` | `src/channels/traits.rs` | `start()` / `stop()` / `send_message()`，异步生命周期管理 | Desktop / Telegram / Feishu 统一接口 |

### Storage 契约

| Trait | 位置 | 关键约束 | 用途 |
|-------|------|---------|------|
| `NoteStorage` | `src/storage/markdown.rs` | 读写 Markdown 文件，维护 frontmatter 元数据 | Vault 笔记持久化 |
| `VectorStorage` | `src/storage/vector.rs` | `index()` / `search()` / `delete()`，支持语义搜索 | 向量索引与检索 |

---

## 核心数据结构

### Agent 执行契约

| 结构体 | 位置 | 用途 | 关键约束 |
|--------|------|------|---------|
| `AgentRunSpec` | `src/agent/spec.rs` | 一次 Agent 执行的完整声明式配置 | 构建后不可变，Clone 实现 |
| `AgentRunResult` | `src/agent/spec.rs` | 一次执行的完整结构化输出 | 包含完整消息链，用于 Turn 持久化 |
| `Message` | `src/agent/spec.rs` | 单条对话消息 | System / User / Assistant / ToolResult 四种角色 |
| `ToolCall` | `src/agent/spec.rs` | LLM 请求的工具调用 | 含 `tool_call_id`，用于结果匹配 |
| `IterationState` | `src/agent/runner.rs` | 单次迭代运行时快照 | 仅迭代期间存在，传给 Hook |

### 消息总线

| 结构体 | 位置 | 用途 | 关键约束 |
|--------|------|------|---------|
| `InboundMessage` | `src/bus/events.rs` | 用户到 Agent 的消息 | `session_key` 格式 `{channel}:{chat_id}` |
| `OutboundMessage` | `src/bus/events.rs` | Agent 到用户的响应 | Delta / Done / Error / ToolProgress 四种类型 |

### 领域模型

| 结构体 | 位置 | 用途 | 关键约束 |
|--------|------|------|---------|
| `Session` | `src/agent/session.rs` | 运行时会话状态 | 按 `session_key` 唯一标识 |
| `Turn` | `src/models/conversation.rs` | 一次"用户输入 + Agent 响应"记录 | 持久化到 SQLite |
| `Memory` | `src/agent/memory/types.rs` | Agent 私有观察记录 | 含重要性权重和衰减系数 |
| `Note` | `src/models/note.rs` | 用户知识笔记 | 对应 Markdown 文件，SQLite 维护索引 |
| `Task` | `src/models/task.rs` | 用户待办项 | 支持状态流转 |

### 错误类型

| 类型 | 位置 | 用途 |
|------|------|------|
| `AppError` | `src/error.rs` | 全局错误类型，实现 `Serialize` 供前端消费 |
| `ToolError` | `src/agent/tools/traits.rs` | 工具执行错误，传给 LLM 做决策 |
| `StopReason` | `src/agent/spec.rs` | Agent 执行停止原因枚举：Completed / MaxIterations / ToolError / Cancelled |

---

## 关键类型签名

### AgentHook

```rust
pub trait AgentHook: Send + Sync {
    fn wants_streaming(&self) -> bool;
    fn before_iteration(&mut self, state: &IterationState);
    fn on_stream(&mut self, delta: &str);
    fn on_stream_end(&mut self, resuming: bool);
    fn before_execute_tools(&mut self, calls: &[ToolCall]);
    fn after_iteration(&mut self, state: &IterationState);
    fn finalize_content(&mut self, content: &str) -> String;
}
```

### LlmProvider

```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn chat(&self, messages: &[Message], tools: &[ToolSchema]) -> Result<Message, AppError>;
    async fn chat_stream(
        &self,
        messages: &[Message],
        tools: &[ToolSchema],
    ) -> Result<BoxStream<'_, Result<String, AppError>>, AppError>;
}
```

### Channel

```rust
#[async_trait]
pub trait Channel: Send + Sync {
    async fn start(&self, bus: Arc<MessageBus>) -> Result<(), AppError>;
    async fn stop(&self) -> Result<(), AppError>;
    async fn send_message(&self, message: OutboundMessage) -> Result<(), AppError>;
}
```
