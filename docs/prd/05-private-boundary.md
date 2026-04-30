> **Status**: `draft`

# Private 私密边界

→ 架构关联：[06-storage.md](../architecture/06-storage.md)
→ 父文档：[00-overview.md](00-overview.md)

## 背景与目标

**背景**：MindClaw 需要同时支持共享知识共建和个人私密记录。Private 不是独立数据库、独立 Vault 或独立 ContextURI 类型，而是当前 Vault 下的 `private/` 文件夹。该文件夹内容不进入 Agent 上下文、不参与记忆、不参与共有知识索引，并在界面上阻断相关操作。

**目标**：定义 Private 文件夹的打开、编辑、文件夹内搜索、隔离提示、误操作防护和 Agent 操作禁用状态。

## 功能描述

### US-01 打开 Private 工作域

作为用户，我希望从 Ribbon 打开 Private 工作域，以便访问只属于自己的私密内容。

**验收标准**：

Given 用户点击 Ribbon 中的 Private
When Private 工作域加载完成
Then 左侧面板显示 Private 文件树，中央内容区打开最近访问的私密笔记或 Private 首页

Given 用户从 Vault 中点击 Private 隔离入口
When 操作触发
Then 系统切换到 Private 工作域

- [ ] Private 文件树只显示 Private 范围内的内容。
- [ ] 当前打开的私密文件在左侧文件树中显示选中态。
- [ ] Private 文件路径位于当前 Vault 的 `private/` 文件夹下，不显示为独立 Vault 或独立数据库空间。
- [ ] Private 工作域标题显示“Private”。

**优先级**：P0

---

### US-02 编辑 Private 笔记

作为用户，我希望在 Private 中编辑 Markdown 笔记，以便保存不进入 Agent 协作范围的内容。

**验收标准**：

Given Private 笔记已打开
When 用户编辑正文
Then 编辑器实时显示修改后的 Markdown 内容

Given 用户停止输入 1 秒
When 当前内容存在变更
Then 系统保存 Private 笔记，并在状态栏显示已保存状态

Given 保存失败
When 编辑器收到失败结果
Then 状态栏显示保存失败，并保留未保存内容

- [ ] Private 笔记支持与 Vault 笔记相同的基础 Markdown 编辑能力。
- [ ] Private 笔记可包含 `tags`、`overview` 和 `confidence`，但这些字段只服务文件夹内浏览，不进入共有知识索引或 Agent 召回。
- [ ] Private 内容不显示 Add Context、Send to Agent、Create Memory、Save as Knowledge 操作。

**优先级**：P0

---

### US-03 搜索 Private 内容

作为用户，我希望在 `private/` 文件夹内部搜索私密笔记，以便快速找到个人内容。

**验收标准**：

Given 用户在 Private 搜索框输入不少于 2 个字符
When 输入停止 300ms
Then 搜索结果只包含 Private 范围内的笔记

Given 搜索结果列表已显示
When 用户点击结果项
Then 中央内容区打开对应 Private 笔记

- [ ] Private 搜索结果显示标题、路径、`tags`、`overview` 和 `confidence`。
- [ ] Private 搜索结果不出现在 Vault 搜索结果中。
- [ ] Private 搜索不依赖独立数据库索引；它只是在 `private/` 文件夹范围内执行本地文件搜索或即时解析。
- [ ] 无结果时显示空状态和清除搜索入口。

**优先级**：P1

---

### US-04 阻断 Agent 操作

作为用户，我希望系统在 Private 内容上禁用 Agent 相关操作，以便避免误把私密内容发送给 Agent。

**验收标准**：

Given 中央内容区激活 Private 笔记
When 右侧上下文面板展开
Then 面板不显示 Agent 引用、记忆生成、知识写入类操作

Given 用户处于 Agent Session
When 当前激活对象是 Private 笔记
Then Add Context 按钮禁用，并显示 Private 内容不可发送给 Agent 的说明

Given 用户尝试把 Private 内容移动到 Vault
When 操作触发
Then 系统显示确认对话框，说明移动后该内容会进入共享知识空间

**优先级**：P0

## 范围界定

**In Scope**：

- Private 工作域入口、文件树、Markdown 编辑、保存。
- `private/` 文件夹范围内搜索。
- Private 内容对 Agent 操作的界面阻断。
- Private 移动到 Vault 前的确认。

**Out of Scope**：

- 加密方案配置：加密属于安全和存储实现策略，不在功能需求中定义。
- 与外部应用共享 Private 内容：该行为会突破 Private 边界，需单独定义导出权限。
- Private 内容的 Agent 总结：该行为与隔离原则冲突。
- Private 与 Vault 的自动同步：自动同步会弱化用户确认边界。

## 非功能需求

- Private 搜索结果在输入停止后 1 秒内显示。
- Private 笔记保存防抖时间为 1 秒。
- Private 中 Agent 禁用状态必须在内容加载完成时同步显示。
