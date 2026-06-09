> **Status**: `draft`

# 产品蓝图：消息渠道与 Agent 调度

## Part 1：产品定位

- **目标用户**：重度 LLM 用户（开发者、研究员、知识工作者），在飞书等 IM 平台接收大量信息，需要本地 Agent 自动处理
- **核心问题**：IM 渠道（飞书等）的消息无法自动流入本地 Agent 处理；敏感数据不敢上传云端 AI 服务；跨渠道消息分散，缺乏统一 Agent 调度入口
- **核心方式**：通过 `im_channel` 接入不同 IM 渠道 → `gateway` 进行标准化和安全控制 → `message_bus` 统一路由 → `acp` 执行智能处理 → 结果回写到渠道
- **长期价值**：用户以 MindClaw 为中枢，所有 IM 渠道的消息自动汇聚到本地 Agent，实现"消息入 → Gateway 标准化 → Bus 路由 → ACP 处理 → 结果出"的闭环，数据永不离开设备

## Part 2：核心场景

### 场景 1：飞书消息自动摘要

用户在飞书中收到大量群聊消息，下班后打开 MindClaw，Agent 已自动将未读消息按优先级摘要，用户快速了解要点。

**数据流**：飞书 API → `im_channel/feishu` → `gateway` → `message_bus` → `acp` → 摘要结果 → `gateway` → `im_channel` → 飞书回复

### 场景 2：飞书指令触发 Agent 任务

用户在飞书中 @机器人 发送"帮我整理本周的会议纪要"，消息经 `im_channel` → `gateway` → `message_bus` → `acp`，ACP 自动完成任务并将结果回复到飞书。

### 场景 3：多渠道消息聚合处理

（演进方向）用户同时使用飞书和 Telegram，所有消息经 `gateway` 标准化后流入同一 `message_bus`，ACP 根据消息来源和内容智能分流处理。

## Part 3：产品原则

1. **本地优先**：所有消息处理和 Agent 调用均在本地完成，不上传云端
2. **分层解耦**：`im_channel` / `gateway` / `message_bus` / `acp` 四层独立，新增渠道不影响下游
3. **渠道无关**：`gateway` 提供统一消息抽象，具体渠道作为可插拔的 Adapter
4. **Agent 无关**：通过 ACP 标准协议调用 Agent，不绑定特定 Agent 实现
5. **安全第一**：飞书 API Token 等密钥存储在 Stronghold 中，`gateway` 负责鉴权限流
6. **渐进增强**：先飞书单渠道跑通全链路，再扩展多渠道

## Part 4：价值边界

### 明确解决

- 飞书消息自动流入本地 Agent 处理（`im_channel` → `gateway` → `message_bus` → `acp`）
- 消息统一标准化和路由（`gateway` + `message_bus`）
- Agent 处理结果回写到飞书（`acp` → `message_bus` → `gateway` → `im_channel`）
- 本地 Agent 通过 ACP 协议标准化调用
- 安全控制：鉴权、限流、凭证管理（`gateway` 层）

### 明确不解决

- 飞书消息的云端同步/备份
- 团队协作和权限管理
- 飞书之外的其他渠道对接（v1 阶段，架构已预留扩展点）
- Agent 本身的实现（Agent 是外部/内置服务，本蓝图只解决调度层）
- 消息的实时推送通知（v1 采用轮询，v2 考虑 WebSocket）

## Part 5：演进方向

### v1.0 — 飞书单渠道闭环

- `im_channel/feishu`：飞书 API 适配
- `gateway`：渠道注册、鉴权、标准化
- `message_bus`：消息路由核心
- `acp`：Agent 调用执行
- 基础 UI：查看消息流和 Agent 处理结果

### v1.1 — Agent 管理增强

- 多 Agent 支持和路由规则
- Agent 处理状态和日志可视化
- `acp` 技能插件系统

### v2.0 — 多渠道扩展

- Telegram / 钉钉 / Slack Adapter
- 渠道优先级和分流规则
- `gateway` 增强：富媒体处理、会话管理

## Part 6：关键假设

| 假设 | 验证方式 | 失效后果 |
|------|---------|---------|
| 飞书用户与 LLM 重度用户重合度高 | 社区调研 | 降低飞书优先级 |
| ACP 协议生态持续发展 | 跟踪社区活跃度 | 替换为自定义 Agent 协议 |
| 飞书开放平台 API 满足需求 | 原型验证 | 增加中间层适配 |
| 本地 Agent 处理延迟用户可接受 | 内测反馈 | 增加流式响应和进度反馈 |

## Part 7：非目标

- 不做 SaaS 化的云服务
- 不做飞书机器人商店/应用市场
- 不做消息的历史归档和全文搜索
- 不做团队协作功能（多用户、权限）
