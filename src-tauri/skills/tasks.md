---
name: tasks
description: "Task management: create, list, and update task status"
version: "0.1.0"
---

# Tasks Skill

Use the `operations` tool to manage tasks.

## Available operations

### Create a task

```json
{
  "action": "call",
  "service": "tasks",
  "method": "create",
  "params": {
    "title": "Task title",
    "description": "Optional details"
  }
}
```

Returns the created task with `id`, `title`, `status`, and `created_at`.

### List tasks

```json
{
  "action": "call",
  "service": "tasks",
  "method": "list",
  "params": {
    "status": "todo"
  }
}
```

`status` filter is optional. Values: `"todo"`, `"in_progress"`, `"done"`.

### Update task status

```json
{
  "action": "call",
  "service": "tasks",
  "method": "update_status",
  "params": {
    "id": "task-id",
    "status": "done"
  }
}
```

## Usage guidelines

- List tasks before creating to avoid duplicates.
- Use `"in_progress"` when actively working on a task.
- Mark tasks `"done"` once completed; do not delete them.
- Titles should be action-oriented (e.g., "Write unit tests for parser").
