## Blueprint MCP

This project uses Blueprint MCP to track implementation work. When working on tasks or epics from a Blueprint project, keep their status up to date:

### Starting work

- Mark the **blueprint task** as `in_progress` when you begin implementing it.
- Mark the parent **blueprint epic** as `in_progress` if it isn't already.

### Completing work

- Mark the **blueprint task** as `done` when implementation is complete.
- Mark the parent **blueprint epic** as `done` only when **all** of its tasks are done.

### Creating tasks

- Before calling `create_task`, read `.blueprint/task_template.md` for description templates.
- Pick the template that best matches the task type and structure the task description accordingly.
