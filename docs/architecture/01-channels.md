> **Status**: `active`

# Channels — 多通道架构

---

## § 职责定位

Channels 层负责接收来自各平台的用户输入并推送 Agent 响应，不负责消息的内容处理、路由决策或 Agent 的执行控制。

---

## § 核心原则

**通道无关性**：所有通道通过 MessageBus 与 AgentLoop 交互，Channel 实现不持有 AgentLoop 引用，AgentLoop 实现不持有任何 Channel 引用。

**白名单边界**：非白名单的消息在 Channel 层拒绝，不进入 MessageBus，不触发 AgentLoop 的任何处理。

---

## § 边界与实体

**输入**：各平台的原始用户消息（Tauri IPC 事件、Telegram Webhook 推送、飞书事件推送）。

**输出**：格式化后的响应文本和流式增量，推送给对应平台用户。

**核心实体**：

**Channel trait**：所有通道实现的统一接口契约。

```
fn name(&self) -> &str
fn channel_type(&self) -> ChannelType
async fn start(&mut self) -> Result<()>
async fn stop(&mut self) -> Result<()>
async fn send(&self, message: &OutboundMessage) -> Result<()>
fn check_sender(&self, sender_id: &str) -> bool
```

**ChannelManager**：所有 Channel 实例的生命周期管理器和出站消息路由器。
关键属性：通道名称到 Channel 实例的映射（`HashMap<String, Box<dyn Channel>>`）。
关系：由 AppRuntime 创建；持续消费 MessageBus 的 OutboundMessage，按 `session_key` 前缀匹配目标 Channel，调用 `channel.send()`。

**ChannelConfig**：单个通道的配置声明，由 `channels.yaml` 加载，运行期不可变。
关键属性：通道类型（Desktop / Telegram / Feishu）、实例名称（唯一，构成 `session_key` 的前缀）、启用状态、发送者白名单、平台特定凭证引用。
关系：AppRuntime 启动时读取，用于构建对应的 Channel 实现实例。

---

## § 三个通道实现

**Desktop（Tauri）**：

- 入站：前端通过 `invoke("send_message")` 调用 Tauri 命令，命令层将消息发布到 MessageBus。
- 出站：ChannelManager 调用 Tauri 的 `emit` 将 OutboundMessage 推送到前端，前端监听事件更新 UI。
- Session Key 格式：`desktop:default`（单窗口），多窗口时以窗口 ID 区分。

**Telegram Bot**：

- 入站：Telegram 平台向配置的 Webhook URL 推送消息，Gateway 接收后转发到 MessageBus。
- 出站：ChannelManager 调用 Telegram Bot API `sendMessage` 或流式 `editMessageText`。
- Session Key 格式：`telegram_<name>:<chat_id>`，支持同一平台多个 Bot 实例。

**飞书 Bot**：

- 入站：飞书平台推送事件，签名验证后提取消息内容，发布到 MessageBus。
- 出站：调用飞书消息 API 回复，支持富文本格式。
- Session Key 格式：`feishu_<name>:<open_chat_id>`。

---

## § 关键流程

1. AppRuntime 启动时读取 `channels.yaml`，为每个 `enabled: true` 的配置构建对应 Channel 实例。
2. 各 Channel 调用 `start()` 启动监听（Telegram 注册 Webhook，飞书注册事件，Desktop 注册 Tauri 命令处理器）。
3. Channel 接收到用户消息，调用 `check_sender()` 验证发送者白名单；非白名单消息直接丢弃。
4. 验证通过后，Channel 构建 InboundMessage（`session_key = "{name}:{chat_id}"`）发布到 MessageBus。
5. ChannelManager 在独立后台任务中持续消费 OutboundMessage，按 `session_key` 前缀路由到对应 Channel。
6. Channel 的 `send()` 将响应内容推送给平台用户（流式增量逐个推送，或完整消息一次发送）。

---

## § 设计决策与权衡

| 决策问题 | 选择 | 放弃的替代方案 | 理由 |
|---------|------|--------------|------|
| Channel 如何与 Agent 通信？ | 通过 MessageBus 异步队列，互不持有引用 | Channel 直接调用 AgentLoop 方法 | MessageBus 解耦两侧生命周期：Channel 重启不影响 AgentLoop，AgentLoop 繁忙时 Channel 不阻塞 |
| 通道配置如何管理？ | `channels.yaml` 声明式配置文件 | 代码内硬编码配置 | 用户可不修改代码地启用/禁用通道和配置多实例；配置文件在 git 中可见但凭证通过环境变量引用 |
| 同一平台是否支持多实例？ | 支持（通过不同 `name` 字段，对应不同 session_key 前缀） | 每个平台唯一实例 | 用户同时运行个人 Telegram Bot 和团队 Telegram Bot 时，需要不同白名单和独立 session |
| 发送者白名单在哪层控制？ | Channel 层（进入 MessageBus 之前）| AgentLoop 层 | 权限验证尽量靠前；未授权消息不进入 MessageBus，减少无效处理，避免日志污染 |
| 敏感凭证（Bot Token）如何配置？ | `channels.yaml` 中引用 `${ENV_VAR}` 环境变量 | 直接写入配置文件 | 凭证不应进入版本控制；环境变量是 12-Factor App 的标准实践，与容器部署兼容 |
| 热重载通道配置是否支持？ | 不支持，需要重启应用 | 支持运行时热重载 | 热重载需要优雅停止/启动 Channel，增加状态管理复杂度；个人应用重启成本低 |
| Desktop 通道的 Session Key 如何生成？ | `desktop:{window_id}` | 固定 `desktop:default` | 支持多窗口场景；单窗口时 window_id 为 "default"，多窗口时每个窗口独立会话 |
| 如何处理平台特定的消息格式？ | Channel 层转换为统一 InboundMessage | 统一格式透传 | Telegram、飞书的原始消息格式不同；Channel 层转换使 AgentLoop 无需感知平台差异 |
| 如何支持新通道类型？ | 实现 Channel trait，在 channels.yaml 中添加配置 | 修改 AgentLoop 代码 | trait 抽象使新增通道无需改动核心代码；配置驱动使通道启用/禁用无需重新编译 |
| 通道配置变更如何生效？ | 重启应用 | 运行时热重载 | 热重载需要处理状态迁移和连接管理，增加复杂度；个人应用重启成本低，重启是最简单可靠的方式 |
