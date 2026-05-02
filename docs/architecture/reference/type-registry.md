> **Status**: `active`
>
> 本文档描述跨模块接口契约（trait）和核心数据结构的位置与用途。随接口契约变更同步更新。

# 类型注册表

## 跨模块接口契约（Traits）

### Agent 核心契约

| Trait | 位置 | 关键约束 | 用途 |
|-------|------|---------|------|
| `RunHooks` | `src/agent/hooks.rs` | 12 个生命周期观察方法，`finalize_response` 返回处理后内容 | RigPromptHook 与业务层之间的 observer 契约 |
| `RunHookPublisher` | `src/agent/hooks.rs` | 发布状态、文本分段和段结束信号 | 将 `InteractiveRunHooks` 事件桥接到 MessageBus |
| `rig::tool::ToolDyn` | `rig-core` | `definition()` / `call()`，由 Rig ToolSet / ToolServer 执行 | 内置工具、spawn 工具和 MCP 工具的唯一执行接口 |

### Provider 契约

Provider 层不再暴露自定义 trait。`ProviderRegistry` 负责配置、密钥和主/轻量模型解析，并返回 `AgentModelSet` 给 `AgentRunner`。

### Channel 契约

| Trait | 位置 | 关键约束 | 用途 |
|-------|------|---------|------|
| `Channel` | `src/channels/traits.rs` | `start()` / `stop()` / `send_message()`，异步生命周期管理 | Desktop / Telegram / Feishu 统一接口 |

### Storage 契约

| Trait | 位置 | 关键约束 | 用途 |
|-------|------|---------|------|
| `ContextStore` | `src/storage/` | 使用 ContextURI 读写上下文，维护 Frontmatter、PathGuard 和 ContextIndex | Services 访问 Vault、Inbox、外部资源和 agent 资产的统一入口 |
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
| `AgentProfile` | `src/agent/agents.rs` | Agent 静态定义 | Main Profile 固定创建；派生 Profile 由 Markdown definition 映射到 prompt / model / tool / context / execution 等策略 |
| `AgentMarkdownDefinition` | `src/agent/agents.rs` | Markdown 派生 Agent 定义 | frontmatter 承载 name / description / tools / model / maxTurns / skills，body 承载 system prompt |
| `AgentDefinitionFrontmatter` | `src/agent/agents.rs` | Markdown 派生 Agent frontmatter schema | 解析 `src/agent/build-in/subagent/*.md`，缺必填字段或空 body 启动失败 |
| `AgentRegistry` | `src/agent/agents.rs` | AgentProfile 注册表 | `bootstrap(...)` 是内置 Agent 唯一启动入口；Main Profile 固定创建，派生 Profile 从 Markdown 加载 |
| `ModelRouter` | `src/agent/agents.rs` | 根据 profile 解析模型 | 当前为最小实现，直接返回 profile.model，已接入主执行链路 |
| `ChatMessage` | `src/agent/messages.rs` | 单条对话消息 | Runtime 契约，由 AgentRunner 转换为 Rig `Message` |
| `ToolSchema` | `src/agent/messages.rs` | 历史兼容字段 | 当前工具执行以 `ToolDyn` / `ToolSet` 为准 |
| `ToolCallPlaceholder` | `src/agent/hooks.rs` | Hook 观测用工具调用占位 | Rig 执行工具，MindClaw 只向 observer 暴露名称和 id |
| `AgentSpawnDispatcher` | `src/agent/spawn.rs` | 管理派生执行 | 当前已接通 inline `SubAgent` 与后台派发 |
| `UserVisiblePhase` | `src/agent/events.rs` | 对外简化状态 | 通过 MessageBus / Channel 暴露给前端 |
| `ContextUri` | `src/storage/` | 上下文稳定引用 | 跨 Vault 文件、外部资源、agent 资产和 session 证据引用 |
| `ContextFrontmatter` | `src/storage/markdown.rs` | 可索引 Markdown 的通用 Frontmatter | 承载 `tags`、`overview`、`confidence`、`origin`、`refs` 和来源扩展 |
| `MemoryFrontmatter` | `src/agent/memory.rs` | Agent 记忆文档的 Frontmatter 扩展 | Markdown 是记忆真相源，ContextIndex 只维护召回索引 |
| `Memory` | `src/agent/memory.rs` | Agent 可召回记忆记录 | 对应 Vault 中的受管 Markdown 文件 |
| `SkillManifest` | `src/agent/skills.rs` | 技能完整清单 | 启动时只索引元数据，激活时加载完整内容 |
| `LLMCompletionModel` | `src/providers/registry.rs` | 具体 Rig completion model 枚举 | Runner 做 Rust 类型分派后进入泛型 Rig 执行路径 |
| `AgentModelSet` | `src/providers/registry.rs` | 主模型 / 轻量模型集合 | Runner 持有；轻量模型未配置时等同主模型 |

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
| `rig::tool::ToolError` | `rig-core` | 工具执行错误，传给 Rig Agent 做决策 |
| `StopReason` | `src/agent/spec.rs` | Agent 执行停止原因枚举：Completed / MaxIterations / ToolError / Cancelled |

---

## 关键类型签名

### RunHooks

```rust
pub trait RunHooks: Send {
    fn streaming_mode(&self) -> StreamingMode;
    fn on_run_start(&mut self, ctx: &RunStartContext);
    fn on_iteration_start(&mut self, ctx: &IterationStartContext);
    fn on_model_request_start(&mut self, ctx: &ModelRequestContext);
    fn on_model_text_delta(&mut self, delta: &str);
    fn on_model_response_ready(&mut self, ctx: &ModelResponseContext);
    fn on_tool_batch_start(&mut self, calls: &[ToolCallPlaceholder]);
    fn on_tool_call_start(&mut self, call: &ToolCallPlaceholder);
    fn on_tool_call_finish(&mut self, call: &ToolCallPlaceholder, success: bool, result_summary: &str);
    fn on_iteration_finish(&mut self, ctx: &IterationFinishContext);
    fn finalize_response(&mut self, content: &str) -> String;
    fn on_finish(&mut self, result: &AgentRunResult);
    fn on_abort(&mut self, reason: &RunAbortReason);
}
```

### Provider

Provider 层没有自定义 trait 签名。公开入口是 `ProviderRegistry::create_agent_models_from_env(...) -> AgentModelSet`，Runner 再按 spec model id 选择主模型或轻量模型。

### Channel

```rust
#[async_trait]
pub trait Channel: Send + Sync {
    async fn start(&self, bus: Arc<MessageBus>) -> Result<(), AppError>;
    async fn stop(&self) -> Result<(), AppError>;
    async fn send_message(&self, message: OutboundMessage) -> Result<(), AppError>;
}
```
