# Covers Check via spec-trace

- **Date**: 2026-10-28
- **Status**: completed
- **Type**: development tooling (test guard → upstream CLI check; coverage script; CI). No library behavior change.
- **Impact**: none for consumers; this repo loses one guard test and one permanent spec, and gains a CI step
- **Supersedes**: none

## Change Metadata

| Field | Value |
|---|---|
| Status | completed |
| plan-file | `specs/changes/2026-10-28-covers-check-via-spec-trace.scm` |
| updates | `CLAUDE.md`, `bin/check_coverage.sh`, `.github/workflows/ci.yml`, `spec-files.txt` |
| adds | (none) |
| obsoletes | `specs/development/covers-macro.md`, `tests/covers_macro_guard.rs` |
| merge-into | (none) |
| new-spec-ids | (none) |
| modified-spec-ids | (none) |
| retired-spec-ids | `mcp-tools/development/covers-must-record`, `mcp-tools/development/design-rationale` |
| non-testable-sections | (none) |
| consumer-notice | granter (requester; plan agreed in msg 1070, decided in 1076, hold lifted in 1106) — owed: the merge commit, after merge. Other consumers: (none) — "we moved our own covers! check to spec-trace's CLI, you need do nothing" |

## Summary
<!-- spec-id: mcp-tools-changes/covers-check-via-spec-trace/summary non-testable -->

spec-trace now owns the rule that tests must record coverage through its `covers!` (`spec-trace-spec/impl/covers-must-record`) and ships a check for it, `spec-trace check-macros` (`spec-trace-spec/cli/check-macros`), tested upstream with planted controls. This change retires our own copy of that rule and its guard test, cites spec-trace instead, runs `check-macros` in CI and in the coverage script, moves the `spec-trace` pin to a revision that has the check, and replaces the old 56-line coverage script with `spec-trace run`.

## Motivation
<!-- spec-id: mcp-tools-changes/covers-check-via-spec-trace/motivation non-testable -->

