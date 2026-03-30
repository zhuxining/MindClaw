# MindClaw 技术架构设计 — Cron 定时任务

> 完整架构文档索引见 [README.md](./README.md) | Agent 核心见 [03-agent-loop.md](./03-agent-loop.md)

## Cron — 定时任务调度

| 任务 | 默认频率 | 说明 | Phase |
|------|---------|------|-------|
| `daily_summary` | 每日 22:00 | 生成当日回顾，写入日记 | MVP |
| `resource_process` | 每 5 分钟 | 处理 pending 资源（解析 + 结晶） | MVP |
| `history_prune` | 每日 03:00 | 压缩旧对话历史，超 90 天转冷归档 | Phase 2 |
| `knowledge_review` | 每周日 10:00 | 回顾知识库，发现新关联 | Phase 2 |
| `index_rebuild` | 每日 04:00 | 增量重建 Markdown → SQLite 索引 | MVP |
| `memory_surface` | 每日 09:00 | 检查未浮出记忆的浮出时机 | Phase 2 |
| `heartbeat_check` | 每 30 秒 | 系统健康检测 | MVP |

基于 `tokio-cron-scheduler` 精确调度。
