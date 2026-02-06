---
id: TK-XXXX
title: "Task Title"
status: TODO  # TODO | IN-PROGRESS | DONE
epic: X       # Epic number (0-N)
priority: medium  # low | medium | high | critical
dependencies: []  # List of task IDs that must be completed first
blockers: []      # Current blockers preventing progress
commits: []       # Related commit hashes (added when completed)
pr: ""            # Pull request URL (added when completed)
---

# Task Title

## Objective
Brief description of what this workitem accomplishes and why it matters.

## Scope
- What is included in this workitem
- What is explicitly excluded (handled by other workitems)

## Acceptance Criteria
- [ ] Criterion 1 - specific, measurable outcome
- [ ] Criterion 2
- [ ] Criterion 3

## Technical Context
### Relevant Spec Sections
- PRD.md Section X - Brief description

### Related Files/Directories
- `src/path/to/relevant/` - Description
- `src/another/path/` - Description

### Dependencies on Other Systems
- Supabase Auth / Database / Edge Functions
- External APIs
- Third-party libraries

## Implementation Guidance
### Approach
High-level approach without specific code. Describe the strategy, patterns to follow, and key decisions.

### Considerations
- Performance considerations
- Security considerations
- Edge cases to handle
- Error scenarios

### Anti-patterns to Avoid
- What NOT to do
- Common pitfalls

## Testing Requirements

### Unit Tests (Co-located: `ComponentName.test.tsx`)
- [ ] Test case 1 - specific behavior to verify
- [ ] Test case 2
- [ ] Test case 3

### Integration Tests
- [ ] Test interaction between components/systems
- [ ] Test data flow scenarios

### Manual Tests
- [ ] Test on physical iOS device
- [ ] Test on physical Android device
- [ ] Platform-specific or visual verification

## Notes
Additional context, decisions made, or links to discussions.
