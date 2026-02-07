## Work Items

This project uses a `.workitems/` directory to track all implementation work as structured markdown files. Before starting any task, consult these files to understand scope, dependencies, and acceptance criteria.

### Structure

- `.workitems/CLAUDE.md` — Full guidelines for working with workitem files
- `.workitems/_template.md` — Template for creating new task files
- `.workitems/epic_NN_name/overview.md` — Epic summary with task table and status
- `.workitems/epic_NN_name/task_NN_slug.md` — Individual task files with YAML frontmatter

### Workflow

1. **Find work:** Read the epic `overview.md` files to find tasks with `status: TODO` whose dependencies are all `DONE`.
2. **Start a task:** Read the full task file, review referenced PRD sections, then set `status: IN-PROGRESS`.
3. **Complete a task:** Verify all acceptance criteria, set `status: DONE`, add commit SHAs to `commits`, and update the epic's task table.
4. **Create new tasks:** Copy `_template.md`, follow the naming convention (`task_{NN}_{kebab-slug}.md`), and add it to the epic's overview.

### Key Rules

- Task IDs follow `TK-{epicNN}{taskNN}` format (e.g., `TK-0100` = epic 01, task 00).
- Task files describe **what** to build, not **how** — no code snippets unless absolutely necessary.
- Always check dependency chains before starting work.
- See `.workitems/CLAUDE.md` for the complete reference.

