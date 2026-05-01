
# mcp-planner Project Rules

## Communication Style
- Provide honest opinions directly without excessive praise
- Avoid agreeable responses that don't add value
- Share constructive feedback and genuine assessments
- Focus on technical accuracy over politeness when needed

## Document first approach
- Update spec document in specs/ before design or implement source code.

## Bug Fix Discipline
- Triage bugs before investigation: behavioral vs coding error
- For behavioral bugs: check spec first, then fix code to match spec
- Never close a behavioral bug without spec coverage and covers!() in the regression test
- This workflow applies only to ad-hoc bug investigation, not during plan execution
- See docs/bug-fix-workflow.md for the full workflow

## Implement and then Test approach
- Incrementally implement code and test just after its implementation.
- DO NOT update specification document during source code updates. They should be clearly separated.

## Spec-Trace: MANDATORY for All Development

### Specification Phase (Document First)

**ALWAYS add spec-id annotations when creating or updating specification documents:**

1. **Add spec-id immediately after each second-level heading (##):**
   ```markdown
   ## Feature Name
   <!-- spec-id: mcp-planner/category/feature -->
   
   Feature description...
   
   ### Subsection (no spec-id needed for ### headings)
   Details about the feature...
   ```
   
   **Rule:** Every `##` heading MUST have a spec-id annotation on the next line.

2. **Follow spec-id format rules:**
   - Format: `<doc>/<category>/<item>[/<subitem>...]`
   - Lowercase, kebab-case, ASCII only
   - At least 2 slashes (minimum 3 components)
   - Stable across refactoring (don't change unless concept changes)
   - Examples: `mcp-planner/goal-spec/delegated-goals`, `mcp-planner/api-spec/register-plan`

3. **Mark non-testable sections:**
   ```markdown
   ## Design Rationale
   <!-- spec-id: mcp-planner/design/rationale non-testable -->
   ```
   - Use `non-testable` flag for: design rationale, references, future enhancements, background
   - Do NOT use for: API behavior, data formats, algorithms, validation rules, error handling

4. **Regenerate code after spec changes:**
   ```bash
   cargo build  # Regenerates tests/traceability_gen.rs
   ```

### Implementation Phase (Test with Coverage)

**ALWAYS add covers!() macro to tests:**

#### For Integration Tests (tests/ directory):

```rust
mod common;
use common::{covers, SpecItem};

#[test]
fn test_feature() {
    covers!([SpecItem::McpPlannerCategoryFeature]);
    // Test implementation...
}

#[test]
fn test_complex_feature() {
    // Cover multiple related specs
    covers!([
        SpecItem::McpPlannerCategoryFeature,
        SpecItem::McpPlannerOtherRelatedSpec
    ]);
    // Test implementation...
}
```

#### For Library Tests (src/ modules):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::SpecItem;
    
    // Define compile-time coverage check macro
    macro_rules! covers {
        ([$($item:expr),* $(,)?]) => {
            {
                $(
                    // Compile-time check that the item exists
                    let _ = $item;
                )*
            }
        };
    }

    #[test]
    fn test_feature() {
        covers!([SpecItem::McpPlannerCategoryFeature]);
        // Test implementation...
    }
}
```

### Mandatory Workflow

**Every feature implementation MUST follow this sequence:**

1. **Write Specification First**
   - Add spec document in `specs/` with `<!-- spec-id: ... -->` annotations
   - Mark non-testable sections appropriately
   - Add new spec files to `spec-files.txt` and run `cargo build` to generate SpecItem enum
   - **NEVER add change specs (`specs/changes/`) to `spec-files.txt`** — only permanent spec files

2. **Write Tests with Coverage**
   - Add `covers!([SpecItem::...])` at start of each test
   - Link tests to relevant spec-ids
   - Verify compilation (ensures spec-ids exist)

3. **Implement Feature**
   - Write implementation code
   - Ensure tests pass

4. **Verify Traceability**
   - All testable specs MUST have at least one test with `covers!()`
   - Compile-time verification prevents broken spec references
   - Use `docs/spec-trace-guide.md` for detailed instructions

### Enforcement

**Spec-trace provides compile-time safety:**
- Tests won't compile if spec-id doesn't exist
- Prevents broken references between specs and tests
- Ensures documentation stays synchronized with tests

**AI Assistants MUST:**
- Add spec-id annotations when creating/updating specs
- Add covers!() macros when creating/updating tests
- Follow the patterns in `docs/spec-trace-guide.md`
- Never skip spec-trace annotations

**Developers MUST:**
- Review that all new specs have spec-id annotations
- Review that all new tests have covers!() macros
- Run `cargo build` after spec changes
- Verify tests compile successfully

**Reference:** See `docs/spec-trace-guide.md` for complete details and examples.

## Rust Design Principle

- Define enum to convey meaning of the code so that specification change produces compile time error as much as possible.
- Use functional style code as much as possible. Procedural style should be avoided.


## Rust Code Style

- Use Rust 2021 edition idioms
- Prefer explicit error handling with `anyhow::Result` for application code
- Use `thiserror` for custom error types in library code
- Always add comprehensive error context with `.context()` or `.with_context()`

## Testing

- Write tests for all public API functions
- Use edge case tests for parser and state management
- Maintain test coverage for validation logic
- Test files should be in `tests/` directory for integration tests
- **MANDATORY: Add covers!() macro to EVERY test** (see Spec-Trace section above)
- **MANDATORY: Link every test to its specification via covers!()**
- Use spec-trace to detect missing error cases
- Read `docs/spec-trace-guide.md` for complete implementation guide

### Critical Testing Discipline

**NEVER delete tests to make them pass. This is absolutely forbidden.**

- If tests fail due to missing SpecItem variants, add the spec-ids to permanent spec files
- If tests fail due to compilation errors, fix the test code or the implementation
- If tests fail due to behavior changes, update the tests OR fix the code to match spec
- Deleting tests destroys coverage, hides problems, and violates fundamental testing principles
- The only acceptable reason to delete a test is if the feature itself is being removed

**Spec-id placement rule for change spec plans:**
1. New testable spec-ids must be added to permanent spec files during implementation — not deferred to plan completion
2. Change specs are NEVER added to `spec-files.txt`; their spec-ids serve as documentation only
3. At plan all-complete, verify all `new-spec-ids` from the change spec exist in permanent spec files
4. NEVER delete tests just because a change spec was archived
5. The `/spec-change-archive` workflow will verify spec-id placement before archiving

## Goal AST Format

- Use S-expression format for goal trees
- Use `get-concept` from MCP Planner to load Goal AST rules (e.g., `goal-ast-basics`, `validation-rules`, `output-propagation`)
- Validate all goal trees before registration
- Support both atomic and non-atomic goals
- Use lazy goals ONLY for human decision-making tasks, not LLM-plannable tasks

## MCP Protocol

- Use `rmcp` crate for MCP protocol implementation
- Support multiple transport types (stdio, HTTP, child process)
- Follow MCP server/client patterns
- Provide clear tool descriptions in S-expression format

## Documentation

- Keep specification documents in `specs/`
- Keep documentation and guides in `docs/`
- Use markdown for both specifications and documentation
- **MANDATORY: Add <!-- spec-id: ... --> after EVERY second-level heading (##) in spec documents (specs/)**
- Third-level headings (###) and below do NOT need spec-ids
- Mark informational sections with `non-testable` flag
- Maintain clear separation between specs and implementation
- Update TODO.md for tracking pending work
- Spec-ids must be stable across refactoring

## MCP Planner Workflow Discipline

When an MCP Planner plan namespace is active (i.e., a goal tree has been registered and is not yet `all-complete`):

- **NEVER work on goals outside the planner.** All goal progress MUST be tracked through `claim-goal` → work → `complete-goal` → `get-current-goal`.
- **On session start or context resumption:** Call `get-active-trees` to check for in-progress plans. If one exists, call `get-current-goal` to determine where to resume. Do NOT start working from memory or checkpoint summaries alone.
- **After completing work for a goal:** ALWAYS call `complete-goal` immediately, then `get-current-goal` to advance. Never skip these calls.
- **Do NOT use `todo_list` as a substitute** for the MCP Planner when a plan is active. The planner is the single source of truth for goal state.
- **If the planner is unreachable:** Stop and inform the user rather than continuing without it.

## Dependencies

- Keep dependencies up to date
- Use semantic versioning
- Test after dependency updates
- Document any version constraints
