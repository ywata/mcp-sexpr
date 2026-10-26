# Tests Must Record Coverage

- **Date**: 2026-10-26
- **Status**: completed
- **Type**: test infrastructure (development rule + guard; no library behavior change)
- **Impact**: low for consumers (test-only code); high for this repo's coverage numbers, since `src/` tests currently record nothing
- **Supersedes**: none

## Change Metadata

| Field | Value |
|---|---|
| Status | completed |
| plan-file | `specs/changes/2026-10-26-covers-must-record.scm` |
| updates | (none) |
| adds | `specs/development/covers-macro.md` |
| obsoletes | (none) |
| merge-into | (none) |
| new-spec-ids | `mcp-tools/development/covers-must-record`, `mcp-tools/development/design-rationale` |
| modified-spec-ids | (none) |
| retired-spec-ids | (none) |
| non-testable-sections | `mcp-tools/development/design-rationale` |
| consumer-notice | granter (requester, msg 986; plan agreed in msg 1004/1008) — owed: the commit sha plus before/after coverage numbers, after merge. Other consumers: (none) — test-only change; "we changed how our own tests record coverage, you need do nothing" |

## Summary
<!-- spec-id: mcp-tools-changes/covers-must-record/summary non-testable -->

Eight `src/` test modules define a module-local `macro_rules! covers` that expands to `let _ = $item;`. It keeps the compile-time check but records nothing, so with `SPEC_TRACE_DB` set the `src/` unit tests write **0** coverage rows, and nothing warns about it. This change replaces those definitions with spec-trace's recording macro (`use spec_trace::covers;`), states the rule in a new permanent spec that cites spec-trace, adds a guard test that fails on any local definition, and fixes the template in `CLAUDE.md` that the local definitions were copied from.

## Motivation
<!-- spec-id: mcp-tools-changes/covers-must-record/motivation non-testable -->

- granter msg 986: the template that mcp-planner's `CLAUDE.md` and `docs/spec-trace-guide.md` showed for library tests defines the local macro. In mcp-planner it hid 602 rows over 126 spec-ids. mcp-planner fixed its templates in a08fade.
- Baseline measured here at d16e53f (`SPEC_TRACE_DB=<db> cargo test`, 208 tests pass): `count(*)` = 111, rows from `src/` tests (`test_name like '%::%'`) = **0**, distinct `spec_id` = 41.
- granter msg 1008 asks for the rule to be stated in our own spec, citing spec-trace (whose `covers!` and recording these are), and for our own guard test. The guard must not depend on mcp-planner or mcp-grant.

## Scope
<!-- spec-id: mcp-tools-changes/covers-must-record/scope non-testable -->

**In scope:**
- Replace the local `macro_rules! covers` in: `src/parser/types.rs`, `src/parser/lexer.rs`, `src/parser/differential.rs`, `src/parser/lexpr_compat.rs`, `src/parser/reader.rs`, `src/match_form/mod.rs`, `src/pretty/mod.rs`, `src/build/mod.rs`. `tests/` has none: it already uses `spec_trace::covers` via `tests/common`.
- New permanent spec `specs/development/covers-macro.md`, added to `spec-files.txt`.
- Guard test `tests/covers_macro_guard.rs`.
- `CLAUDE.md`: replace the library-test template with the recording form, and add a "never define a local `covers!`" note.
- `docs/spec-trace-guide.md`: take mcp-planner's fixed copy via `mcp-planner setup --update`; do not hand-edit it (provisioned file).
- Re-measure coverage and record before/after.

**Out of scope:**
- A spec-trace-owned check that would retire per-project guards (granter intends to ask spec-trace; not ours).
- `spec-trace run` (the coverage runner) — announced separately by granter.
- Changing which spec-ids the existing tests cover.

## New Permanent Spec: `specs/development/covers-macro.md`
<!-- spec-id: mcp-tools-changes/covers-must-record/new-spec non-testable -->

