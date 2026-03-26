# MindClaw 技术架构设计

> 拆分自 architecture.md，完整索引见 [README.md](./README.md)

## 九、技术依赖

### Rust（Cargo.toml 新增）

```toml
[dependencies]
# 存储
rusqlite = { version = "0.31", features = ["bundled", "fts5"] }

# 网络
reqwest = { version = "0.12", features = ["json", "stream"] }
axum = { version = "0.8", features = ["ws"] }           # Gateway HTTP/WS 服务
tower-http = { version = "0.6", features = ["cors", "fs"] }  # 静态文件 + CORS

# 异步运行时
tokio = { version = "1", features = ["full"] }
tokio-stream = "0.1"                                      # Stream 工具
tokio-util = "0.7"                                        # CancellationToken
async-trait = "0.1"

# 调度
tokio-cron-scheduler = "0.11"                             # 精确 cron 调度

# 安全
keyring = "3"

# 可观测性
tracing = "0.1"
tracing-subscriber = "0.3"

# 工具
chrono = { version = "0.4", features = ["serde"] }
serde_yaml = "0.9"
uuid = { version = "1", features = ["v4"] }
futures = "0.3"                                            # Stream trait
```

已有依赖：`tauri 2`, `tauri-plugin-opener 2`, `serde 1`, `serde_json 1`

### 前端（package.json 新增）

```json
{
  "zustand": "^5",
  "react-markdown": "^9",
  "date-fns": "^4",
  "@tauri-apps/plugin-fs": "^2"
}
```

路由方案：MVP 页面仅 5 个，使用 Zustand 状态管理当前页面即可，无需引入路由库。
样式方案：保持 CSS 方案，按需引入 Tailwind（团队决策点）。

已有依赖：`react 19`, `react-dom 19`, `@tauri-apps/api ^2`, `@tauri-apps/plugin-opener ^2`
