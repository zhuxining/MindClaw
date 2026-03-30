# MindClaw 技术架构设计 — Gateway 层

> 完整架构文档索引见 [README.md](./README.md) | Agent 核心见 [03-agent-loop.md](./03-agent-loop.md)

## Gateway Layer — HTTP/WebSocket 服务

Gateway 为移动端 PWA 提供静态文件和 API，为 Webhook 通道提供接入点。通过 Bus 解耦，不直接引用 Agent。

| 端点 | 方法 | 说明 | Phase |
|------|------|------|-------|
| `/api/chat` | POST | 发送消息，返回 Agent 响应 | Phase 1 后期 |
| `/api/daily/:date` | GET | 获取日记内容 | Phase 2 |
| `/api/knowledge` | GET | 知识库搜索 | Phase 2 |
| `/api/tasks` | GET | 任务列表 | Phase 2 |
| `/ws/chat` | WS | WebSocket 实时对话 | Phase 2 |
| `/webhook/telegram` | POST | Telegram Bot Webhook | Phase 1 后期 |
| `/webhook/feishu` | POST | 飞书 Bot Webhook | Phase 2 |
| `/` | GET | PWA 静态文件服务 | Phase 2 |

### 认证

| 场景 | 认证方式 | 说明 |
|------|---------|------|
| 本地 WiFi（PWA /api/*） | Bearer Token | Token 存储在 OS Keychain |
| Tailscale 远程接入 | 双重保护 | Bearer Token + Tailscale 身份验证 |
| Webhook（Telegram/Feishu） | 平台签名验证 | 验证平台签名，**不需要** Bearer Token |
| WebSocket（/ws/chat） | Bearer Token | 连接时认证，之后保持会话 |
