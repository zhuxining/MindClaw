> **Status**: `active`
> **Last updated**: 2026-05-01

# Providers — LLM Provider Adapter 层

---

## § 职责定位

Providers 层负责 Provider 配置、密钥读取、主模型/轻量模型解析，并为 AgentRunner 创建 `AgentModelSet`；不负责上下文构建、Session 管理、工具执行、stream 消费或 Agent 路由。

---

## § 核心原则

**rig 作为唯一 Provider 来源**：所有 LLM 服务商通过 rig 内置 provider 模块访问，不再手写 Provider Adapter。

**不再二次封装执行 API**：`LLMCompletionModel` 只是具体 Rig completion model 的枚举容器，不暴露 `complete()` / `stream()`；执行语义集中在 `AgentRunner`。

**只向 Runner 提供模型集合**：ProviderRegistry 读取当前 provider 的主模型与轻量模型配置，创建 `AgentModelSet { main, light }` 交给 AgentRunner；轻量模型未配置时回退到主模型。

**契约保持稳定**：上层仍使用 MindClaw 的 `AgentRunSpec` 和 `AgentRunResult`，rig 类型不穿透到业务层。

**Secret 由 OS Keychain 管理**：API Key 不以明文落盘，由 keychain 模块安全存储。

---

## § 核心对象

**ProviderConfig**

单个 LLM 提供商的配置定义。
关键属性：name、api_base、api_key_env、models、default_model。

**ModelConfig**

单个模型的配置定义。
关键属性：id、display_name、tier、max_output_tokens、context_window。

**ProviderRegistry**

rig client 的创建工厂。
关键属性：configs（HashMap<String, ProviderConfig>）。
关系：由 AppRuntimeBuilder 初始化；根据当前配置创建 `AgentModelSet`。

**AgentModelSet**

AgentRunner 持有的主模型/轻量模型集合。
关键属性：main、light、main_model_id、light_model_id。
关系：Profile/Spec 仍只携带模型 id；Runner 根据 id 在主模型和轻量模型之间选择，不接收 provider name。

**rig Client**

rig 框架的 LLM 客户端。
类型：`anthropic::Client`、`openai::Client`、`deepseek::Client` 等。
用途：创建 completion model。Agent、StreamingPromptRequest、ToolServer 和 PromptHook 在 AgentRunner 内部构建。

---

## § 已支持的 Provider 类型

| Provider 类型       | rig 模块                    | 覆盖模型      | 认证方式                |
| ------------------- | --------------------------- | ------------- | ----------------------- |
| Anthropic（Claude） | `rig::providers::anthropic` | Claude 系列   | `X-API-Key`             |
| OpenAI              | `rig::providers::openai`    | GPT 系列      | `Authorization: Bearer` |
| DeepSeek            | `rig::providers::deepseek`  | DeepSeek 系列 | `Authorization: Bearer` |
| Gemini              | `rig::providers::gemini`    | Gemini 系列   | API Key                 |
| Groq                | `rig::providers::groq`      | Groq 系列     | `Authorization: Bearer` |
| Mistral             | `rig::providers::mistral`   | Mistral 系列  | `Authorization: Bearer` |
| Ollama              | `rig::providers::ollama`    | 本地模型      | 无认证                  |
| 其他 20+            | rig 内置                    | 各厂商模型    | 各厂商认证方式          |

---

## § 关键流程

1. AppRuntimeBuilder 读取 Provider 配置
2. ProviderRegistry 加载内置 ProviderConfig
3. 运行时从 OS Keychain 获取 API Key
4. ProviderRegistry 解析主模型；轻量模型为空时回退主模型
5. ProviderRegistry 创建 Rig client 和 `AgentModelSet`
6. AgentRunner 使用 `AgentModelSet` + Rig AgentBuilder / StreamingPromptRequest 执行

---

## § Provider 与 Runtime 的边界

Provider 层不负责：

- 选择用哪个 AgentProfile
- 组装历史上下文
- 过滤工具白名单
- 执行工具调用
- 处理 child/background invocation

这些职责都属于 Agent Runtime。

rig client/model 创建在 Provider 层完成；Rig Agent 构建、streaming、tool calling 和 history 转换属于 Execution Layer。Provider name 不穿透到 AgentRunner 的构造入口，Rig 类型不穿透到 Orchestration Layer 或 Definition Layer。

---

## § 文件结构

当前 Providers 目录结构：

```text
providers/
├── mod.rs          # 导出所有公开类型
├── config.rs       # ProviderConfig、ModelConfig、ModelTier 定义 + 内置配置
└── registry.rs     # LLMClient / LLMCompletionModel / AgentModelSet / ProviderRegistry
```

已删除文件：

- ✅ `traits.rs`（自定义 Provider trait）
- ✅ `claude.rs`（手写 Anthropic API）
- ✅ `openai_compat.rs`（async-openai 封装）
- ✅ `rig_adapter.rs`（adapter 模式）

---

## § 设计决策与权衡

| 决策问题                    | 选择                                  | 放弃的替代方案                   | 理由                                                      |
| --------------------------- | ------------------------------------- | -------------------------------- | --------------------------------------------------------- |
| 如何支持多个 LLM 服务商？   | 使用 rig 内置 providers               | 手写每个 Provider Adapter        | rig 提供统一抽象，减少适配代码和维护负担                  |
| Provider trait 是否保留？   | 删除，`LLMCompletionModel` 只保存具体 Rig model | 保留自定义 Provider trait | Runner 需要 Rig `CompletionModel` 约束，自定义 trait 会重复抽象 |
| 流式响应如何处理？          | Runner 使用 Rig `StreamingPromptRequest` | Provider 层暴露 stream API | streaming 是执行语义，应由 Runner 和 PromptHook 统一处理 |
| Provider 枚举如何处理？     | ProviderRegistry 创建模型集合，Runner 只按已解析模型做 Rust 类型分派 | Runner 接收 provider name 再解析 | provider 选择应停在配置层，Runner 不需要知道当前 provider id |
| ProviderRegistry 职责？     | 作为 rig client 工厂和主/轻量模型解析器 | 持有 Provider 实例并管理生命周期 | rig client 轻量，按需创建；Runner 只需要可执行模型集合 |
| Provider 是否持有业务状态？ | 否                                    | Provider 直接感知 Session/Agent  | Provider 只应关心协议与能力，而不是业务编排               |