Title: **Coverage Macro**. Intro: tests link to specs with `covers!([SpecItem::…])`. spec-trace provides that macro, and its recording is spec-trace behavior (`spec-trace-spec/impl/runtime-reporting` in spec-trace's `docs/spec-trace.spec.md`): `covers!` calls `spec_trace::coverage::mark(item.spec_id())` for each item, and with the `runtime-coverage` feature (a default feature of this crate) and `SPEC_TRACE_DB` set, each mark becomes a row in the coverage database.

### `## Covers Must Record` — `mcp-tools/development/covers-must-record` (testable)

- Every test that links a spec uses spec-trace's `covers!`, either `use spec_trace::covers;` or a re-export of it (as `tests/common` does).
- No file under `src/` or `tests/` defines its own `covers!` (`macro_rules! covers`), and neither does any code block in `CLAUDE.md`, because contributors copy from there.
- A local definition that only type-checks the items (`let _ = $item;`) is the defect this rule forbids: the tests compile and pass but record nothing.
- Enforced by `tests/covers_macro_guard.rs`. The sweep excludes `docs/spec-trace-guide.md`, which is provisioned by mcp-planner and fixed there.

### `## Design Rationale` — `mcp-tools/development/design-rationale` (non-testable)

Why a source sweep instead of a database check, why `CLAUDE.md` is swept, why the guide is excluded, why there is a control, and that the guard is meant to be retired once spec-trace owns the check.

## Implementation Notes
<!-- spec-id: mcp-tools-changes/covers-must-record/implementation non-testable -->

1. **Replace** each local definition: delete the `macro_rules! covers { … }` block and add `use spec_trace::covers;` next to the module's other `use`s. `SpecItem` already exposes `spec_id()`, and `spec-trace` is already a `[dependencies]` entry with `runtime-coverage` in the default features, so no Cargo change is needed. Check whether the macro needs anything else in scope (e.g. a `SpecItem` method) and record what was needed.
2. **Guard test** `tests/covers_macro_guard.rs`, with `covers!([SpecItem::McpToolsDevelopmentCoversMustRecord])` in each test:
   - a pure detector `fn local_covers_definitions(text: &str) -> Vec<usize>` that returns the line numbers whose trimmed start is `macro_rules! covers`;
   - for `CLAUDE.md`, apply the detector only to the contents of fenced code blocks;
   - the sweep test walks `src/` and `tests/` (`*.rs`) plus the `CLAUDE.md` code blocks, and asserts there are no hits, listing `path:line` on failure;
   - the control test feeds the detector a fixture with a local definition and expects a hit, and a fixture with `use spec_trace::covers;` and expects none. The fixture builds the forbidden text with `concat!`/`format!` so that no line of the guard file itself starts with `macro_rules! covers` (granter msg 1008).
3. **CLAUDE.md**: in "For Library Tests (src/ modules)", replace the local macro block with `use spec_trace::covers;`, and add a short "Never define a local `covers!` macro" warning that names the silent-zero-rows failure and points at `specs/development/covers-macro.md`.
4. **Guide**: run `mcp-planner setup --update`, confirm `docs/spec-trace-guide.md` no longer contains `macro_rules! covers`, and handle any merge-pending customizable file through `/merge-customizations`.
5. **Measure**: `SPEC_TRACE_DB=<db> cargo test`, then `sqlite3 <db> "select count(*), sum(test_name like '%::%'), count(distinct spec_id) from coverage"`. Expect the second number to rise from 0. Record before/after in the Completion Record.
6. **Verify**: `cargo test --all-features`, `cargo clippy -- -D warnings`, `cargo fmt --check`, `cargo doc --no-deps` (`-D warnings`).

## Design Rationale
<!-- spec-id: mcp-tools-changes/covers-must-record/rationale non-testable -->

- **Sweep instead of a database check.** A database check (run the tests and expect rows from `src/`) needs `SPEC_TRACE_DB` and a full test run inside a test. A text sweep is fast, needs no environment, and fails at the line that is wrong. The before/after measurement checks the database side once, by hand.
- **Sweep `CLAUDE.md` code blocks.** The defect spread by copying the template. Guarding only the code would let the template seed the defect again.
- **Exclude the guide.** It is provisioned by mcp-planner. A hand edit would be overwritten, and mcp-planner owns the fix (a08fade).
- **Control.** Without it, a sweep that silently matches nothing (wrong directory, broken pattern) would pass forever.
- **Own spec citing spec-trace.** `covers!` and its recording belong to spec-trace. Our spec states only our rule (use it, do not shadow it) and cites spec-trace for the behavior, so the guard depends on no other consumer project. granter plans to ask spec-trace to own this check; this guard can then be retired.

## Next Steps
<!-- spec-id: mcp-tools-changes/covers-must-record/next-steps non-testable -->

1. Review this proposal, then run `plan-and-run`.
2. Order: write `specs/development/covers-macro.md` → add it to `spec-files.txt` → `cargo build` → guard test (the sweep fails while the 8 local definitions exist) → replace the 8 definitions → `CLAUDE.md` → `setup --update` for the guide → measure → verify.
3. After merge, send granter the sha and the before/after numbers (consumer-notice). Archiving waits for the maintainer.

## Completion Record (2026-10-26)
<!-- spec-id: mcp-tools-changes/covers-must-record/completion-record non-testable -->

- **Plan**: namespace `601a3d95-5334-4fb8-a77f-c405af541dd5`, tree `main`, 8 atomic goals, all complete (run via mcp-compose `plan-and-run`).
- **Spec-ids**: `mcp-tools/development/covers-must-record`, `mcp-tools/development/design-rationale` present in `specs/development/covers-macro.md`, which is listed in `spec-files.txt`; this change spec is not.
- **Coverage** (`SPEC_TRACE_DB=<db> cargo test`; `select count(*), sum(test_name like '%::%'), count(distinct spec_id) from coverage`):

  | | tests | rows | rows from `src/` tests | distinct spec-ids |
  |---|---|---|---|---|
  | before (d16e53f) | 208 | 111 | 0 | 41 |
  | after | 210 | 206 | 93 | 42 |

  The +1 spec-id is the guard's own `mcp-tools/development/covers-must-record`; the `src/` unit tests cover spec-ids the integration tests already covered, so the rise shows in rows, not in distinct ids. Sample `src/` row: `build::tests::cons_produces_pair`.
- **Replacement**: 8 `src/` modules switched to `use spec_trace::covers;`; nothing else was needed in scope (`SpecItem::spec_id()` already existed; this crate's default `runtime-coverage` feature gates the call-site `cfg`). No `covers!()` call or test body changed.
- **Guard**: `tests/covers_macro_guard.rs` — sweep flagged all 9 sites (8 `src/` files + `CLAUDE.md:99`) before the fix and passes after; control passes.
- **Guide**: `mcp-planner setup --update` refreshed `docs/spec-trace-guide.md` to mcp-planner's fixed copy (0 local definitions). The guide is git-excluded in this repo, so there is nothing to commit for it; setup changed no tracked file and no customizable file was merge-pending.
- **Checks**: `cargo test --all-features` 318 passed, 0 failed; `cargo clippy -- -D warnings`, `cargo fmt --check`, `cargo doc --no-deps` (`-D warnings`) clean. Known, out of scope: `cargo clippy --tests/--all-targets` fails on main already (`tests/parser_grammar.rs`, `tests/parser_api.rs`); the guard itself is clippy-clean.
- **Consumer notice**: granter (msg 986; plan agreed in 1004/1008). Still owed after merge: the commit sha and the numbers above. Archive waits for the maintainer.
