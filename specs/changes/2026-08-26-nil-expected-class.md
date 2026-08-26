# `nil` token as an expected discrepancy class (2026-08-26)

- **Date:** 2026-08-26
- **Status:** completed (2026-08-27) — plan: `specs/changes/2026-08-26-nil-expected-class.scm`
- **Type:** defect fix (diagnostics only). One row added to the expected-class table; no change to any parse result.
- **Impact:** low. One match arm in `is_expected_class` in `src/parser/differential.rs`, one table row in `specs/parser/differential-mode.md`, tests under an existing spec-id.
- **Supersedes:** none. Follow-up to `specs/changes/2026-08-26-differential-report-hazards.md` (merged as PR #3, `d8c6e37`), which introduced the expected-class mechanism and recorded this case as out of scope.
- **Origin:** surfaced while probing inputs for the class-dedup tests in PR #3: `(a nil)` is reported at `1.atom` with both sides `Ok`.

## Change Metadata
<!-- spec-id: mcp-tools-changes/nil-expected-class/metadata non-testable -->

| Field | Value |
|---|---|
| Status | completed |
| plan-file | `specs/changes/2026-08-26-nil-expected-class.scm` |
| updates | `specs/parser/differential-mode.md` (§ Expected Discrepancy Classes: one table row; § Operating Notes: one bullet) |
| adds | (none) |
| obsoletes | (none) |
| merge-into | (none) |
| new-spec-ids | (none) |
| modified-spec-ids | `mcp-tools/parser/expected-discrepancy-classes` (new row + test obligations), `mcp-tools/parser/differential-operating-notes` (suppressed-by-rule list gains the `nil` entry) |
| retired-spec-ids | (none) |
| non-testable-sections | `mcp-tools-changes/nil-expected-class/metadata`, `.../summary`, `.../non-goals`, `.../decisions`, `.../next-steps` |

## Summary
<!-- spec-id: mcp-tools-changes/nil-expected-class/summary non-testable -->

`specs/parser/grammar.md` defines `nil := "nil" | "()"` and mandates that both parse to `Value::Nil`. lexpr's default reader has no such rule: it reads the bare token `nil` as `Symbol("nil")`. (`()` is already handled — lexpr reads it as `Null`, and `compare_values` treats `(Nil, Null)` as equal.) So every payload carrying a literal `nil` is reported at that position as a both-`Ok` divergence, e.g. `new=List lexpr=Cons path=1.atom`.

This is a representational difference in which the new parser holds the documented canonical form — precisely the definition of an *expected class* introduced in PR #3 — and it is the one remaining both-`Ok` class that fires on ordinary well-formed input. After PR #3 it costs a consumer one report per position-class (bounded by the budget) rather than one per parse, so the motivation is signal quality during the 0.3 window: a reader of the stderr line cannot tell this apart from a real bug.

## Expected class: `nil`
<!-- spec-id: mcp-tools-changes/nil-expected-class/rule -->

Permanent home: the table in `specs/parser/differential-mode.md` § Expected Discrepancy Classes (`mcp-tools/parser/expected-discrepancy-classes`) gains one row:

| New parser | lexpr | Rule |
|---|---|---|
| `Value::Nil` | `lexpr::Value::Symbol("nil")` | equal |
| `Value::List(items)` | improper cons chain whose final cdr is `Symbol("nil")` | compared element-wise as a proper list |

**Amended during execution (2026-08-27, `q-tail-nil`).** The second row was added when the `(nil . nil)` obligation failed: the grammar reads a `nil` tail as `()`, so `(a . nil)` is the proper list `(a)` for the new parser, while lexpr yields an improper cons ending in `Symbol("nil")`. Same root cause, different shape — a list/pair structure difference the atom rule cannot reach. `compare_list` accepts a `Symbol("nil")` tail exactly as it accepts `Null`; any other symbol tail stays a reported discrepancy.

The asymmetry rule carries over unchanged: new-parser `Symbol("nil")` against lexpr `Nil`, `Null`, or `Symbol("nil")` remains a reported discrepancy at `atom`, because it would mean the new parser failed to apply its own grammar.

Implementation: one arm in `is_expected_class`:

```rust
(Value::Nil, lexpr::Value::Symbol(s)) => &**s == "nil",
```

`compare_values` is otherwise untouched; the existing `(Nil, Nil)` and `(Nil, Null)` arms stay where they are.

Test obligations, all `covers!([SpecItem::McpToolsParserExpectedDiscrepancyClasses])`:

- `(a nil)` produces no discrepancy (integration, `Callback` sink).
- `(a (b (c nil)))` produces no discrepancy.
- `(nil . nil)` produces no discrepancy.
- The reverse pairs (new `Symbol("nil")`, lexpr `Nil`) and (new `Symbol("nil")`, lexpr `Null`) fed to `compare_values` **are** reported at `atom`. (Unit test in `differential.rs`. The pair (new `Symbol("nil")`, lexpr `Symbol("nil")`) is a plain symbol match and is correctly not a discrepancy — it is not part of the asymmetry rule.)
- ~~`(a NIL)` (case differs) **is** reported~~ — **corrected during execution**: `NIL` is an ordinary symbol to both parsers, so `(a NIL)` never diverges at all. The byte-exactness obligation is `is_expected_class(Nil, Symbol("NIL")) == false` on the rule function directly.
- `(a . nil)` and `(a b . nil)` produce no discrepancy (tail rule); a `List` against a chain ending in `Symbol("other")` is still reported at `atom`.

Also: § Operating Notes "Suppressed by rule" list gains a bullet for `nil`, pointing at the table.

## Test-suite consequence
<!-- spec-id: mcp-tools-changes/nil-expected-class/test-consequence -->

PR #3's class-dedup integration tests (`tests/parser_differential.rs`) use `(a nil)` / `(a b nil)` as the both-`Ok` divergence that distinguishes `1.atom` from `2.atom`. Once `nil` is an expected class those inputs stop diverging and three tests become vacuous or fail:

- `class_dedup_distinguishes_divergence_position`
- `class_dedup_evicts_least_recently_seen_class`
- (indirectly) any future test that copies the pattern

They must be re-pointed at a divergence that survives — **not deleted**. Candidates verified during PR #3's probing, all `Err`/`Ok` at path `.`, so they cannot distinguish list positions: bignum, `#nil`, `#\c`, `#(…)`, `#x1F`, `1.`, `|s y|`, `1e400`. No both-`Ok` divergence at a list position remains once `nil` is suppressed, so the position-distinguishing tests must instead construct `Discrepancy` reports directly and exercise the class cache through `record_discrepancy_if_diverging`'s inputs — or, cleaner, through a `pub(crate)` seam that takes a prebuilt `Discrepancy`. Decision recorded as `q-test-seam`.

## Non-goals
<!-- spec-id: mcp-tools-changes/nil-expected-class/non-goals non-testable -->

- No change to how the new parser reads `nil` — that is `grammar.md`'s call and it is not in question.
- No general "lexpr symbol vs new-parser special form" rule. `nil` is the only bare token the grammar reserves; `#t`/`#f` already agree with lexpr.
- No new spec-id. The row lives under the existing `expected-discrepancy-classes` heading and its test obligations.

## Decisions
<!-- spec-id: mcp-tools-changes/nil-expected-class/decisions non-testable -->

| Id | Question | Decision | Alternatives considered |
|---|---|---|---|
| `q-case` | Match `nil` case-insensitively? | **No** — byte-exact `"nil"`, matching `grammar.md`. `NIL` is a symbol to the new parser and to lexpr alike, so it is not a divergence anyway. | Case-insensitive (rejected: would diverge from the grammar). |
| `q-tail-nil` | `(nil . nil)` still diverged after the atom arm: treat a `Symbol("nil")` improper tail as `Null`, or report it? | **Treat as `Null`** (second table row). Same root cause as the atom case; reporting `(a . nil)` would be exactly as misleading as reporting `(a nil)`. Scoped to the `nil` symbol only — other symbol tails remain structural discrepancies. | Report it and narrow the spec obligation (rejected: leaves a known-representational report in the 0.3 signal). |
| `q-test-seam` | How do the position-distinguishing class-dedup tests get a both-`Ok` divergence once `nil` is suppressed? | **Add a `pub(crate) fn record_discrepancy(report: Discrepancy)`-style seam** that enters the pipeline after `compare` (dedup → class-dedup → budget → dispatch), and move the two affected tests to the `differential.rs` unit-test module, constructing reports with explicit paths. Keeps them testing the real dedup path with the real caches. | Keep them as integration tests using a hand-made lexpr mismatch (none exists that parses `Ok` on both sides at a list position); mark them `#[ignore]` (rejected: hides coverage). |

## Next steps
<!-- spec-id: mcp-tools-changes/nil-expected-class/next-steps non-testable -->

1. `/make-plan` → `specs/changes/2026-08-26-nil-expected-class.scm`. Suggested order: spec row + operating-notes bullet → unit test for the reverse pairs → implementation arm → re-point the two class-dedup tests via the `q-test-seam` seam → integration tests for `(a nil)` and friends → `cargo test` + coverage.
2. Land together with `/spec-change-archive` of `2026-08-26-differential-report-hazards` so `specs/changes/archive/` is created once.
