---
name: notes
description: "Vault note operations: create, search, get, and tag notes"
version: "0.1.0"
---

# Notes Skill

Use the `operations` tool to manage Vault notes.

## Available operations

### Create a note

```json
{
  "action": "call",
  "service": "vault",
  "method": "create",
  "params": {
    "title": "Note title",
    "content": "Markdown content",
    "tags": ["tag1", "tag2"]
  }
}
```

### Search notes

```json
{
  "action": "call",
  "service": "vault",
  "method": "search",
  "params": {
    "query": "search terms"
  }
}
```

Returns a list of matching notes with `id`, `title`, `tags`, and a content excerpt.

### Get a note by ID

```json
{
  "action": "call",
  "service": "vault",
  "method": "get",
  "params": {
    "id": "note-id"
  }
}
```

Returns the full note with `id`, `title`, `content`, `tags`, and `created_at`.

### List all tags

```json
{
  "action": "call",
  "service": "vault",
  "method": "list_tags",
  "params": {}
}
```

Returns all tags used across notes.

## Usage guidelines

- Always search before creating to avoid duplicates.
- Use meaningful tags to make notes discoverable.
- Titles should be concise and descriptive.
- Content supports Markdown formatting.
