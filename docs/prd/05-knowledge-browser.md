> **Status**: `draft`

# 知识库浏览

→ 架构关联：[08-desktop-frontend.md](../design/arch/08-desktop-frontend.md)  
→ 父文档：[00-overview.md](00-overview.md)

## 背景与目标

**背景**：Vault 根目录下有四个独立文件夹：`daily/`（日记）、`tasks/`（任务）、`private/`（私密，Agent 不可见）、`source/`（外部资源：链接/PDF）。Vault Tab 展示除隐藏文件（`.` 开头）外的全部内容，是用户浏览 Vault 整体的入口。

**目标**：定义用户在 Vault、Private、Source 三个 Tab 下浏览、搜索、查看笔记和资源的行为。

## 功能描述

### US-01 浏览 Vault 全库（Vault）

作为用户，我希望在 Vault Tab 下浏览整个 Vault 的文件结构，以便找到任意已有内容。

**验收标准**：

Given 用户点击左侧 Tab 中的"Vault"  
When Tab 切换完成  
Then 左侧目录树显示完整 Vault 文件树（以 `{vault}/` 为根，包含 `daily/`、`tasks/`、`private/`、`source/` 等所有文件夹，排除名称以 `.` 开头的隐藏文件和文件夹）

Given 用户点击目录树中的某个 `.md` 文件  
When 文件点击触发  
Then 中央区域以只读模式显示该文件的 Markdown 渲染内容

Given 用户点击目录树中的非 `.md` 文件（如 `.pdf`、图片）  
When 文件点击触发  
Then 中央区域以对应格式预览该文件（PDF 用 PDF 查看器，图片直接展示）

- [ ] 文件夹默认折叠，点击展开；`daily/`、`tasks/`、`private/`、`source/` 四个根文件夹默认展开
- [ ] 目录树按文件名字母顺序排列（文件夹优先于文件）

**优先级**：P1

---

### US-02 笔记全文搜索

作为用户，我希望通过关键词搜索 Vault 中的笔记，以便在大量笔记中快速定位内容。

**验收标准**：

Given 用户在搜索框中输入关键词（≥ 2 个字符）  
When 用户停止输入 300ms 后  
Then 中央区域显示搜索结果列表，每条结果显示：文件标题、匹配的文本片段（高亮关键词）、文件路径

Given 搜索结果列表已显示  
When 用户点击某条结果  
Then 中央区域切换到该笔记的只读视图，并滚动到第一个关键词出现的位置（高亮显示）

- [ ] 搜索结果最多显示 20 条
- [ ] 无结果时显示"未找到相关笔记"提示，不显示空列表
- [ ] 搜索框清空时恢复显示目录树

**优先级**：P1

---

### US-03 查看私密区笔记（Private）

作为用户，我希望在 Private Tab 下查看私密笔记，以便访问不对 Agent 开放的个人内容。

**验收标准**：

Given 用户点击左侧 Tab 中的"Private"  
When Tab 切换完成  
Then 左侧目录树显示 `{vault}/private/` 目录下的文件结构

Given 用户打开 Private Tab 下的某个笔记  
When 笔记加载完成  
Then 中央区域显示该笔记内容，编辑器工具栏不显示"分享"或"发送给 Agent"等操作

- [ ] Private 目录下的所有文件内容不通过任何接口传递给 Agent（后端 PathGuard 执行隔离，前端不显示"发送"类操作）
- [ ] Private Tab 外观与其他 Tab 无视觉区别，不做额外的锁图标等隐私强调

**优先级**：P2

---

### US-04 查看和预览资源（Source）

作为用户，我希望在 Source Tab 下查看收集的链接和 PDF，以便访问外部参考资料。

**验收标准**：

Given 用户点击左侧 Tab 中的"Source"  
When Tab 切换完成  
Then 左侧目录树显示按类型分组的资源列表（链接 / PDF）

Given 用户点击某个链接类型的资源  
When 资源加载触发  
Then 中央区域使用内嵌网页视图（WebView）打开该 URL，显示网页内容

Given 用户点击某个 PDF 类型的资源  
When 资源加载触发  
Then 中央区域使用 PDF 查看器显示该 PDF 文件内容

- [ ] 网页预览加载超过 5 秒时显示加载中状态，超过 15 秒显示"加载超时"提示
- [ ] 网页预览提供"在浏览器中打开"按钮，点击后用系统浏览器打开该 URL

**优先级**：P2

---

### US-05 查看和编辑笔记

作为用户，我希望点击目录树中的笔记后直接查看并编辑内容，以便无缝阅读和修改笔记。

**验收标准**：

Given 用户在 Vault 或 Private Tab 下点击了某个 `.md` 文件  
When 笔记在中央区域显示  
Then 中央区域打开 Milkdown 编辑器，渲染该笔记内容；点击任意位置可直接编辑

- [ ] 笔记内容变更后 1 秒内自动保存到对应 Markdown 文件
- [ ] 编辑器工具栏支持：标题、加粗、斜体、列表、代码块

**优先级**：P1

## 范围界定

**In Scope**：

- Vault Tab：Vault 全文件树浏览（含 `daily/`、`tasks/`、`private/`、`source/` 等所有内容，排除隐藏文件）+ 关键词搜索 + 只读查看
- Private Tab：`private/` 目录浏览 + 只读查看（Agent 隔离）
- Source Tab：`source/` 资源列表 + 链接 WebView 预览 + PDF 预览
- 笔记 Milkdown 编辑器（点击即编辑，自动保存）

**Out of Scope**：

- 笔记创建（在知识库中新建笔记）：Phase 2 功能；Phase 1 通过 Chat 对话触发 Agent 创建知识笔记
- Wikilink 导航（`[[笔记名]]` 点击跳转）：Phase 2，需要链接解析系统
- 图谱视图：笔记关系图谱是独立的可视化功能，排除
- 资源添加（手动添加链接/PDF）：Phase 2，需要资源管理系统
- 笔记创建（在 Vault 中新建笔记）：Phase 2，通过 Chat 触发 Agent 创建知识笔记

## 非功能需求

- 搜索结果首屏显示时间 ≤ 1 秒（从用户停止输入算起）
- 笔记只读视图加载时间 ≤ 500ms（本地文件读取 + Markdown 渲染）
- WebView 预览失败时（网络不可达）显示错误提示，不显示空白区域
