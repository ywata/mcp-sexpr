# Differential-parse reporting: expected-class suppression, class-keyed dedup, bounded stderr (2026-08-26)

- **Date:** 2026-08-26
- **Status:** completed (2026-08-26) — plan: `specs/changes/2026-08-26-differential-report-hazards.scm`
- **Type:** defect fix (behavioural, diagnostics only) + documentation. No change to any parse result or to the public `parse_value` signature.
- **Impact:** low–medium. Touches `src/parser/differential.rs` only (comparison, dedup key, stderr sink), plus the `specs/parser/differential-mode.md` spec and a version statement. Consumers see fewer, shorter, bounded stderr reports; the `Discrepancy` struct handed to `Callback` sinks is unchanged.
- **Supersedes:** none. Refines the differential-mode design landed by `specs/changes/2026-04-30-source-position-parser.md` (decisions `q-verbose`, `q-tracing` in `2026-04-30-source-position-parser-decisions.md`).
- **Origin:** external defect report from the deep-rev project, filed as `docs/2026-08-26-differential-parse-stderr.md` (D1–D3, A1). All four claims were verified against `f09e3f9` — see **Verification** below. That report is the evidence record; this document is the change of record.

## Change Metadata
<!-- spec-id: mcp-tools-changes/differential-report-hazards/metadata non-testable -->

| Field | Value |
|---|---|
| Status | completed |
| plan-file | `specs/changes/2026-08-26-differential-report-hazards.scm` |
| updates | `specs/parser/differential-mode.md` (comparison rules, dedup key, stderr format, version applicability); `specs/migration/lexpr-deprecation.md` (cross-reference to version applicability only); `Cargo.toml` (version — see `q-version`) |
| adds | (none) |
| obsoletes | (none) |
| merge-into | (none) |
| new-spec-ids | `mcp-tools/parser/expected-discrepancy-classes`, `mcp-tools/parser/discrepancy-class-deduplication`, `mcp-tools/parser/stderr-report-budget`, `mcp-tools/parser/version-applicability` |
| modified-spec-ids | `mcp-tools/parser/discrepancy-reporting` (hashed-mode line omits tree contents), `mcp-tools/parser/discrepancy-deduplication` (second key), `mcp-tools/parser/differential-operating-notes` (keyword class moves from "expected" to "suppressed") |
| retired-spec-ids | (none) |
| non-testable-sections | `mcp-tools-changes/differential-report-hazards/metadata`, `.../summary`, `.../verification`, `.../non-goals`, `.../decisions`, `.../next-steps`; permanent: `mcp-tools/parser/version-applicability` |

## Summary
<!-- spec-id: mcp-tools-changes/differential-report-hazards/summary non-testable -->

The 0.3 differential wrapper is **default-on, writes to stderr, and reports a difference the spec itself calls expected** (`Keyword("k")` vs lexpr `Symbol(":k")`). For a consumer whose payloads all carry keyword arguments — every MCP tool server — this yields one full report per parse, with both parse trees printed verbatim, unbounded. A stdio MCP server whose client does not drain stderr blocks on `eprintln!` once the 64 KB pipe fills, mid-request, silently. The consumer misfiled that as a bug in their own write path.

Four changes, in order of leverage:

1. **Expected-class suppression (D1).** `compare_values` treats the new parser's `Keyword(k)` as equal to lexpr's `Symbol(":" + k)`. This is a representational difference the spec already acknowledges, not a parse divergence. After this, a keyword-bearing payload produces **zero** reports.
2. **Class-keyed deduplication (D2).** A second bounded LRU keyed by *discrepancy class* — `(path, new-variant, lexpr-variant)` — so N distinct inputs exhibiting one grammar-level difference produce one report, not N.
3. **Hashed-mode report omits tree contents (D3a).** When `verbose: false`, the stderr line prints only the variant kinds and the path (`new=Keyword lexpr=Symbol path=1.atom`), never `{:?}` of the values. The rationale for hashing the input — payloads carry secrets — applies equally to the parse of that input.
4. **Per-process stderr report budget (D3b).** The `Stderr` sink emits at most `STDERR_REPORT_BUDGET` (64) reports per process, then one terminal `... further reports suppressed` line, then nothing. Together with (3), worst-case stderr output from this crate is bounded well below the 64 KB pipe buffer, so an undrained stderr can no longer block the consumer regardless of input volume. The `Callback` sink is unbudgeted (the consumer owns it).

Plus **A1**: the spec states the version range in which default-on applies, and `Cargo.toml` is aligned with it (`q-version`).