- granter msg 1069: spec-trace merged the rule and its check (PR #5 9feb054, PR #6 a344b66). The operator proposed deleting each project's guard and project spec-id, citing spec-trace, and running `check-macros`.
- lisper msg 1070 agreed, on the condition that the CI step lands in the same change as the guard's deletion; otherwise a local `covers!` would again go unnoticed.
- granter msg 1076 fixed the order: archive the old change spec (done, PR #14), then make this one change, including the coverage-script replacement. msg 1106 lifted the hold.
- `bin/check_coverage.sh` still runs `cargo run --manifest-path ../spec-trace/Cargo.toml … coverage` and needs a sibling checkout; `spec-trace run` replaces it.

## Scope
<!-- spec-id: mcp-tools-changes/covers-check-via-spec-trace/scope non-testable -->

**In scope:**
- Delete `tests/covers_macro_guard.rs`.
- Delete `specs/development/covers-macro.md` and its line in `spec-files.txt`; `cargo build` drops `SpecItem::McpToolsDevelopmentCoversMustRecord` and `…DevelopmentDesignRationale`.
- `CLAUDE.md`: the "Never define a local `covers!` macro" paragraph cites `spec-trace-spec/impl/covers-must-record` and names `spec-trace check-macros --docs CLAUDE.md` as the check, instead of the guard and the deleted spec.
- `.github/workflows/ci.yml`: in the `test` job, install the `spec-trace` CLI at the revision `Cargo.lock` pins, then run `spec-trace check-macros --docs CLAUDE.md`.
- Local `Cargo.lock`: `cargo update -p spec-trace` (5b1da0b → a344b66 or later). `Cargo.lock` is git-ignored in this library crate, so this only moves the maintainer's working tree; CI resolves the `spec-trace` git dependency afresh on every run.
- `bin/check_coverage.sh`: replace with the two-line script below. The uncommitted local edit to the old script is discarded (maintainer's OK, 2026-10-27).

**Out of scope:**
- `.claude/skills/*/project-conventions.md` lines naming `bin/check_coverage.sh` — the path stays the same.
- `docs/spec-trace-guide.md` — provisioned by mcp-planner, untracked here, not passed to `--docs`.
- Covering the `pretty` specs under default features.

## Coverage Script
<!-- spec-id: mcp-tools-changes/covers-check-via-spec-trace/coverage-script non-testable -->

```bash
#!/bin/bash
# Coverage: reject local covers! macros, then run the suite and report (spec-trace run).
spec-trace check-macros --docs CLAUDE.md || exit 1
exec spec-trace run --threshold 100 "$@" -- cargo test --all-features --no-fail-fast
```

- **Threshold 100.** Measured 2026-10-28 with the CLI at a344b66: `--all-features` covers 50 / 50 testable specs (100.0%); after this change the retired spec-id leaves 49 / 49.
- **`--all-features`.** Under default features coverage is 84.0% (42 / 50): the 8 `mcp-tools/pretty/*` specs are tested only with the `format-pretty` feature. The script therefore names the test command explicitly; `"$@"` passes further `spec-trace run` options (e.g. `--format detailed`).

## CI Step
<!-- spec-id: mcp-tools-changes/covers-check-via-spec-trace/ci-step non-testable -->

In the `test` job, after the SSH agent and toolchain steps:

1. `cargo generate-lockfile` (CI has no committed `Cargo.lock`), then read the resolved revision from `Cargo.lock` (the `#<sha>` of the `spec-trace` `source` line).
2. `cargo install --locked --git ssh://git@github.com/ywata/spec-trace --rev <sha> spec-trace` (the existing deploy key gives access).
3. `spec-trace check-macros --docs CLAUDE.md`.

Installing at the revision this run resolved keeps the CLI's check and the library's `covers!` in step within the run.

## Implementation Notes
<!-- spec-id: mcp-tools-changes/covers-check-via-spec-trace/implementation non-testable -->

1. `cargo update -p spec-trace`; record the new revision. `cargo test --all-features` passes.
2. Delete `tests/covers_macro_guard.rs`, `specs/development/covers-macro.md`, and the `spec-files.txt` line; `cargo build`; confirm no source references the retired `SpecItem` variants.
3. Update the `CLAUDE.md` paragraph.
4. Replace `bin/check_coverage.sh`; run it; expect check-macros clean and 100.0%.
5. Add the CI step.
6. Verify: `cargo test --all-features`, `cargo clippy -- -D warnings`, `cargo fmt --check`, `cargo doc --no-deps` (`-D warnings`), `spec-trace check-macros --docs CLAUDE.md`, and the reference check (no tracked file outside `specs/changes/` refers to a change spec by path).

## Design Rationale
<!-- spec-id: mcp-tools-changes/covers-check-via-spec-trace/rationale non-testable -->

- **Retire, not mark non-testable.** The rule is now spec-trace's; keeping a project spec-id for it would duplicate the source of truth, and a validation rule cannot honestly be `non-testable` (planner's objection, msg 1069).
- **One change.** Deleting the guard without the CI step would leave nothing that fails on a local `covers!`.
- **CLI at the locked revision**, not whatever is in `~/.cargo/bin`, so CI checks against the same spec-trace the tests link.

## Next Steps
<!-- spec-id: mcp-tools-changes/covers-check-via-spec-trace/next-steps non-testable -->

1. Review this proposal, then `plan-and-run` (or implement by hand with `Status: completed` in the finalizing commit).
2. After merge, send granter the merge commit (consumer-notice). Archiving waits for the maintainer.

## Completion Record (2026-10-28)
<!-- spec-id: mcp-tools-changes/covers-check-via-spec-trace/completion-record non-testable -->

- **Plan**: namespace `94bc9328-c5ed-4aed-bcdc-d23fd794382e`, tree `main`, 6 atomic goals, all complete (run via mcp-compose `plan-and-run`).
- **Pin**: local `Cargo.lock` moved spec-trace 5b1da0b → a344b66. Found during execution: `Cargo.lock` is git-ignored here, so the CI step generates the lockfile and installs the CLI at the revision it resolves (Scope and CI Step updated accordingly).
- **Retired**: `tests/covers_macro_guard.rs`, `specs/development/covers-macro.md` and its `spec-files.txt` line; `mcp-tools/development/covers-must-record` and `mcp-tools/development/design-rationale` are gone from `SpecItem`, and only archived change specs still name them.
- **Coverage** (`bin/check_coverage.sh`, CLI a344b66): check-macros clean (43 Rust files, 1 document); 49 / 49 testable specs = 100.0% with `--all-features`.
- **Checks**: `cargo test --all-features` 316 passed, 0 failed (318 before, minus the 2 guard tests); `cargo clippy -- -D warnings`, `cargo fmt --check`, `cargo doc --no-deps` (`-D warnings`) clean; CI YAML parses; reference check finds no change-spec path outside `specs/changes/`.
- **Consumer notice**: granter (1070/1076/1106). Still owed after merge: the merge commit. Archive waits for the maintainer.
