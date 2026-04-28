> **Status**: `active`

# Providers — LLM Provider Adapter 层

---

## § 职责定位

Providers 层负责将 `AgentRunner` 的统一 `ChatRequest` 适配到各服务商 API，不负责上下文构建、Session 管理、工具执行或 Agent 路由。

---

## § 核心原则

**只做协议适配**：Provider Adapter 只处理传输、认证、重试、流式解析与 vendor-specific 请求映射。

**能力声明集中**：模型支持的 streaming、tools、structured output 等能力由 Provider/Model Profile 声明，而不是散落在 AgentLoop 中硬编码。

**请求语义统一**：上层只认识 `ChatRequest`、`ChatResponse` 与 `ProviderStreamEvent`，不直接使用 Claude/OpenAI 原生消息格式。

---

## § 核心对象

**LLMProviderClient trait**

所有 LLM 服务商适配器的统一接口。

```rust
async fn chat(&self, req: &ChatRequest) -> Result<ChatResponse>
fn chat_stream(&self, req: &ChatRequest) -> impl Stream<Item = ProviderStreamEvent>
fn supports_model(&self, model: &str) -> bool
```

**ChatRequest**

单次 LLM 调用的统一请求体。
关键属性：消息列表、工具 schema、模型标识、采样参数、响应格式。

**ProviderStreamEvent**

流式响应的统一事件。
关键属性：文本增量、工具调用、完成事件、用量统计。

**ProviderRegistry**

Provider Adapter 的注册表。
关键属性：provider id、模型映射、默认模型、能力目录。
关系：由 AppRuntimeBuilder 初始化，注入 AgentRunner；AgentRunner 根据 `AgentRunSpec.resolved_provider` 查找对应 Adapter。

---

## § 已支持的 Provider 类型

| Provider 类型       | 覆盖模型                    | 认证方式                |
| ------------------- | --------------------------- | ----------------------- |
| Anthropic（Claude） | Claude 系列                 | `X-API-Key`             |
| OpenAI 兼容         | OpenAI、DeepSeek、Ollama 等 | `Authorization: Bearer` |

---

## § 关键流程

1. AppRuntimeBuilder 读取 Provider 配置与密钥
2. 构建 Provider Adapter，并注册到 ProviderRegistry
3. AgentLoop 经由 ModelRouter 解析出本次 run 的 provider/model
4. AgentRunner 根据 `resolved_provider` 选择 Adapter
5. Adapter 将统一 `ChatRequest` 转为厂商请求体
6. 流式模式下，Adapter 将 SSE/stream 解析为 `ProviderStreamEvent`
7. AgentRunner 消费事件并驱动迭代循环

---

## § Provider 与 Runtime 的边界

Provider Adapter 不负责：

- 选择用哪个 AgentProfile
- 组装历史上下文
- 过滤工具白名单
- 执行工具调用
- 处理 child/background invocation

这些职责都属于 Agent Runtime。

---

## § 设计决策与权衡

| 决策问题                    | 选择                                   | 放弃的替代方案                  | 理由                                        |
| --------------------------- | -------------------------------------- | ------------------------------- | ------------------------------------------- |
| 如何支持多个 LLM 服务商？   | `LLMProviderClient + ProviderRegistry` | 单一硬编码实现                  | 便于扩展新服务商且不污染 Agent Runtime      |
| 流式与非流式是否分开接口？  | 是                                     | 单一动态返回类型                | 两种调用返回形态不同，分开更清晰            |
| Provider 是否持有业务状态？ | 否                                     | Provider 直接感知 Session/Agent | Provider 只应关心协议与能力，而不是业务编排 |
| 模型能力由谁声明？          | Provider / Model Profile               | AgentLoop 手写判断              | 能力声明集中后，模型替换和回退策略更稳定    |
