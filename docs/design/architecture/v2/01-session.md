# Session

## Session Is The Persistent Conversation Unit

V2 去掉显式角色层概念。

系统不再把“角色选择”作为 Agent Core 的一等状态，也不允许会话依赖角色切换来改变核心行为。

主行为来源收敛为三部分：

- 固定核心 prompt
- 当前用户消息
- 会话级设置与历史

## Session Model

```rust
pub struct Session {
    pub id: SessionId,
    pub history: Vec<TurnRecord>,
    pub memory_refs: Vec<MemoryRef>,
    pub settings: SessionSettings,
}
```

```rust
pub struct SessionSettings {
    pub response_style: Option<String>,
    pub locale: Option<String>,
    pub feature_flags: Vec<String>,
}
```

`Session` 是 Agent Core 的持久上下文单元，负责保存三种长期状态：

- 已提交历史
- 该会话可见的记忆引用范围
- 会话级稳定设置

## Session Rules

- 同一个 session 任意时刻最多一个活跃 run
- session settings 在 run 开始前读取，run 中不变化
- 历史只在 run 成功或明确失败后追加
- 后台任务回注到原 session，而不是创建隐式新会话

## Session Settings

V2 允许通过显式入口写入会话设置：

- 创建会话时附带默认设置
- 用户显式修改会话设置
- UI 或 channel 在入口层附带运行偏好

不允许以下行为：

- 模型根据最近消息隐式修改会话设置
- 工具调用偷偷改变会话设置
- 后台任务完成后覆盖当前会话设置

## History Model

V2 历史模型只保留一条规则：

`Session history = committed user turns + committed assistant turns`

run 内产生的中间内容只属于 run-local transcript，不直接进入 session history：

- 流式增量输出
- 工具执行中间态
- 发现阶段的外部工具元数据
- 后台任务运行日志

## No Explicit Role Layer

V2 不再定义以下概念：

- 显式角色选择
- 会话级角色绑定
- 会话级角色切换
- 由模型自行决定当前扮演角色

如果未来需要产品级角色系统，应作为独立产品层能力重新设计，而不是回到 Agent Core 主干。
