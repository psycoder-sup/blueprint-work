# Workitem File Guidelines

This document provides guidance for working with workitem files in the `.workitems/` directory.

## Directory Structure

```
.workitems/
├── _template.md                          # Task file template
├── CLAUDE.md                             # This file (guidelines)
├── epic_00_foundation/
│   ├── overview.md                       # Epic overview & task table
│   ├── task_00_cargo-init.md
│   ├── task_01_sqlite-schema.md
│   └── ...
├── epic_01_mcp-server/
│   ├── overview.md
│   ├── task_00_jsonrpc-transport.md
│   └── ...
└── epic_NN_name/
    └── ...
```

### Naming Conventions

- **Epic folders:** `epic_{NN}_{kebab-name}/` — zero-padded two-digit epic number
- **Task files:** `task_{NN}_{kebab-slug}.md` — zero-padded two-digit task number within the epic
- **Overview files:** `overview.md` — one per epic folder, summarizes the epic and lists all tasks
- **ID format:** `TK-{epicNN}{taskNN}` — e.g., `TK-0100` for epic 01, task 00

## Task File Structure

Every task file follows the structure defined in `_template.md`:

1. **YAML Frontmatter** — Metadata in `---` delimiters
2. **Title** — H1 heading matching the frontmatter title
3. **Objective** — What the task accomplishes and why
4. **Scope** — What's included and excluded
5. **Acceptance Criteria** — Measurable checkboxes
6. **Technical Context** — PRD references, related files, dependencies
7. **Implementation Guidance** — Approach, considerations, anti-patterns
8. **Testing Requirements** — What to test (Unit, Integration, Manual)
9. **Notes** — Additional context

## YAML Frontmatter Format

```yaml
---
id: TK-XXXX            # Required. Format: TK-{epicNN}{taskNN}, e.g., TK-0100
title: "Task Title"    # Required. Descriptive title in quotes
status: TODO           # Required. One of: TODO | IN-PROGRESS | DONE
epic: 1                # Required. Epic number (0-N)
priority: medium       # Required. One of: low | medium | high | critical
dependencies: []       # Required. List of task IDs that must complete first
blockers: []           # Optional. Current blockers preventing progress
commits: []            # Optional. Related commit hashes (added on completion)
pr: ""                 # Optional. Pull request URL (added on completion)
---
```

### Field Details

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | string | Yes | Unique identifier. Pattern: `TK-{epicNN}{taskNN}` |
| `title` | string | Yes | Human-readable title matching the H1 heading |
| `status` | enum | Yes | `TODO` → `IN-PROGRESS` → `DONE` |
| `epic` | number | Yes | Epic number matching the folder (0-N) |
| `priority` | enum | Yes | `low`, `medium`, `high`, or `critical` |
| `dependencies` | array | Yes | Task IDs that block this task (can be empty `[]`) |
| `blockers` | array | No | Current issues preventing progress |
| `commits` | array | No | Commit SHAs implementing this task |
| `pr` | string | No | Associated pull request URL |

## Epic Overview File Structure

Each epic folder contains an `overview.md` with:

```markdown
# Epic NN: Name

## Description
What this epic delivers and why it matters.

## Status
`todo` | `in-progress` | `done`

## Dependencies
Other epics that must complete first (or "None" for root epics).

## Blocked By
(none) or list of blocking epics/issues.

## Blocks
- epic_NN_name (list of epics that depend on this one)

## Tasks

| # | Task | Status |
|---|------|--------|
| 00 | Task title | todo |
| 01 | Task title | todo |

## Acceptance Criteria
- High-level criteria for the epic as a whole
```

## Content Guidelines

### No Code in Task Files

**Task files describe WHAT to build, not HOW to build it.**

Do NOT include:
- Actual code implementations
- Code snippets showing how to write something
- Copy-paste ready code blocks

DO include:
- References to existing code patterns in the codebase
- References to PRD.md with implementation details
- High-level architectural decisions
- Type/interface names to create (not their implementations)
- Function signatures conceptually (not full implementations)

### When Code Is Acceptable

Only include code in task files when there is NO OTHER WAY to explain a concept:

1. **Algorithm/logic flow** — When describing complex business logic that cannot be expressed in prose
2. **Data structure examples** — When showing the shape of data (e.g., JSON response format)
3. **Configuration snippets** — When exact config values are required

Even then, keep code minimal and conceptual.

### Writing Acceptance Criteria

- Make criteria specific and measurable
- Use checkbox format `- [ ]` for tracking
- Each criterion should be independently verifiable
- Avoid vague terms like "properly", "correctly", "well"

```markdown
# BAD
- [ ] Feed works properly
- [ ] Good error handling

# GOOD
- [ ] useFeed hook returns posts, isLoading, error, fetchNextPage, hasNextPage
- [ ] Network errors display user-friendly message via error state
- [ ] Empty feed shows EmptyState component
```

### Writing Testing Requirements

Testing requirements should be organized into three categories:

#### 1. Unit Tests
- Test individual functions, hooks, and components in isolation
- Mock external dependencies
- Co-locate test files with source
- Use checkbox format for specific test cases

#### 2. Integration Tests
- Test interactions between multiple components/systems
- Test data flow through the application

#### 3. Manual Tests
- Tests requiring real environment verification
- Visual/UX verification that can't be automated
- Platform-specific behavior

### Referencing Specifications

Always point to the authoritative source:

```markdown
### Relevant Spec Sections
- PRD.md Section X - Brief description of what's referenced
```

## Working with Tasks

### Starting a Task

1. Read the full task file
2. Check that all dependencies have `status: DONE`
3. Update status to `IN-PROGRESS`
4. Review referenced PRD sections

### Completing a Task

1. Verify all acceptance criteria are met
2. Update status to `DONE`
3. Add commit SHAs to `commits` field
4. Add PR URL to `pr` field if applicable
5. Update the epic's `overview.md` task table status

### Creating New Tasks

1. Copy `_template.md` to the appropriate epic folder
2. Rename to `task_{NN}_{kebab-slug}.md`
3. Fill in all required YAML fields
4. Follow the section structure from the template
5. Add the task to the epic's `overview.md` task table
6. Set dependencies on other task IDs as needed

### Creating New Epics

1. Create folder: `epic_{NN}_{kebab-name}/`
2. Create `overview.md` following the epic overview structure
3. Create task files within the folder
4. Update any existing epic overviews that reference the new epic in Blocks/Dependencies
