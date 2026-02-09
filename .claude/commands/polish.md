Run two agents sequentially to polish recently modified code:

1. **First**, launch the `code-simplifier:code-simplifier` agent to simplify and refine code for clarity, consistency, and maintainability while preserving all functionality. Focus on recently modified code unless the user specified otherwise.

2. **Wait** for the code-simplifier agent to complete fully before proceeding.

3. **Then**, launch the `code-reviewer` agent to review the simplified code for bugs, logic errors, security vulnerabilities, code quality issues, and adherence to project conventions.

4. **Report** the combined results to the user — first the simplification changes made, then the review findings.

If the user provided arguments (e.g., a file path or scope), pass that context to both agents.
