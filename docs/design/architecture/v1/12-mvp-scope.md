# MindClaw 技术架构设计 — MVP 范围

> 完整架构文档索引见 [README.md](./README.md)

## Phase 1（MVP）包含

| 能力                  | 模块                                | 说明                                           |
| --------------------- | ----------------------------------- | ---------------------------------------------- |
| Agent 对话 + 自然路由 | agent, tools::operations            | 对话中理解意图，通过 operations 工具直接执行   |
| 日记视图 + 嵌入式任务 | daily, tasks                        | Daily Note 为锚点，任务一等公民                |
| 基础对话              | conversation, agent::core           | 陪伴 + 知识两种模式                            |
| 知识库浏览与搜索      | knowledge                           | 关键词搜索（FTS5）                             |
| 设置 + API Key        | settings, keychain                  | BYOK Claude API                                |
| SQLite 存储           | storage::database                   | Schema 迁移、基础 CRUD                         |
| Markdown 读写         | storage::markdown                   | 日记和知识笔记                                 |
| Provider 层           | providers::registry + openai_compat | 多提供商支持（OpenAI/DeepSeek 等，配置驱动）   |
| 基础工具              | tools::search, file_ops             | 知识库搜索 + Markdown 文件操作                 |
| Cron 基础任务         | cron                                | index_rebuild, daily_summary, resource_process |
| Heartbeat             | heartbeat                           | 系统健康检测（DB、Vault、API Key）             |
| 统一错误处理          | error.rs                            | AppError → 前端展示                            |
| 结构化日志            | tracing crate                       | 开发调试                                       |
| 可插拔上下文管线      | agent::context                      | ContextSource trait + 内置 5 源                |
| Agent Hooks（Rust）   | agent::hooks                        | HookHandler trait + HookRegistry               |
| SubAgent 任务注册表   | agent::sub_agent                    | SubAgentTask trait + Registry                  |

### Phase 1 后期

| 能力                    | 模块               | 说明                                    |
| ----------------------- | ------------------ | --------------------------------------- |
| Telegram Bot 通道       | channels::telegram | 移动端对话通道（最低开发成本）          |
| Gateway 基础            | gateway::api       | Webhook 接收（Telegram）+ 简单 chat API |
| Gateway 认证            | gateway::auth      | Bearer Token + Telegram 签名验证        |
| Command Hooks           | agent::hooks       | settings.json 命令钩子配置              |
| 自定义 ContextSource    | agent::context     | Skills 注册的上下文源                   |
| Skills 系统（built-in） | tools::skills      | SkillRegistry + 标准技能                |

### Phase 2（延期）

| 能力                   | 说明                                             |
| ---------------------- | ------------------------------------------------ |
| sqlite-vss 向量搜索    | MVP 阶段用 FTS5 关键词搜索替代                   |
| 反思 / 挑战 / 树洞模式 | 陪伴 + 知识模式验证后再扩展                      |
| Layer 3 认知循环       | 需要积累足够数据才有意义                         |
| 分析 / 写作工具        | tools::analysis, writer，需 Agent 能力成熟后再加 |
| 角色模版冷启动         | 可手动设置角色，模版系统后补                     |
| 飞书 Bot 通道          | channels::feishu                                 |
| Gateway WebSocket      | ws.rs，实时对话                                  |
| PWA 移动查看           | Gateway 提供静态文件服务                         |
| Tailscale 远程穿透     | 移动端远程接入                                   |
| JSONL 冷归档           | history_prune cron，90 天后才需要                |
| Cron 高级任务          | knowledge_review, memory_surface                 |
| Agent 主动推送         | 异步日志与浮出机制                               |
| 知识图谱可视化         | —                                                |
| 本地 Embedding 模型    | 用 API embedding 或延期向量搜索                  |
| Ollama 本地模型        | ProviderRegistry.register() 自定义配置           |
| Skills 外部加载/WASM   | 动态加载外部技能包                               |

---
