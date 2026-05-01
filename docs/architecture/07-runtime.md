> **Status**: `active`

# Runtime — 统一运行时

---

## § 职责定位

`AppRuntime` 负责统一初始化顺序、依赖注入和三入口（Desktop、CLI、Gateway）的适配，不负责任何具体业务逻辑。

---

## § 核心原则

**单一启动路径**：所有入口都通过同一 Builder 初始化，避免组件版本、配置和状态漂移。

**Definition / Runtime 一次注入完成**：Agent Registry、Model Router、Provider Registry、Agent Runner、Agent Loop 由 Runtime Builder 统一组装。

---

## § 核心对象

**AppRuntime**

- 持有核心组件强引用
- 暴露 `services()`、`agent_loop()`、`bus()`
- 提供统一 `start()` / `shutdown()`

**AppRuntimeBuilder**

- 按依赖顺序初始化 Storage、Services、Providers、Runtime Core、Channels

**AppConfig**

- 全局静态配置
- 在启动时加载，运行期间不可变

---

## § 启动序列

1. 读取配置文件和环境变量，构建 `AppConfig`
2. 初始化 Storage
3. 初始化 ServiceContainer
4. 初始化 ProviderRegistry，并解析 `AgentModelSet` 主模型/轻量模型
5. 初始化共享 AgentRunner
6. 初始化 AgentRegistry 与 ModelRouter
7. 初始化 AgentSpawnDispatcher 所需共享依赖
8. 初始化 ContextPipeline 与 SessionManager
9. 注入 AgentLoop
10. 每次 run 按需构建 Rig tool scope 和 MCP tools
11. 初始化 ChannelManager
12. 返回 AppRuntime，并由入口调用 `start()`

---

## § 三入口适配

```text
Desktop
-> AppRuntime
-> AgentLoop + Services

CLI
-> AppRuntime
-> Services or one-shot AgentRunner

Gateway
-> AppRuntime
-> AgentLoop + Services + HTTP handlers
```

---

## § Runtime 与 Agent Runtime 的关系

第 7 章回答“这些组件如何被装起来”，第 3 章回答“Agent Runtime 自身如何分层运行”。

因此：

- 第 3 章关注职责边界
- 第 7 章关注构造顺序和依赖注入

---

## § 设计决策与权衡

| 决策问题 | 选择 | 放弃的替代方案 | 理由 |
|---------|------|--------------|------|
| 三个入口如何共享组件？ | `AppRuntime` 统一持有 | 每个入口各自构建 | 避免重复初始化与配置漂移 |
| Agent Runtime 如何注入？ | Builder 统一创建 Registry、Router、Runner、Loop | 在入口中零散拼装 | 依赖关系集中更清晰，也更便于测试 |
| CLI 是否必须启动 AgentLoop？ | 否 | 与 Desktop 一样完整启动 | CLI 可以只做一次性执行，降低启动开销 |
