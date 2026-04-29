> **Status**: `active`
>
> 本文档描述跨模块接口契约（trait）和核心数据结构的位置与用途。随接口契约变更同步更新。

# 类型注册表

## 跨模块接口契约（Traits）

### Agent 核心契约

| Trait | 位置 | 关键约束 | 用途 |
|-------|------|---------|------|
| `RunHooks` | `src/agent/hooks.rs` | 七个扩展点方法，`finalize_content` 返回处理后内容 | AgentRunner 与业务层之间的生命周期桥梁 |
| `RunHookPublisher` | `src/agent/hooks.rs` | 发布状态、文本分段和段结束信号 | 将 `InteractiveRunHooks` 事件桥接到 MessageBus |
| `Tool` | `src/agent/tools/traits.rs` | `execute(&self, input: Value) -> Result<Value, ToolError>`，必须声明 `name()` 和 `schema()` | 内置工具实现统一接口 |
| `McpTransport` | `src/agent/tools/mcp.rs` | `start()` / `stop()` / `send_request()`，支持 stdio 和 streamable-http | MCP 客户端传输层抽象 |

### Provider 契约

| Trait | 位置 | 关键约束 | 用途 |
|-------|------|---------|------|
| `Provider` | `src/providers/traits.rs` | `chat()` / `chat_stream()` 两种调用模式，返回标准 `ProviderResponse` 或流事件 | 统一 Claude / OpenAI 兼容 API 调用 |

### Channel 契约

| Trait | 位置 | 关键约束 | 用途 |
|-------|------|---------|------|
| `Channel` | `src/channels/traits.rs` | `start()` / `stop()` / `send_message()`，异步生命周期管理 | Desktop / Telegram / Feishu 统一接口 |

### Storage 契约

| Trait | 位置 | 关键约束 | 用途 |
|-------|------|---------|------|
| `ContextStore` | `src/storage/` | 使用 ContextURI 读写上下文，维护 Frontmatter、PathGuard 和 ContextIndex | Services 访问 Vault、Inbox、source、agent 资产的统一入口 |
| `MarkdownStorage` | `src/storage/markdown.rs` | 读写 Markdown 文件，解析 Frontmatter，保证原子写入 | ContextFS 的 Markdown 文件适配 |
| `ContextIndex` | `src/storage/database/vault.rs` | 保存可重建文档级索引，不保存内容真相 | L0 查询、FTS 搜索和召回候选过滤 |
| `VectorStorage` | `src/storage/vector.rs` | `index()` / `search()` / `delete()`，只保存可重建语义缓存或引用 | 可选语义搜索增强 |

---

## 核心数据结构

### Agent 执行契约

| 结构体 | 位置 | 用途 | 关键约束 |
|--------|------|------|---------|
| `AgentRunSpec` | `src/agent/spec.rs` | 一次 Agent 执行的完整声明式配置 | 构建后不可变，Clone 实现 |
| `AgentRunResult` | `src/agent/spec.rs` | 一次执行的完整结构化输出 | 包含完整消息链，用于 Turn 持久化 |
| `AgentProfile` | `src/agent/agents.rs` | Agent 静态定义 | 当前包含 model 与 execution 默认值，是定义层的最小可用形态 |
| `AgentRegistry` | `src/agent/agents.rs` | AgentProfile 注册表 | 当前为轻量内存注册表，已接入 AppRuntime / AgentLoop / spawn |
| `ModelRouter` | `src/agent/agents.rs` | 根据 profile 解析模型 | 当前为最小实现，直接返回 profile.model，已接入主执行链路 |
| `ChatMessage` | `src/providers/traits.rs` | 单条对话消息 | System / User / Assistant / ToolResult 四种角色 |
| `ToolCall` | `src/agent/tools/mod.rs` | LLM 请求的工具调用 | 含 `tool_call_id`，用于结果匹配 |
| `IterationState` | `src/agent/runner.rs` | 单次迭代运行时快照 | 仅迭代期间存在，传给 Hook |
| `AgentSpawnDispatcher` | `src/agent/spawn.rs` | 管理派生执行 | 当前已接通 inline `SubAgent` 与后台派发 |
| `SubAgentDef` | `src/agent/spawn.rs` | 子代理静态定义 | 含 mode / model / capabilities / prompt |
| `ProviderEvent` | `src/agent/events.rs` | Provider 到 Runner 的标准化流事件 | 只表达流式协议事件，不承载 runtime 观测语义 |
| `ProviderUsage` | `src/agent/events.rs` | Provider 原始 token 使用量 | 与 `TokenUsage` 区分，前者是 provider 原始统计，后者是 run 聚合统计 |
| `AgentEvent` | `src/agent/events.rs` | Runtime 内部观测事件 | 供 observability / tracing / metrics 消费 |
| `LoopPhase` | `src/agent/events.rs` | Runtime 内部阶段机 | 不直接暴露给前端 |
| `UserVisiblePhase` | `src/agent/events.rs` | 对外简化状态 | 通过 MessageBus / Channel 暴露给前端 |
| `ContextUri` | `src/storage/` | 上下文稳定引用 | 跨 Vault 文件、source、agent 资产和 session 证据引用 |
| `ContextFrontmatter` | `src/storage/markdown.rs` | 可索引 Markdown 的通用 Frontmatter | 承载 `tags`、`overview`、`source`、引用和来源扩展 |
| `MemoryFrontmatter` | `src/agent/memory.rs` | Agent 记忆文档的 Frontmatter 扩展 | Markdown 是记忆真相源，ContextIndex 只维护召回索引 |
| `Memory` | `src/agent/memory.rs` | Agent 可召回记忆记录 | 对应 Vault 中的受管 Markdown 文件 |
| `SkillManifest` | `src/agent/skills.rs` | 技能完整清单 | 启动时只索引元数据，激活时加载完整内容 |

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

### RunHooks

```rust
pub trait RunHooks: Send {
    fn wants_streaming(&self) -> bool;
    fn before_iteration(&mut self, state: &mut IterationState);
    fn on_stream(&mut self, delta: &str);
    fn on_stream_end(&mut self, resuming: bool);
    fn before_execute_tools(&mut self, calls: &[ToolCall]);
    fn after_iteration(&mut self, state: &IterationState);
    fn finalize_content(&mut self, content: &str) -> String;
}
```

### Provider

```rust
#[async_trait]
pub trait Provider: Send + Sync {
    fn name(&self) -> &str;
    fn model_id(&self) -> &str;
    async fn chat_stream(
        &self, request: ChatRequest<'_>
    ) -> AppResult<Pin<Box<dyn Stream<Item = AppResult<ProviderEvent>> + Send>>>;
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
