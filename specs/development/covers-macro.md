# Coverage Macro

Tests link to specs with `covers!([SpecItem::…])`. The macro and its recording come from spec-trace (`spec-trace-spec/impl/runtime-reporting` in spec-trace's `docs/spec-trace.spec.md`). For each item, `spec_trace::covers!` calls `spec_trace::coverage::mark(item.spec_id())`, under `#[cfg(feature = "runtime-coverage")]`.

That `cfg` is evaluated in the crate that calls the macro. So this crate's own `runtime-coverage` feature, which is on by default and forwards to `spec-trace/runtime-coverage`, decides whether marks are recorded. When `SPEC_TRACE_DB` is set, each mark becomes a row in that coverage database. The coverage report (`bin/check_coverage.sh`) reads those rows.

## Covers Must Record
<!-- spec-id: mcp-tools/development/covers-must-record -->

- Every test that links a spec uses spec-trace's `covers!`, either directly (`use spec_trace::covers;`) or through a re-export of it, as `tests/common` does.
- No file under `src/` or `tests/` defines its own `covers!` macro. Neither does any code block in `CLAUDE.md`, because contributors copy from there.
- This forbids, in particular, a local definition that only type-checks its items (`let _ = $item;`). With one, the tests compile and pass but record nothing, and nothing warns.
- `tests/covers_macro_guard.rs` enforces this rule. Its sweep excludes `docs/spec-trace-guide.md`, which mcp-planner provisions and fixes on its side.

## Design Rationale
<!-- spec-id: mcp-tools/development/design-rationale non-testable -->

- **The defect.** Until 2026-10-26, eight `src/` test modules defined `covers!` locally as `let _ = $item;`, copied from the library-test template in `CLAUDE.md`. With `SPEC_TRACE_DB` set, the `src/` unit tests wrote 0 coverage rows, while the integration tests in `tests/` recorded normally. granter reported the pattern across projects (msg 986). In mcp-planner it hid 602 rows over 126 spec-ids.
- **Source sweep, not a database check.** A database check would have to set `SPEC_TRACE_DB` and run the whole suite from inside a test. A text sweep is fast, needs no environment, and points at the offending line. The database side is measured by hand when the rule changes.
- **`CLAUDE.md` is swept.** The defect spread by copying its template. Guarding only the code would let the template bring it back.
- **The guide is not swept.** `docs/spec-trace-guide.md` is provisioned by mcp-planner. A hand edit would be overwritten, and mcp-planner owns the fix.
- **A control.** The guard also checks that its detector finds a planted local definition. Otherwise a sweep that silently matches nothing (a wrong directory, a broken pattern) would pass forever.
- **This rule lives here and cites spec-trace.** spec-trace owns `covers!` and its recording. This spec states only our rule, which is to use that macro and not shadow it, so the guard depends on no other consumer project. If spec-trace takes over this check, the guard can be retired.
