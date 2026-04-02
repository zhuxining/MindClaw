# MindClaw 技术架构设计 — 文档索引

> MindClaw 项目完整技术架构设计文档

## 文档索引

| 编号      | 文件                                                       | 内容                   | 层级             |
| --------- | ---------------------------------------------------------- | ---------------------- | ---------------- |
| 00        | [00-overview.md](./00-overview.md)                         | 系统总览               | 基础             |
| 01        | [01-directory-structure.md](./01-directory-structure.md)   | 代码结构               | 基础             |
| 02        | [02-data-flows.md](./02-data-flows.md)                     | 数据流程               | 基础             |
| **03**    | **[03-agent-loop.md](./03-agent-loop.md)**                 | **AgentLoop 核心**     | **Agent 核心**   |
| **03.01** | **[03.01-context.md](./03.01-context.md)**                 | **Context Pipeline**   | **Agent 核心**   |
| **03.02** | **[03.02-provider.md](./03.02-provider.md)**               | **Provider 层**        | **Agent 核心**   |
| **03.03** | **[03.03-tools.md](./03.03-tools.md)**                     | **Tools 层**           | **Agent 核心**   |
| **03.04** | **[03.04-memory.md](./03.04-memory.md)**                   | **Memory 层**          | **Agent 核心**   |
| **03.05** | **[03.05-services.md](./03.05-services.md)**               | **Services 层**        | **Agent 核心**   |
| **03.06** | **[03.06-subagent.md](./03.06-subagent.md)**               | **SubAgent**           | **Agent 核心**   |
| **04**    | **[04-channel.md](./04-channel.md)**                       | **Channel 层**         | **外围基础设施** |
| **05**    | **[05-gateway.md](./05-gateway.md)**                       | **Gateway 层**         | **外围基础设施** |
| **06**    | **[06-cron.md](./06-cron.md)**                             | **Cron 定时任务**      | **外围基础设施** |
| **07**    | **[07-heartbeat.md](./07-heartbeat.md)**                   | **Heartbeat 健康检测** | **外围基础设施** |
| 08        | [08-storage.md](./08-storage.md)                           | 存储架构               | 存储             |
| 09        | [09-command-architecture.md](./09-command-architecture.md) | 命令架构               | 其他             |
| **10**    | **[10-runtime.md](./10-runtime.md)**                       | **Runtime 模块**       | **基础设施**     |
| 11        | [11-security.md](./11-security.md)                         | 安全架构               | 其他             |
| 12        | [12-mvp-scope.md](./12-mvp-scope.md)                       | MVP 范围               | 其他             |
| 13        | [13-dependencies.md](./13-dependencies.md)                 | 技术依赖               | 其他             |
| 14        | [14-troubleshooting.md](./14-troubleshooting.md)           | 常见问题               | 其他             |

## 阅读建议

### 快速了解全貌

1. 先读 [00-overview.md](./00-overview.md) 的架构分层图
2. 再看 [02-data-flows.md](./02-data-flows.md) 理解数据流动
3. 接着读 [03-agent-loop.md](./03-agent-loop.md) 了解 Agent 核心

### Agent 开发

- **核心流程**：[03-agent-loop.md](./03-agent-loop.md) → [03.01-context.md](./03.01-context.md) → [03.02-provider.md](./03.02-provider.md)
- **工具扩展**：[03.03-tools.md](./03.03-tools.md)（含 MCP、Hooks、Skills）
- **业务逻辑**：[03.05-services.md](./03.05-services.md)
- **多 Agent 编排**：[03.06-subagent.md](./03.06-subagent.md)

### 外围设施

- **通道集成**：[04-channel.md](./04-channel.md)（Desktop/Telegram/Feishu）
- **API 服务**：[05-gateway.md](./05-gateway.md)（HTTP/WebSocket）
- **后台任务**：[06-cron.md](./06-cron.md)
- **健康监控**：[07-heartbeat.md](./07-heartbeat.md)

### 存储相关

- [08-storage.md](./08-storage.md) 包含所有表结构和 Markdown 同步规则

### 其他

- **Runtime**: [10-runtime.md](./10-runtime.md) — 统一运行时设计，三入口共享初始化
- **安全**：[11-security.md](./11-security.md)
- **依赖**：[13-dependencies.md](./13-dependencies.md)
- **故障排除**：[14-troubleshooting.md](./14-troubleshooting.md)
