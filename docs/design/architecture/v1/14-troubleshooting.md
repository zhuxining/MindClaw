# MindClaw 技术架构设计

> 完整架构文档索引见 [README.md](./README.md)

## Tauri Plugin 需要 `tauri.conf.json` 配置才能初始化

**现象**

```
thread 'main' panicked at src/lib.rs:N:10:
error while running tauri application: PluginInitialization("plugin-name",
"Error deserializing 'plugins.plugin-name' within your Tauri configuration:
invalid type: null, expected struct Config")
```

**原因**

部分 Tauri 插件在 `lib.rs` 中注册后，启动时会尝试从 `tauri.conf.json` 的
`plugins.<name>` 块反序列化配置。若该块不存在，Tauri 将其视为 `null` 并
panic——即使插件在代码中已提供了默认值。

**受影响插件**

| 插件                   | 必需的 `tauri.conf.json` 配置                                                          |
| ---------------------- | -------------------------------------------------------------------------------------- |
| `tauri-plugin-updater` | `plugins.updater.pubkey`（签名公钥）+ `plugins.updater.endpoints`（更新检查 URL 列表） |
| `tauri-plugin-cli`     | `plugins.cli.args` / `plugins.cli.subcommands`（CLI 参数定义）                         |

**处理策略**

- **功能尚未就绪时**：从 `lib.rs` 中移除对应的 `.plugin(...)` 调用，避免阻塞开发。
  Cargo.toml 中的依赖声明可保留（不影响编译）。

- **updater 就绪时**：
  1. 生成密钥对：`bunx tauri signer generate -w ~/.tauri/mindclaw.key`
  2. 在 `tauri.conf.json` 中添加：

     ```json
     "plugins": {
       "updater": {
         "pubkey": "<生成的公钥>",
         "endpoints": ["https://your-update-server/releases/{{target}}/{{arch}}/{{current_version}}"]
       }
     }
     ```

  3. 将 `.plugin(tauri_plugin_updater::Builder::new().build())` 加回 `lib.rs`。

- **cli 就绪时**：
  在 `tauri.conf.json` 中添加 CLI 定义后再注册插件：

  ```json
  "plugins": {
    "cli": {
      "description": "MindClaw CLI",
      "args": []
    }
  }
  ```

**当前状态（2026-03-29）**

`tauri-plugin-updater` 与 `tauri-plugin-cli` 均已从 `lib.rs` 中移除，
等待对应功能实现后再启用。Cargo.toml 依赖声明保留。
