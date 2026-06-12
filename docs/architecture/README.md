# 架构文档

> 本目录描述 MindClaw 如何实现蓝图中的“本地优先的 ACP-native Agent 控制平面”。产品目标见 `docs/blueprint/00-overview.md`，需求验收见 `docs/prd/`，当前实现状态见 `docs/architecture/reference/migration.md`。

## 文档地图

| 文档 | 主题 |
|------|------|
| `00-overview.md` | 总体架构、层次边界、核心数据流 |
| `10-channel-gateway.md` | GatewaySupervisor、Channel gateway、Feishu-first 接入、App 内驻留 |
| `20-message-bus.md` | SessionDispatcher、EventBus、legacy MessageBus 边界 |
| `30-acp-client.md` | ACP-native execution layer、ACP Server registry、Transport / ToolExecutor 边界 |
| `35-agent-context.md` | Agent Identity、Skill instruction、prompt / context 组装 |
| `40-agent-skill-command.md` | Agent、Skill、SlashCommand、ConversationExecutionState 控制平面 |
| `reference/dependencies.md` | 依赖和架构约定 |
| `reference/directory-structure.md` | 当前与目标目录结构 |
| `reference/migration.md` | 当前实现状态与迁移路线 |
| `reference/traceability.md` | 蓝图 / PRD / 架构 / 代码模块追踪关系 |

## 文档边界

- 架构文档负责系统结构、模块职责、数据流、依赖边界和安全边界。
- 架构文档不维护用户故事和验收标准；这些内容在 `docs/prd/`。
- 架构文档不重复维护实现状态；实现状态只写在 `reference/migration.md`。
- 架构文档可以引用当前实现状态，但应避免写“已实现 / 未实现”的重复状态表。

## 核心架构原则

1. **ACP-native**：MindClaw 不自研基础 Agent Server；Agent 执行通过用户配置的 ACP Server 完成。
2. **Control plane**：MindClaw 管用户侧 Agent、Skill、SlashCommand、会话执行状态和渠道调度。
3. **Messaging-native**：MindClaw 把 Agent 接入 Feishu 等真实消息渠道。
4. **Local-first**：消息调度、上下文组装和 ACP Server 调用由本机 App 内运行时协调；MindClaw 自身不上传消息到 MindClaw 云端。
5. **Explicit over magic**：默认 Agent + SlashCommand 显式选择优先于自动 Agent 路由。
6. **Observable by default**：每次执行都应能追踪到 channel、conversation、agent、skill、acp_server 和状态。
