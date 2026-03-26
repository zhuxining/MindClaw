# MindClaw 技术架构设计

> 拆分自 architecture.md，完整索引见 [README.md](./README.md)

## 四、核心数据流

### 4.1 捕获流（Capture Flow）

```
用户在 QuickCapture 输入
  → invoke("capture_submit", { raw, source: "desktop" })
  → Command → CaptureService.submit(): 写入 capture_queue 表，返回 CaptureItem
  → SubAgent 异步: Haiku 分类 → { task | thought | feeling | link }
  → CaptureService.update_route(): 更新 suggested_route
  → 前端: InboxPage 轮询/监听 → 展示待审核项
  → 用户确认 → invoke("capture_confirm_route")
  → Command → CaptureService.confirm(): 按路由委托对应 Service
      ├── task    → TaskService.create()
      ├── thought → KnowledgeService.create() → vault/knowledge/ 草稿
      ├── feeling → DailyService.append() → vault/daily/当日.md
      └── link    → CaptureService.mark_processed()
```

### 4.2 对话流（Conversation Flow）

消息经过 Channel 抽象层统一处理，无论来源是桌面 UI 还是 Telegram Bot：

```
用户发送消息（桌面 UI / Telegram / Feishu）
  → Channel 将平台消息转为 ChannelMessage { sender, content, source, mode }
  → Bus.publish_inbound() → AgentLoop 消费
      ├─ UserIdentityResolver: 跨通道身份统一（→ "owner"）
      ├─ SessionManager: 按统一身份加载/创建 Session
      ├─ ContextBuilder: 组装 prompt
      │    [1] 基础人格 + 模式指令
      │    [2] 用户角色上下文（user_roles 表）
      │    [3] KnowledgeService.search_with_rerank(): L0 粗筛 → L1 注入
      │    [4] 压缩对话历史（近 5 轮完整 + 早期摘要）
      │    [5] Memory.unsurfaced_observations(): 记忆召回
      │    [6] 用户消息
      ├─ call_with_tools(): 两阶段流式策略
      │    stream_with_tool_detection(): 解析 SSE 事件
      │    text → 立即推送 Bus.outbound（用户可见）
      │    tool_use → 静默累积 → 执行工具 → 再次流式调用
      ├─ PostProcess: 写入 Memory + 派发 SubAgent 任务
      └─ SessionManager: 追加消息对，触发裁剪
  → Bus.outbound → run_outbound_dispatcher() → Channel.send()
      Desktop: Tauri Event → 前端 useConversation 实时渲染
      Telegram: sendMessage API → 用户手机
```

### 4.3 日记流（Daily Flow）

```
DailyPage 挂载，传入今日日期
  → invoke("daily_get", { date: "2026-03-26" })
  → Command → DailyService.get(): 读取 vault/daily/2026-03-26.md（不存在则模板创建）
  → Command → TaskService.list_by_date(): 查询关联任务
  → 返回 DailyNote { markdown, tasks: Vec<Task> }
  → 前端: 渲染 Markdown + 嵌入 TaskCard 组件
  → 用户编辑 → invoke("daily_save") → Command → DailyService.save()
  → 用户切换任务状态 → invoke("task_update") → Command → TaskService.update()
```

---
