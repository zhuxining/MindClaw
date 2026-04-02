# Tools And Services

## Tool Model

V2 只承认两类工具：

- `Resident tools`
- `Discoverable tools`

```rust
pub enum ToolKind {
    Resident,
    Discoverable,
}

pub struct ToolDescriptor {
    pub name: String,
    pub kind: ToolKind,
    pub description: String,
    pub input_schema: JsonSchema,
}
```

## Resident Tools

Resident tools 始终存在于 `ToolCatalog` 中，适合高频、稳定、可预测的能力：

- 文件读写与搜索
- 受控 shell
- service wrapper tools

它们在 run 开始时即可进入 `ActiveTools`。

## Discoverable Tools

Discoverable tools 是外部能力，通常来自 MCP 或其他连接器。

规则如下：

- 它们先存在于 `ToolCatalog.discoverable_tools`
- run 过程中可以被发现并加入 `ActiveTools`
- 激活结果只影响当前 run
- 工具元数据不直接写入会话历史

V2 只要求“发现”与“激活”是显式步骤，不强制要求旧版 `capability_search` 的统一入口形式。

## No Mixed Activation Semantics

旧版的问题是把 MCP 工具和 Skill 放进同一个发现接口，但激活结果完全不同。

V2 取消这种混合设计：

- discoverable tool 的激活结果只能是“加入 `ActiveTools`”
- prompt augmentation 不属于工具激活语义

因此，Skill 不再是 Agent Core 一等概念。

## Tool Execution Contract

```rust
pub trait ToolExecutor {
    async fn execute(&self, call: ToolCall) -> ToolResult;
}
```

V2 的最小约束：

- 输入必须通过 schema 校验
- 输出必须可序列化
- 失败必须结构化返回
- 工具执行细节不允许修改 session history

工具是否串行或并行属于执行策略，不是 Agent Core 的主边界。

## Hooks Are Optional Extensions

Hooks 不再属于核心主流程。

如果保留，只允许作为后置扩展：

- 审计
- 观测
- 策略控制

它们不能改变以下核心边界：

- session 串行模型
- run-local state ownership
- background task model

## Services Are The Business Core

`Services` 是稳定业务逻辑层，Agent 只通过工具包装访问它们。

```rust
pub struct ServiceContainer {
    pub knowledge: KnowledgeService,
    pub daily: DailyService,
    pub task: TaskService,
}
```

### Service Rules

- Service API 与 memory 无关
- Service API 与 background task 调度无关
- Agent 通过 tool wrapper 调用 service，而不是直接把业务逻辑塞进 loop

## Service Wrapper Tools

面向 Agent 的 service tool 建议保持一一对应：

- `knowledge`
- `daily`
- `task`

这样可以保证：

- tool schema 稳定
- 业务能力边界清晰
- Agent Core 与业务实现解耦

## Out Of Scope

以下内容不属于 V2 Agent Core：

- Skill 生命周期
- Skill 上下文保护
- Skill fork 成 SubAgent
- 以 Hook 为中心的插件框架
