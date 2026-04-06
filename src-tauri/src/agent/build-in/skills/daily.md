---
name: daily
description: "Daily journal: read and append to the daily log"
version: "0.1.0"
---

# Daily Skill

Use the `operations` tool to interact with the daily journal.

## Available operations

### Get today's journal

```json
{
  "action": "call",
  "service": "daily",
  "method": "get",
  "params": {
    "date": "2026-03-29"
  }
}
```

`date` is in `YYYY-MM-DD` format. Returns the full Markdown content for that day.

### Append to today's journal

```json
{
  "action": "call",
  "service": "daily",
  "method": "append",
  "params": {
    "date": "2026-03-29",
    "content": "## 14:30\n\nFinished the auth module refactor."
  }
}
```

Appends `content` to the end of the existing journal entry for that day.

### Save (overwrite) journal

```json
{
  "action": "call",
  "service": "daily",
  "method": "save",
  "params": {
    "date": "2026-03-29",
    "content": "Full journal content"
  }
}
```

## Usage guidelines

- Prefer `append` over `save` to avoid overwriting existing entries.
- Use ISO 8601 date format (`YYYY-MM-DD`).
- Structure entries with Markdown headings and timestamps.
- Get the current entry before appending to understand existing context.
