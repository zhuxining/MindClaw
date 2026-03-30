# MindClaw 技术架构设计

> 完整架构文档索引见 [README.md](./README.md)

## 安全架构

### CSP 策略

`tauri.conf.json` 中设置 Content Security Policy：

```json
{
  "app": {
    "security": {
      "csp": "default-src 'self'; script-src 'self'; connect-src 'self' https://api.anthropic.com"
    }
  }
}
```

仅允许本地内容和 Claude API 请求。

### 私密区隔离

- `vault/private/` 路径下的所有文件，Agent 不可见
- Rust storage 模块在读取 Markdown 供 Agent 使用时，显式拒绝 `private/` 路径前缀
- 私密区内容永不进入 SQLite 索引、不参与 RAG 检索、不出现在任何 IPC 响应中

### Tauri Capabilities

`capabilities/default.json` 需声明：

```json
{
  "identifier": "default",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "opener:default",
    "fs:read-files",
    "fs:write-files"
  ]
}
```

文件系统权限限定在 vault 和 data 目录范围内。

### 树洞模式特殊处理

- 原始消息保留时间更短（用户可配置或手动清除）
- 摘要只存 `memories` 表（category='cases'），不生成人类可读摘要
- 内容永不进入共有知识库

---
