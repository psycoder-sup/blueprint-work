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

### Plan Requirements

When creating a plan for a TK task, **always** include these steps explicitly:

1. **First step of the plan:** Set the task file's `status: IN-PROGRESS` in YAML frontmatter.
2. **Last steps of the plan:** Set `status: DONE`, add commit SHAs to `commits`, and update the epic's `overview.md` task table.

### Key Rules

- Task IDs follow `TK-{epicNN}{taskNN}` format (e.g., `TK-0100` = epic 01, task 00).
- Task files describe **what** to build, not **how** — no code snippets unless absolutely necessary.
- Always check dependency chains before starting work.
- See `.workitems/CLAUDE.md` for the complete reference.

## Code Quality Workflow

When creating plans related to code implementations or changes, include the following steps at the end of the plan. **These steps must be executed sequentially, not in parallel:**

1. **Code Simplification:** After implementing code changes, use the `code-simplifier:code-simplifier` agent to simplify and refine the created or edited code for clarity, consistency, and maintainability.

2. **Code Review:** After simplification is complete, use the `code-reviewer` agent to review the code for bugs, security vulnerabilities, and quality issues.