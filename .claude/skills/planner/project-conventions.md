
# Planner Project Conventions — mcp-planner

This file defines project-specific conventions for the planner skill bundle.

## :meta Field Conventions

The `:meta` field is optional. Do not add `:meta` unless it provides clear value.

When `:meta` is present, it SHOULD only use the keys defined below.

### Communicating `:meta` Semantics to AI/Agents

The semantics of `:meta` keys are project-defined and may not be known to generic tooling.

Rule: when a goal includes `:meta`, the goal MUST also include a short human-readable explanation of what the `:meta` means in either:

- The goal `:spec` string, or
- The goal `:descr` string.

This ensures that:

- A human reading the goal tree understands the intent of `:meta`.
- An LLM agent that does not consult this file still receives the semantics in-band.

### Optional Keys

- `:test-scope` (string)
  - Meaning: test intensity expected after completing the goal.
  - Allowed values:
    - `"targeted"` — run only the most obviously affected integration test target(s) using `cargo test --test <name>`.
    - `"full"` — run the full suite using `cargo test`.

- `:test-targets` (list of strings)
  - Meaning: integration test targets to run when `:test-scope` is `"targeted"`.
  - Each item is the integration test crate name (filename under `tests/` without `.rs`).
  - Example: `("scenario_goal_ast_tests" "atomic_api_tests")`.

## Test Execution Policy (Planning Discipline)

When generating or executing plans:

- Prefer **targeted tests during development**:
  - Use `cargo test --test <name>` for the most obviously affected integration test file(s).
  - Use this after small, localized changes to reduce iteration time.

- Run **full tests after major workstreams complete**:
  - A “major workstream” means completing a cohesive subtree that changes behavior across multiple modules (e.g., spec change + implementation + tests; or multi-file refactor; or API/behavior change).
  - At that milestone, run `cargo test`.

## Examples

Targeted tests goal:

```lisp
(goal "run-targeted-tests"
  :spec "Run targeted integration tests for the affected area. Meta: test-scope=targeted means do not run the full suite. Meta: test-targets lists integration test targets (tests/<name>.rs without .rs) to run via cargo test --test <name>."
  :atomic true
  :meta (:test-scope "targeted" :test-targets ("scenario_goal_ast_tests"))
  :outputs ((doc "targeted-tests-passed")))
```

Full suite milestone goal:

```lisp
(goal "run-full-tests"
  :spec "Run full test suite (milestone check). Meta: test-scope=full means run cargo test (ignore any targeted list)."
  :atomic true
  :meta (:test-scope "full")
  :outputs ((doc "full-tests-passed")))
```