**Why not default `Off` or land `tracing`?** Both discard the mode's purpose or add a dependency. The consumer's own closing note says they would rather have D1 and D3 fixed and the mode back on. (1)+(3)+(4) keep the signal and remove the hazard; `q-tracing` stays deferred.

## Verification against `f09e3f9`
<!-- spec-id: mcp-tools-changes/differential-report-hazards/verification non-testable -->

| Report claim | Code / spec evidence |
|---|---|
| Default-on, `Stderr`, `verbose: false` | `src/parser/differential.rs:239` `default_mode()` |
| `Keyword` vs `Symbol(":k")` reported at `atom` | `compare_values` `:429` matches only `Keyword`/`Keyword`; the pair falls to `_ => Atom`. Yet `specs/parser/differential-mode.md:179` lists keyword normalisation as an **expected** category — the code never implemented that expectation |
| Dedup keyed by input hash only | `:322-323` `guard.record(sha256(input))`; `path` is not part of the key |
| Trees printed regardless of `verbose` | `write_stderr` `:369-380` always emits `format!("Ok({:?})", v)` for both sides; `verbose` gates only the `input=` line |
| No bound on stderr volume | no counter anywhere in the module; every cache miss prints |
| Version ambiguity | `Cargo.toml:3` `version = "0.2.0"`; spec line 39 "The 0.3 release ships default-on"; `specs/migration/lexpr-deprecation.md:7` "Phase 1 — 0.3 Surface" |

The consumer's measured figures (181 KB per 100-record call; hang at 50 records with undrained stderr; 0 rows committed at 400) are consistent with ~1.8 KB per report × one report per parse.

## Expected discrepancy classes
<!-- spec-id: mcp-tools-changes/differential-report-hazards/expected-classes -->

Permanent home: `specs/parser/differential-mode.md`, new `##` section `mcp-tools/parser/expected-discrepancy-classes`, placed after `## Discrepancy Reporting`.

An *expected class* is a `(new, lexpr)` value pair that differs only in representation, where the new parser's reading is the documented canonical form. Expected classes are **not discrepancies**: `compare_values` returns `None` for them, so nothing is hashed, deduplicated, or dispatched.

The 0.3 expected classes:

| New parser | lexpr | Rule |
|---|---|---|
| `Value::Keyword(k)` | `lexpr::Value::Symbol(s)` where `s == format!(":{k}")` | equal |
| `Value::Keyword(k)` | `lexpr::Value::Keyword(k)` | equal (already) |

Nothing else is added. In particular the two directions are **not** symmetric: new-parser `Symbol(":k")` vs lexpr `Keyword("k")` remains a reported discrepancy, because it would mean the new parser failed to canonicalise a keyword — exactly the class of bug the mode exists to catch.

The **Operating Notes** entry "Keyword normalization … may disagree at the symbol/keyword variant boundary" moves from "expected discrepancy categories" to a new "suppressed by rule" list that points here.

Test obligations (all `covers!([SpecItem::McpToolsParserExpectedDiscrepancyClasses])`):
- `(tool :key "v")` produces no discrepancy (`compare` returns `None`).
- `(:a . :b)` dotted keywords produce no discrepancy.
- A keyword nested three lists deep produces no discrepancy.
- The reverse pair (new `Symbol(":k")`, lexpr `Keyword("k")`), constructed directly on the `compare_values` API, **is** reported at `atom`.

## Class-keyed deduplication
<!-- spec-id: mcp-tools-changes/differential-report-hazards/class-dedup -->

Permanent home: `## Discrepancy Deduplication` (`mcp-tools/parser/discrepancy-deduplication`) is extended; a new sub-heading and new spec-id `mcp-tools/parser/discrepancy-class-deduplication` is added as a sibling `##` section immediately after it.

A discrepancy's **class** is the triple:

```rust
struct DiscrepancyClass {
    path: StructuralPath,            // as reported, indices included
    new_kind: ValueKind,             // variant name of new_value, or the ParseErrorRepr.kind
    lexpr_kind: LexprKind,           // variant name of lexpr_value, or "Err"
}
```

`ValueKind` / `LexprKind` are small enums (not strings) enumerating the variant names, so a new `Value` variant is a compile-time error here rather than a silently-unclassified report.

Suppression rule — a discrepancy is dispatched only if **both** are true:

1. its input hash is not in the input LRU (existing rule, capacity 1024, unchanged), and
2. its class is not in the class LRU (**new**, default capacity 256).

Both caches are updated whenever a lookup misses, before dispatch. `set_discrepancy_dedup_capacity` continues to resize the input cache only; a new `set_discrepancy_class_dedup_capacity(usize)` resizes the class cache, with `0` disabling class dedup (every cache miss on the input side is reported — the pre-change behaviour).

