# MindClaw 技术架构设计 — Heartbeat 健康检测

> 完整架构文档索引见 [README.md](./README.md) | Agent 核心见 [03-agent-loop.md](./03-agent-loop.md)

## Heartbeat — 健康检测

```rust
pub struct SystemHealth {
    pub status: HealthStatus,          // healthy | degraded | down
    pub db_connected: bool,
    pub api_key_valid: bool,
    pub vault_accessible: bool,
    pub gateway_running: bool,
    pub channels: Vec<ChannelHealth>,
    pub last_check: DateTime<Utc>,
    pub uptime_seconds: u64,
}
```

通道断线时自动重连，指数退避（2s → 4s → 8s → ... → 60s）。
