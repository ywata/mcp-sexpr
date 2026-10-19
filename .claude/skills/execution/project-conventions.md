# Execution Project Conventions — mcp-planner

This file defines the project-specific conventions for execution processes in the mcp-planner repository. All execution-related workflows read this file at startup and apply the extensions defined here.

---

## Bug Fix Process

When encountering bugs during ad-hoc investigation (not during plan execution), follow the triage and fix process defined in `bug-fix-workflow.md`.

### Triage Criteria

Use the triage question from bug-fix-workflow.md to determine if a bug is behavioral (spec issue) or a coding error.

### Behavioral Bugs

For behavioral bugs:
1. Check the relevant specification first
2. Fix the spec to match desired behavior using `/spec-change`
3. Update code to match the corrected spec
4. Add regression tests with `covers!()` macros

### Coding Errors

For obvious coding errors:
1. Fix directly in code
2. Add tests to prevent regression
3. No spec changes needed

---

## Issue Documentation

When recording execution issues encountered during plan execution:

1. Use the template in `issue-template.md` for consistent formatting
2. Place issue files in `docs/issues/` with naming convention: `YYYY-MM-DD-<short-slug>.md`
3. Include execution context (namespace ID, goal ID, workflow) when available

---

## Execution Context

These conventions apply specifically to:
- Ad-hoc bug investigation outside of plan execution
- Issue documentation during or after execution
- Execution-specific processes that complement generic workflows

During active plan execution under a claimed goal, the plan's goal tree takes precedence over these conventions.

---

## Post-`/exec-plan` Verification

When `/exec-plan` finishes in the terminal state `all-complete`, run post-execution verification **only if the execution modified the source tree**.

### Modification Gate

Treat the source tree as modified if `git status --porcelain` is non-empty after execution.

If the tree is unchanged, skip post-execution verification to keep `/exec-plan` lightweight and generic across projects.

### Verification for This Repository

If the tree is modified, run:

- `./bin/check_coverage.sh`

This validates spec-trace coverage remains complete and catches missing `covers!()` coverage links.