The path keeps its indices: `1.atom` and `2.atom` are different classes. This is deliberate — collapsing indices would hide a divergence that appears at a *new position* in a structurally different payload, and the remaining volume is already bounded by the stderr budget.

Test obligations (`covers!([SpecItem::McpToolsParserDiscrepancyClassDeduplication])`):
- 100 distinct inputs that all diverge as `(1.atom, Integer, Number-bignum)` → the `Callback` sink is invoked exactly once.
- Two inputs diverging at `1.atom` and `2.atom` respectively → invoked twice.
- Same input twice → once (input LRU still works; class LRU does not double-count).
- `set_discrepancy_class_dedup_capacity(0)` → 100 distinct inputs → invoked 100 times.
- Class LRU eviction: capacity 2, three classes in rotation → the first class is reported again after eviction.

## Hashed-mode stderr line omits tree contents
<!-- spec-id: mcp-tools-changes/differential-report-hazards/hashed-line -->

Permanent home: modifies `### Stderr sink format` under `mcp-tools/parser/discrepancy-reporting`.

The stderr line depends on `verbose`:

```
# verbose: false  (the compiled-in default)
[mcp-tools differential] new=<kind> lexpr=<kind> path=<path>
  input-sha256=<hex>

# verbose: true
[mcp-tools differential] new=<repr> lexpr=<repr> path=<path>
  input=<source>
```

where `<kind>` is the variant name only (`Keyword`, `Symbol`, `List`, `Err(IntegerOutOfRange)`, …) — never the contents of a string, symbol, keyword, number, or error message — and `<repr>` is the existing `Ok(<one-line debug>)` / `Err(<kind>: <message>)` form.

Invariant: **in hashed mode, no byte of the source string, and no byte derived from it other than its SHA-256, reaches stderr.** Error messages are included in this rule because `ParseError` messages and lexpr error strings can quote the offending input.

The `Discrepancy` struct passed to `Callback` sinks is **unchanged** and still carries full values in both modes: a consumer who wires a callback has chosen where those bytes go.

Test obligations (`covers!([SpecItem::McpToolsParserDiscrepancyReporting])`):
- Hashed mode, input `(tool "SECRET-9f3a")` engineered to diverge (e.g. via a bignum sibling): captured stderr does not contain `SECRET-9f3a`.
- Verbose mode, same input: captured stderr does contain it.
- Hashed mode, an `Err` on one side: the line contains the error *kind* and not the message text.

