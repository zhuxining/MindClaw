# 可追溯性矩阵

> 本文档维护产品场景、需求规格、架构文档与代码模块之间的对应关系。蓝图只描述产品方向；具体需求和实现状态应通过本文档与 PRD / migration 文档追踪。
>
> “当前重点模块”列记录当前代码目录；目标目录结构见 `docs/architecture/reference/directory-structure.md`。

## 产品场景追踪

| 场景 | 对应能力 | 需求规格 | 架构文档 | 当前重点模块 |
|------|----------|----------|----------|--------------|
| 把已有 ACP Server 接入 MindClaw | acp_client、ACP Server registry | `docs/prd/20-acp-native-feishu-agent-mvp.md` Story 1、Story 7 | `docs/architecture/30-acp-client.md` | `src-tauri/src/services/acp_client/` |
| 在 MindClaw 中管理 Agent 和 Skill | AgentStore、SkillStore、Agent-Skill 绑定 | `docs/prd/20-acp-native-feishu-agent-mvp.md` Story 2、Story 3；`docs/prd/10-agent-skill-slash-command.md` | `docs/architecture/40-agent-skill-command.md` | `src-tauri/src/services/agent/` |
| 通过 SlashCommand 显式选择执行方式 | SlashCommand、ConversationExecutionState | `docs/prd/10-agent-skill-slash-command.md` Story 5–8 | `docs/architecture/40-agent-skill-command.md` | `src-tauri/src/services/agent/command_parser.rs`、`src-tauri/src/services/agent/state_store.rs` |
| 把 Agent 接入 Feishu 消息流 | FeishuChannel、SessionDispatcher、GatewaySupervisor | `docs/prd/20-acp-native-feishu-agent-mvp.md` Story 4–8 | `docs/architecture/10-channel-gateway.md` | `src-tauri/src/services/im_channel/feishu/`、`src-tauri/src/services/session_dispatcher/`、`src-tauri/src/services/gateway/` |
| 窗口关闭到托盘后的本地驻留 | GatewaySupervisor、Tauri tray lifecycle | `docs/prd/20-acp-native-feishu-agent-mvp.md` Story 10 | `docs/architecture/00-overview.md`、`docs/architecture/10-channel-gateway.md` | `src-tauri/src/lib.rs`、`src-tauri/src/services/gateway/` |
| Desktop UI 作为控制台 | Tauri commands、Gateway API adapter、EventBus | `docs/prd/20-acp-native-feishu-agent-mvp.md` Story 9 | `docs/architecture/00-overview.md`、`docs/architecture/10-channel-gateway.md` | `src-tauri/src/commands/`、`src/components/`、`src-tauri/src/services/event_bus/` |

## MVP 需求追踪

| MVP Story | 主要能力 | 当前实现状态来源 |
|-----------|----------|------------------|
| Story 1：注册并测试默认 ACP Server | ACP Server registry、test connection | `docs/architecture/reference/migration.md` Phase 4 |
| Story 2：创建 Zero-config 默认 Agent | AgentStore、默认 Agent、Identity | `docs/architecture/reference/migration.md` Phase 2 |
| Story 3：配置最小 Skill 集合 | SkillStore、Agent-Skill 绑定、agent_context | `docs/architecture/reference/migration.md` Phase 2、Phase 5 |
| Story 4：配置 Feishu 渠道 | Feishu credentials、test connection | `docs/architecture/reference/migration.md` Phase 7 |
| Story 5：接收 Feishu 消息并转换为 ChannelMessage | Feishu converter、ChannelMessage、去重 | `docs/architecture/reference/migration.md` Phase 1、Phase 7 |
| Story 6：按会话顺序调度消息到默认 Agent | SessionDispatcher、ConversationExecutionState | `docs/architecture/reference/migration.md` Phase 2、Phase 3 |
| Story 7：通过 ACP Server 执行 Agent 请求 | acp_client、AgentResponse / DispatchResult | `docs/architecture/reference/migration.md` Phase 4 |
| Story 8：生成建议回复并支持用户确认发送 | AgentResponse、Channel reply、发送状态 | `docs/architecture/reference/migration.md` Phase 3、Phase 7 |
| Story 9：查看 MVP 控制台状态 | Tauri commands、EventBus、执行元数据 | `docs/architecture/reference/migration.md` Phase 3、Phase 6 |
| Story 10：托盘驻留与显式退出 | Tauri tray、GatewaySupervisor lifecycle | `docs/architecture/reference/migration.md` Phase 6 |

## 维护规则

- 蓝图新增或删除核心场景时，更新“产品场景追踪”。
- PRD 新增或重排 Story 时，更新“MVP 需求追踪”。
- 代码模块迁移时，先更新 `docs/architecture/reference/directory-structure.md`，再同步本文档的“当前重点模块”。
- 实现状态只写在 `docs/architecture/reference/migration.md`，本文档只引用状态来源，避免多处维护同一状态。
