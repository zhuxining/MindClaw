# MindClaw 技术架构设计

> 拆分自 architecture.md，完整索引见 [README.md](./README.md)

## 八、MVP 范围

### Phase 1（MVP）包含

| 能力 | 模块 | 说明 |
|------|------|------|
| 快速捕获 + Agent 路由 | capture, agent::router | Haiku 分类，人类审核确认 |
| 日记视图 + 嵌入式任务 | daily, tasks | Daily Note 为锚点，任务一等公民 |
| 基础对话 | conversation, agent::core | 陪伴 + 知识两种模式 |
| 知识库浏览与搜索 | knowledge | 关键词搜索（FTS5） |
| 设置 + API Key | settings, keychain | BYOK Claude API |
| SQLite 存储 | storage::database | Schema 迁移、基础 CRUD |
| Markdown 读写 | storage::markdown | 日记和知识笔记 |
| Provider 层 | providers::claude | Claude API Haiku/Sonnet 调用 |
| 基础工具 | tools::search, file_ops | 知识库搜索 + Markdown 文件操作 |
| Cron 基础任务 | cron | index_rebuild, daily_summary |
| Heartbeat | heartbeat | 系统健康检测（DB、Vault、API Key） |
| 统一错误处理 | error.rs | AppError → 前端展示 |
| 结构化日志 | tracing crate | 开发调试 |

### Phase 1 后期

| 能力 | 模块 | 说明 |
|------|------|------|
| Telegram Bot 通道 | channels::telegram | 移动端对话通道（最低开发成本） |
| Gateway 基础 | gateway::api | Webhook 接收（Telegram）+ 简单 chat API |
| Gateway 认证 | gateway::auth | Bearer Token + Telegram 签名验证 |

### Phase 2（延期）

| 能力 | 说明 |
|------|------|
| sqlite-vss 向量搜索 | MVP 阶段用 FTS5 关键词搜索替代 |
| 反思 / 挑战 / 树洞模式 | 陪伴 + 知识模式验证后再扩展 |
| Layer 3 认知循环 | 需要积累足够数据才有意义 |
| 分析 / 写作工具 | tools::analysis, writer，需 Agent 能力成熟后再加 |
| 角色模版冷启动 | 可手动设置角色，模版系统后补 |
| 飞书 Bot 通道 | channels::feishu |
| Gateway WebSocket | ws.rs，实时对话 |
| PWA 移动查看 | Gateway 提供静态文件服务 |
| Tailscale 远程穿透 | 移动端远程接入 |
| JSONL 冷归档 | history_prune cron，90 天后才需要 |
| Cron 高级任务 | knowledge_review, memory_surface |
| Agent 主动推送 | 异步日志与浮出机制 |
| 知识图谱可视化 | — |
| 本地 Embedding 模型 | 用 API embedding 或延期向量搜索 |
| 多 Provider 支持 | OpenAI, Ollama 等 |

---
