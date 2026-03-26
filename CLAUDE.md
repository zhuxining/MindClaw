# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

MindClaw is a Tauri 2.0 desktop application combining a React 19 + TypeScript frontend with a Rust backend. Package manager is **Bun**.

> 详细架构文档见 `docs/design/architecture/README.md`

## Commands

```bash
# Start full desktop app in dev mode (Rust + frontend with HMR)
bunx tauri dev

# Frontend only (Vite dev server on port 1420)
bun run dev

# Type-check + production build (frontend)
bun run build

# Build distributable desktop app
bunx tauri build
```

No test or lint commands are configured yet.

## Architecture

### Three-Tier Command Architecture

系统有三种命令入口，底层共享 Services 层：

| Tier | 入口 | 位置 | 数量 |
|------|------|------|------|
| Web Commands | React `invoke()` | `src-tauri/src/commands/` | ~28 |
| Agent Commands | 对话中 `/xxx` | `src-tauri/src/agent_commands/` | 4 |
| CLI Commands | 终端 `mindclaw` | `src-tauri/src/cli/` | ~7 |

调用链：`Command → Services → Storage`（Web/CLI 共用），Agent Commands 由 AgentLoop 拦截处理。

### IPC Pattern (Frontend ↔ Rust)

All frontend-to-backend calls go through Tauri's `invoke()`:

```ts
import { invoke } from "@tauri-apps/api/core";
const result = await invoke("greet", { name });
```

```rust
#[tauri::command]
fn greet(name: &str) -> String { ... }
```

- New Rust commands must be registered in `lib.rs` inside `.invoke_handler(tauri::generate_handler![...])`.
- Commands return `Result<T, AppError>`, error type defined in `error.rs` (implements `Serialize` for IPC).
- Tauri state injection via `.manage()`, commands access via `State<'_, T>`.

### Key Locations

| Path | Purpose |
|------|---------|
| `src/App.tsx` | Root React component, routing, global providers |
| `src-tauri/src/lib.rs` | Tauri Builder: plugin/command registration, state injection |
| `src-tauri/src/main.rs` | Rust binary entry (delegates to lib) |
| `src-tauri/src/error.rs` | Unified `AppError` type (Serialize for IPC) |
| `src-tauri/src/commands/` | Web Commands (Tauri IPC handlers) |
| `src-tauri/src/agent/` | AgentLoop, ContextBuilder, SessionManager |
| `src-tauri/src/services/` | Core business logic (Knowledge, Daily, Task, Resource) |
| `src-tauri/src/storage/` | SQLite, Markdown read/write, Keychain |
| `src-tauri/src/providers/` | LLM abstraction (Claude API, Haiku/Sonnet) |
| `src-tauri/src/tools/` | Agent tool registry (filesystem, shell, operations) |
| `src-tauri/src/memory/` | Memory system (profile, preferences, entities, events, cases, patterns) |
| `src-tauri/src/channels/` | Channel trait + Desktop/Telegram/Feishu implementations |
| `src-tauri/src/bus/` | MessageBus: bidirectional async queue (Channel ↔ Agent) |
| `src-tauri/src/models/` | Data models (Note, Task, Message, Session, Settings) |
| `src-tauri/tauri.conf.json` | App config (identifier, window size, bundle) |
| `src-tauri/capabilities/default.json` | Tauri permission grants |
| `src/pages/` | React pages (Daily, Inbox, Knowledge, Conversation, Settings) |
| `src/components/` | UI components organized by feature |
| `src/hooks/` | Custom hooks (useIpc, useCapture, useConversation, etc.) |
| `src/store/` | Zustand stores (app, capture, conversation) |

### Storage Principles

- **Markdown first, SQLite complements.** Knowledge notes use Markdown as source of truth (SQLite is derived index). Tasks, memories, sessions use SQLite as source of truth.
- Knowledge notes use three-level indexing: L0 (tags, ~100 tokens) → L1 (overview, ~2k tokens) → L2 (full Markdown on disk).
- Write order: Markdown first, then update SQLite index. On index failure, write `.index_dirty` marker for rebuild on next startup.
- API Keys and Gateway tokens must be stored in OS Keychain (via `keyring` crate), never in plaintext files.
- Conversation messages: hot in SQLite for 90 days, then archived to JSONL cold storage.

### Security / Permissions

- Tauri 2.0 uses a capabilities-based permission model. New plugins/APIs must be declared in `src-tauri/capabilities/default.json`.
- `vault/private/` is invisible to Agent — storage layer rejects `private/` prefix paths for Agent access.
- Private content never enters SQLite index, RAG retrieval, or any IPC response.
- CSP should restrict to `'self'` + `https://api.anthropic.com` (currently `null` — to be configured).

### Agent Architecture (Key Patterns)

- **Channel abstraction**: All message sources (Desktop/Telegram/Feishu) implement `Channel` trait → unified `ChannelMessage`.
- **MessageBus**: Decouples Channel ↔ Agent via async inbound/outbound queues. Outbound queue survives Channel disconnects.
- **AgentLoop**: Main loop consumes Bus.inbound → SessionManager → ContextBuilder → Provider.chat() → ToolRegistry → Bus.outbound.
- **Agent Commands** (`/new`, `/stop`, `/restart`, `/status`): Intercepted in AgentLoop before context assembly, no LLM call.
- **SubAgent**: Async background tasks (knowledge distill, memory analyze, resource parse, etc.) dispatched from AgentLoop.
- **Provider**: Haiku for lightweight tasks (routing, classification, L1 generation), Sonnet for deep conversation.
- **Tools**: 4 always-in-context tools (filesystem, shell, mcp_client, operations). `operations` is a meta-tool for dynamic Service/Memory access.

### Build Flow

- Dev server: Vite on `http://localhost:1420`, HMR on port 1421
- Tauri wraps the Vite dev server in a native window during `tauri dev`
- Production: `bun run build` outputs to `../dist`, then Tauri bundles the binary

### User Data Directory (Runtime)

```
~/MindClaw/
  vault/                    # Markdown content (Obsidian-compatible)
    daily/                  # YYYY-MM-DD.md
    knowledge/              # Topic-organized knowledge notes
    private/                # Private zone (Agent cannot see)
  data/
    main.db                 # SQLite (indexes + FTS5 + memories + resources)
    archive/                # Cold archive (YYYY-MM.jsonl)
  config/
    settings.json           # Non-sensitive settings
```

Entire `~/MindClaw/` directory can be zip-backed up as a complete backup.