(Stderr capture in tests goes through a `Callback` that mirrors `write_stderr`'s formatting into a buffer, or by extracting the line formatter into a pure `fn format_report(&Discrepancy, verbose: bool) -> String` and testing that directly. The latter is preferred — functional style, no process-global capture.)

## Per-process stderr report budget
<!-- spec-id: mcp-tools-changes/differential-report-hazards/stderr-budget -->

Permanent home: new `##` section `mcp-tools/parser/stderr-report-budget` in `differential-mode.md`, after `## Discrepancy Deduplication` and its class sibling.

The `Stderr` sink holds a process-wide monotonic counter. Behaviour:

- Reports 1..=`STDERR_REPORT_BUDGET` are written as specified above. `STDERR_REPORT_BUDGET = 64`.
- On the report that would be number `BUDGET + 1`, the sink writes exactly one terminal line and increments past the budget:
  ```
  [mcp-tools differential] report budget (64) exhausted; further reports suppressed. Use DiscrepancySink::Callback or MCP_TOOLS_DIFFERENTIAL_PARSE=off.
  ```
- Every subsequent report is dropped before formatting. No further stderr writes occur from this module for the life of the process.
- The counter is **never reset** by `set_differential_mode`; switching the sink to `Callback` and back does not refill the budget. (Rationale: the budget exists to bound total bytes on a stream the consumer may never read; a refill would reopen the hazard.)
- The budget applies to the `Stderr` sink only. `Callback` dispatch is unlimited.
- Ordering guarantee: the budget is checked *after* both dedup caches, so a budgeted report always corresponds to a fresh `(input, class)` pair.

Bound: 64 hashed-mode lines of ≤ ~200 bytes plus the terminal line is < 16 KB, i.e. less than a quarter of the smallest common pipe buffer. This is the property that removes the deadlock, and it holds independently of whether D1 and D2 fire.

Test obligations (`covers!([SpecItem::McpToolsParserStderrReportBudget])`):
- With the formatter extracted, drive the sink's decision function with 70 fresh `(input, class)` pairs: 64 `Emit`, 1 `EmitExhausted`, 5 `Drop`. (Model the decision as an enum returned by a pure function over `(counter, budget)` so the test does not touch the real stderr.)
- Switching mode to `Callback` and back after exhaustion: still `Drop`.
- Dedup-before-budget: a duplicate input after 63 reports does not consume report 64.

## Version applicability
<!-- spec-id: mcp-tools-changes/differential-report-hazards/version-applicability non-testable -->

Permanent home: new `##` section `mcp-tools/parser/version-applicability non-testable` at the top of `differential-mode.md`, directly after the overview, plus a one-line cross-reference from `specs/migration/lexpr-deprecation.md` Phase 1.

Text to land:

> The default-on policy applies to every crate version `>= 0.3.0, < 0.4.0`. Versions `< 0.3.0` pre-date the differential wrapper and ship no comparison at all; `0.4.0` and later ship `Off` by default. A consumer can determine whether they are inside the window from `Cargo.lock` alone.

And the alignment: `Cargo.toml` `version` becomes `0.3.0` when this change lands (`q-version`). Every phase-1 surface item in `lexpr-deprecation.md` is already present at `0.2.0`, so the bump is a labelling correction, not a feature release.

## Non-goals
<!-- spec-id: mcp-tools-changes/differential-report-hazards/non-goals non-testable -->

- **No change to any parse result.** The wrapper still never propagates a discrepancy; `parse_value`'s return is bit-identical before and after.
- **No `tracing` sink.** `q-tracing` remains deferred; the budget makes it unnecessary as a hazard fix.
- **No change to the compiled-in default (`On { Stderr, verbose: false }`).** See `q-default-sink`.
- **No async / non-blocking stderr.** Bounding volume is simpler and sufficient.
- **No change to `Discrepancy`'s fields** — callback consumers keep full-fidelity data.
- **No relocation of the origin report.** `docs/2026-08-26-differential-parse-stderr.md` stays where it is as the evidence record; it is not a spec and must not enter `spec-files.txt`.

## Decisions
<!-- spec-id: mcp-tools-changes/differential-report-hazards/decisions non-testable -->

| Id | Question | Decision | Alternatives considered |
|---|---|---|---|
| `q-keyword-rule` | Suppress the keyword class by normalising in `compare_values`, or report it under a distinct "expected" tag? | **Normalise in compare** — it is not a discrepancy, and a tagged report still costs bytes on stderr. | Tagged report (rejected: still unbounded until D2/D3b). |
| `q-class-key` | Include list indices in the class key? | **Yes.** Different position = different class. | Shape-only key (rejected: hides position-specific divergences; budget already bounds volume). |
| `q-default-sink` | Keep `Stderr` as the compiled-in default? | **Keep it**, now that its output is bounded (< 16 KB/process) and content-free in hashed mode. Default `Off` would end the 0.3 signal window early. | `Off` default (consumer's first ask); `tracing` sink (deferred, `q-tracing`). **Confirmed by owner 2026-08-26.** The external report argued for `Off`; declined for the reason given. |
| `q-budget-size` | 64 reports? | **64.** Enough to see several classes; total bytes stay far below any pipe buffer. Constant, not configurable — a setter would invite consumers to reopen the hazard. | 16 / 256 / configurable. |
| `q-err-in-hashed` | Include error *messages* in hashed mode? | **No**, kind only — messages can quote input. | Include (rejected: violates the hashed-mode invariant). |
| `q-version` | Bump `Cargo.toml` to `0.3.0` with this change, or document `0.2.x` as the 0.3-policy build? | **Bump to 0.3.0.** The spec already calls this surface 0.3 everywhere; the number is what's wrong. Semver-wise a diagnostics behaviour change under a default-on flag justifies a minor bump anyway. **Confirmed by owner 2026-08-26.** | Annotate spec only (rejected: leaves two version vocabularies in play). |

## Next steps
<!-- spec-id: mcp-tools-changes/differential-report-hazards/next-steps non-testable -->

1. ~~Owner confirms `q-default-sink` and `q-version`~~ — both confirmed 2026-08-26; all decisions are now closed.
2. `/make-plan` → `specs/changes/2026-08-26-differential-report-hazards.scm`. Goal ordering must respect spec-before-test: the four new permanent spec-ids are added to `specs/parser/differential-mode.md` (and `spec-files.txt` is already listing that file) **before** any test goal writes `covers!()` against them.
3. Implementation order = leverage order: expected-class suppression → hashed-line formatter extraction → budget → class dedup → version statement + `Cargo.toml` bump.
4. After completion, reply to the origin report: D1 fixed, D2 fixed, D3 fixed both halves, A1 fixed; note the `Callback` sink as the recommended wiring for a stdio server that wants unbounded reports through its own logger.
