> **Status**: `active`

# Providers — LLM 适配层

---

## § 职责定位

Providers 层负责将 AgentRunner 的 LLM 调用请求适配到各服务商 API，不负责上下文构建、工具执行、会话管理或任何业务逻辑。

---

## § 边界与实体

**输入**：AgentRunner 传入的消息列表（`Vec<ChatMessage>`）、工具 Schema 列表（`Vec<ToolSchema>`）和模型参数（模型标识、采样参数）。

**输出**：LLM 响应内容，以两种形式提供：

- `ChatResponse`（非流式）：包含完整响应文本、工具调用列表、停止原因、Token 用量。
- `Stream<ProviderEvent>`（流式）：事件序列，涵盖文本增量、工具调用信息、用量统计。

**核心实体**：

**Provider trait**：所有 LLM 服务商适配器的统一接口契约。

```
async fn chat(&self, req: &ChatRequest) -> Result<ChatResponse>
fn chat_stream(&self, req: &ChatRequest) -> impl Stream<Item = ProviderEvent>
fn model_id(&self) -> &str
```

**ChatRequest**：单次 LLM 调用的完整请求参数，服务商无关的内部表示。
关键属性：消息列表（`Vec<ChatMessage>`）、工具 Schema 列表、模型标识符、采样温度、最大 token 数。
关系：由 AgentRunner 构建，传给 Provider；Provider 将其转换为服务商特定的 HTTP 请求体。

**ChatMessage**：消息列表中的单条消息，与任何服务商 API 格式解耦。
关键属性：角色（System / User / Assistant / ToolResult）、文本内容、工具调用列表（仅 Assistant）、媒体附件（仅 User）。
关系：由 ContextPipeline 构建，通过 AgentRunSpec 传入 AgentRunner，再传给 Provider 进行格式转换。

**ProviderEvent**：流式响应中的单个事件，表示一段增量或状态变更。
关键属性：事件类型（TextDelta / ToolCallStart / ToolCallArgsDelta / ToolCallEnd / StreamEnd）、事件内容。
关系：由流式 Provider 持续产生，由 AgentRunner 处理并通过 `hook.on_stream()` 转发给 AgentHook。

**ProviderRegistry**：已配置 Provider 实例的路由表，根据模型标识符查找对应 Provider。
关键属性：模型标识符前缀到 Provider 实例的映射。
关系：由 AppRuntimeBuilder 初始化，注入 AgentRunner；AgentRunner 在每次 LLM 调用前通过 Registry 查找 Provider。

---

## § 已支持的 Provider

| Provider 类型 | 覆盖模型 | 认证方式 |
|-------------|---------|---------|
| Anthropic（Claude） | claude-opus-4-6、claude-sonnet-4-6、claude-haiku-4-5 等 | `X-API-Key` 请求头 |
| OpenAI 兼容 | OpenAI GPT 系列、Ollama 本地模型、DeepSeek 等 | `Authorization: Bearer` 请求头 |

---

## § 关键流程

1. AppRuntimeBuilder 从 OS Keychain 读取各服务商的 API Key，构建 Provider 实例，注册到 ProviderRegistry。
2. AgentRunner 从 AgentRunSpec 读取 `model` 字段，向 ProviderRegistry 查询对应 Provider。
3. AgentRunner 根据 `hook.wants_streaming()` 选择调用 `provider.chat()` 或 `provider.chat_stream()`。
4. Provider 将 `ChatRequest` 转换为服务商特定的 JSON 请求体，通过 HTTP 发送至 LLM API。
5. 流式模式下，Provider 解析 SSE（Server-Sent Events）响应，将每个事件解析为 `ProviderEvent` 产出。
6. AgentRunner 消费 `ProviderEvent` 流：TextDelta 事件转发给 `hook.on_stream()`，ToolCall 事件积累后传给工具执行，StreamEnd 事件触发 `hook.on_stream_end()`。

---

## § 设计决策与权衡

| 决策问题 | 选择 | 放弃的替代方案 | 理由 |
|---------|------|--------------|------|
| 如何支持多个 LLM 服务商？ | Provider trait + ProviderRegistry（工厂模式） | 单一硬编码实现 | trait 允许新增服务商时只增加新实现文件，不修改 AgentRunner；OpenAI 兼容 Provider 已覆盖大量兼容接口 |
| 流式和非流式是否统一接口？ | 两个独立方法（`chat` / `chat_stream`） | 单一方法自动判断返回类型 | 两者返回类型不同（完整对象 vs 事件流），在 Rust 的类型系统中无法统一；分开方法意图更清晰 |
| ChatMessage 格式是否依赖服务商？ | 服务商无关的内部格式，Provider 负责转换 | 直接使用 Anthropic 或 OpenAI 的消息格式 | 内部格式统一使 ContextPipeline 和 AgentRunner 不依赖任何服务商 SDK，切换服务商无需改动上层 |
| API Key 如何传入 Provider？ | 初始化时从 OS Keychain 读取，注入构造函数 | 每次 LLM 调用前从 Keychain 读取 | 初始化时读取一次，避免频繁 Keychain I/O 系统调用；Provider 实例在内存中持有 Key（生命周期与 AppRuntime 绑定） |
| 如何处理 API 调用失败？ | Provider 层重试（指数退避），超出上限返回错误 | AgentRunner 层重试 | Provider 最了解服务商的限速策略（rate limit 重试）；AgentRunner 层重试会重复执行 `before_iteration` 等 Hook 逻辑 |
| 工具调用格式如何适配？ | Provider 层将内部格式转换为服务商特定格式（如 Claude 的 `tool_use` vs OpenAI 的 `function`） | 所有服务商使用统一格式 | 各服务商 API 格式差异客观存在，Provider 层屏蔽差异使上层无需感知 |
| 多个模型如何选择 Provider？ | ProviderRegistry 按模型标识符前缀路由（如 `claude-*` → ClaudeProvider） | AgentLoop 硬编码模型到 Provider 映射 | Registry 集中管理路由规则，新增模型类型只需修改 Registry 映射，不改动业务代码 |
| 如何处理模型不支持的特性？ | Provider 层返回错误（如不支持工具调用） | 上层检测模型标识符后跳过特性 | Provider 层最了解模型能力；返回错误使上层可以优雅降级（如提示用户切换模型） |
| 流式和非流式 Token 统计是否一致？ | 是，两者均返回 TokenUsage | 流式不返回用量 | 用量统计对成本监控和预算控制重要；流式响应结束时汇总用量并返回 |
