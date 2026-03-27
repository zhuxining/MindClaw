# MindClaw 技术架构设计

本目录包含 MindClaw 的完整技术架构文档，按板块拆分便于查阅和维护。

## 文档索引

| 文件 | 内容 | 说明 |
|------|------|------|
| [00-overview.md](./00-overview.md) | 系统总览 | 技术栈、架构分层图、桌面端即服务器理念 |
| [01-directory-structure.md](./01-directory-structure.md) | 目录结构 | 代码目录（src-tauri/ + src/）与用户数据目录 |
| [02-command-architecture.md](./02-command-architecture.md) | 三层命令架构 | Web Commands / Agent Commands / CLI Commands |
| [03-data-flows.md](./03-data-flows.md) | 核心数据流 | Agent 输入路由、对话流、日记流 |
| [04-storage.md](./04-storage.md) | 存储架构 | SQLite 表结构、Markdown 同步、三级索引、RAG 检索 |
| [05-agent.md](./05-agent.md) | Agent 架构 | Channel / Bus / AgentLoop / Provider / Tools / Memory / Gateway / Cron |
| [06-security.md](./06-security.md) | 安全架构 | CSP、私密区隔离、Capabilities、树洞模式 |
| [07-mvp-scope.md](./07-mvp-scope.md) | MVP 范围 | Phase 1 / Phase 1 后期 / Phase 2 功能划分 |
| [08-dependencies.md](./08-dependencies.md) | 技术依赖 | Rust Cargo.toml 与前端 package.json 依赖清单 |

## 阅读建议

- **快速了解全貌**：先读 [00-overview.md](./00-overview.md) 的架构分层图
- **开发新功能**：查看 [02-command-architecture.md](./02-command-architecture.md) 了解命令注册模式，参考 [01-directory-structure.md](./01-directory-structure.md) 确定文件位置
- **存储相关**：[04-storage.md](./04-storage.md) 包含所有表结构和 Markdown 同步规则
- **Agent 开发**：[05-agent.md](./05-agent.md) 是最详细的模块，包含 Channel、Bus、Loop、Provider、Tools、Memory 等全部子系统
