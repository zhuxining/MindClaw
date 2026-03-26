# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

KnowlCalw is a Tauri 2.0 desktop application combining a React 19 + TypeScript frontend with a Rust backend. Package manager is **Bun**.

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

### IPC Pattern (Frontend ↔ Rust)

All frontend-to-backend calls go through Tauri's `invoke()`:

```ts
// Frontend: src/App.tsx
import { invoke } from "@tauri-apps/api/core";
const result = await invoke("greet", { name });
```

```rust
// Backend: src-tauri/src/lib.rs
#[tauri::command]
fn greet(name: &str) -> String { ... }
```

New Rust commands must be registered in `lib.rs` inside `.invoke_handler(tauri::generate_handler![...])`.

### Key Locations

| Path | Purpose |
|------|---------|
| `src/App.tsx` | Main React component, demonstrates IPC |
| `src-tauri/src/lib.rs` | Rust Tauri commands and app setup |
| `src-tauri/src/main.rs` | Rust binary entry (delegates to lib) |
| `src-tauri/tauri.conf.json` | App config (identifier, window size, bundle) |
| `src-tauri/capabilities/default.json` | Tauri permission grants for windows |

### Security / Permissions

Tauri 2.0 uses a capabilities-based permission model. Any new plugin or API access must be declared in `src-tauri/capabilities/default.json` under the window's permissions array. CSP is currently `null` (disabled).

### Build Flow

- Dev server: Vite on `http://localhost:1420`, HMR on port 1421
- Tauri wraps the Vite dev server in a native window during `tauri dev`
- Production: `bun run build` outputs to `../dist`, then Tauri bundles the binary
