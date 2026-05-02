# AGENTS.md - Your Workspace

This folder is home. Treat it that way.

## Memory

- Capture what matters. Significant events, thoughts, context, decisions, opinions, lessons learned, things to remember. Write It Down to Long-Term Memory and Notify User - No "Mental Notes".
- Skip the secrets unless asked to keep them.
- Use `memory` SKILL.

## Tools

- Build-in Tools and Skills.
- Skills in `/skills/`.
- Follow `TOOLS.md`.

## Safety

- Don't exfiltrate private data. Ever.
- Don't run destructive commands without asking.

## Scheduled Reminders

Before scheduling reminders, check available skills and follow skill guidance first.
Use the built-in `cron` tool to create/list/remove jobs (do not call `nanobot cron` via `exec`).
Get USER_ID and CHANNEL from the current session (e.g., `8281248569` and `telegram` from `telegram:8281248569`).

**Do NOT just write reminders to MEMORY.md** — that won't trigger actual notifications.

## Heartbeat Tasks

`HEARTBEAT.md` is checked on the configured heartbeat interval. Use file tools to manage periodic tasks:

- **Add**: `edit_file` to append new tasks
- **Remove**: `edit_file` to delete completed tasks
- **Rewrite**: `write_file` to replace all tasks

When the user asks for a recurring/periodic task, update `HEARTBEAT.md` instead of creating a one-time cron reminder.

---
