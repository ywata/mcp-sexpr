# `SpannedNode` keyword/symbol discrimination as contract (2026-08-30)

- **Date:** 2026-08-30
- **Status:** completed (2026-08-30) — plan: `specs/changes/2026-08-30-spanned-variant-discrimination.scm`
- **Type:** test + spec hardening, developed falsification-gated (see § Method). No implementation change — the behavior already holds.
- **Impact:** low. Prose added to one existing `##` section, test obligations under two existing spec-ids, no new public API and no behavioral change.
- **Supersedes:** none. Follow-up to `546afa7` (PR #6), which pinned the same class of contract on the `Value` path.
- **Origin:** surfaced while reading mcp-compose PR #79 (`ywata/mcp-compose#79`, 0.2.0 "symbols, not strings") after the mailbox thread `symbols-not-strings`. `546afa7` pinned four behaviors mcp-compose named as contract, all on the `parse_value` → `Value` path. PR #79's surface-language reader turns out to run on the *spanned* path instead, matching `SpannedNode` variants directly — a fifth reliance neither side had named.

## Change Metadata
<!-- spec-id: mcp-tools-changes/spanned-variant-discrimination/metadata non-testable -->

| Field | Value |
|---|---|
| Status | completed |
| plan-file | `specs/changes/2026-08-30-spanned-variant-discrimination.scm` |
| updates | `specs/parser/value-types.md` (§ Spanned Type: variant-discrimination paragraph + test obligations) |
| adds | (none) |
| obsoletes | (none) |
| merge-into | (none) |
| new-spec-ids | (none) |
| modified-spec-ids | `mcp-tools/parser/spanned-type` (discrimination contract + obligations), `mcp-tools/parser/parse-value-with-positions` (equivalence-test input list extended) |
| retired-spec-ids | (none) |
| non-testable-sections | `mcp-tools-changes/spanned-variant-discrimination/metadata`, `.../summary`, `.../method`, `.../gate-results`, `.../coverage-caveat`, `.../non-goals`, `.../decisions`, `.../next-steps` |

## Summary
<!-- spec-id: mcp-tools-changes/spanned-variant-discrimination/summary non-testable -->

`specs/parser/value-types.md` § Spanned Type says `SpannedNode` "mirrors `Value` shape exactly except that `List` and `Pair` recurse into `Spanned`". That sentence is about *shape*. It is silent on whether the atom variants discriminate the same way `Value`'s do — specifically whether a `:k` token reaches a consumer as `SpannedNode::Keyword` in every position, and whether a bare word reaches it as `SpannedNode::Symbol`.

No test in the suite asserts either variant. Every `SpannedNode` assertion in `tests/parser_types.rs` and `tests/parser_positions.rs` destructures `List` and then checks spans; the `Keyword` and `Symbol` arms of `Spanned::into_value` / `to_value` (`src/parser/types.rs:304-305`, `:324-325`) are likewise unasserted. The one indirect guard is `parse_value_with_positions_into_value_matches_parse_value` (`tests/parser_api.rs:35`), whose input list includes `(:k "v")` — but it asserts only that the two parsers *agree*, so an identical regression on both paths passes it, and it has no keyword in value position.

mcp-compose 0.2.0 depends on this discrimination in four functions across two files:

| Consumer site | Positions matched |
|---|---|
| `src/lang/parser.rs::parse_expr` | `SpannedNode::Keyword(k)` in expression position → symbol literal |
| `src/lang/parser.rs::parse_pattern` | `SpannedNode::Keyword(s)` in pattern position → symbol literal pattern |
| `src/lang/parser.rs::parse_app` | `SpannedNode::Keyword(k)` in head position |
| `src/lang/serde_sexpr.rs::parse_value_node` | `SpannedNode::Symbol(s)` and `::Keyword(_)` in value position |

The last is the reader that re-parses a `report :value …` record, so `(record :verdict :pass)` arriving from an LLM tool-use response depends on it. That is the same payload path `546afa7` already pinned on the `Value` side — the two paths are used at different layers of the same consumer, and only one of them is currently guarded.

This change adds no behavior. It states the contract and pins it.

## Variant discrimination on the spanned path
<!-- spec-id: mcp-tools-changes/spanned-variant-discrimination/contract -->

Permanent home: `specs/parser/value-types.md` § Spanned Type (`mcp-tools/parser/spanned-type`) gains a paragraph after the "mirrors `Value` shape exactly" sentence, stating:

- **Atom variants discriminate identically on both paths.** For any source text, the `SpannedNode` variant at a given position and the `Value` variant that `parse_value` yields at the same position are the same variant carrying the same payload. `parse_value_with_positions` and `parse_value` differ only in whether spans and comments are retained; neither collapses, promotes, nor reinterprets an atom the other does not.
- **The mirror is position-independent.** `:k` yields `SpannedNode::Keyword("k")` and a bare word yields `SpannedNode::Symbol("w")` wherever they appear — standalone, in key position, in value position, in head position, nested in a sub-form, as a bare list element, or in a dotted tail. This is the spanned-path restatement of `mcp-tools/parser/keyword-canonicalization`, which is written in terms of `Value`.
- **`into_value` preserves the variant.** `Spanned::into_value` and `to_value` map `SpannedNode::Keyword(s) -> Value::Keyword(s)` and `SpannedNode::Symbol(s) -> Value::Symbol(s)`. Neither is coerced to `String`, and neither is normalized into the other.

Cross-reference both ways: the `keyword-canonicalization` section gains a pointer noting the property holds on the spanned path too.

## Method: falsification-gated TDD
<!-- spec-id: mcp-tools-changes/spanned-variant-discrimination/method non-testable -->

Classic red→green does not apply to this change: there is no implementation to drive, so every test below passes the moment it is written. A red phase asserting otherwise would be theater.

The red phase that *does* carry information is **falsification**. This change exists because a spec-id can be covered by a test that pins a different property than the one a consumer depends on — `mcp-tools/parser/spanned-type` is covered today and the gap survived anyway. Writing a test proves it passes; it does not prove it would fail if the property broke. So each obligation is developed as a three-beat cycle:

1. **Write** the test. It goes green immediately — expected, not evidence.
2. **Falsify.** Apply the named mutation to the parser, run the test, and require it to **fail**. Record the observed failure message.
3. **Revert** the mutation, re-run, and require green again.

A test that stays green under its mutation is not pinning the property and must be strengthened before the goal completes. Mutations are applied to the working tree and reverted within the same goal — none is committed. `git diff --exit-code src/` is clean at every goal boundary.

The mutations:

| Id | Site | Mutation | Property it breaks |
|---|---|---|---|
| `m-lex` | `src/parser/reader.rs:339` | `SpannedNode::Keyword(s)` → `SpannedNode::Symbol(s)` | keyword tokens reach the spanned tree as keywords |
| `m-into-kw` | `src/parser/types.rs:305` (`into_value`) | `=> Value::Keyword(s)` → `=> Value::String(s)` | `into_value` preserves the keyword variant |
| `m-into-sym` | `src/parser/types.rs:304` (`into_value`) | `=> Value::Symbol(s)` → `=> Value::String(s)` | `into_value` preserves the symbol variant |
| `m-to-value` | `src/parser/types.rs:324-325` (`to_value`, the borrowing twin) | both arms → `Value::String(...)` | `to_value` preserves the variants |

A mutation that also reddens pre-existing tests is fine — the gate asks whether the *new* test is live, not whether it is the only one. `m-to-value` is the exception worth watching: if **no** test fails under it, the `to_value` arms are unpinned entirely, which is a finding to record and close rather than a reason to skip the mutation.

## Test obligations
<!-- spec-id: mcp-tools-changes/spanned-variant-discrimination/obligations -->

Each group below is one falsification cycle: write → falsify with the named gate → revert.

**Cycle 1 — variant positions.** `covers!([SpecItem::McpToolsParserSpannedType])`, in `tests/parser_types.rs`. Gate: `m-lex`.

- `parse_value_with_positions("(record :verdict :pass :note \"ok\")")` — assert `SpannedNode::Keyword` at both the key index and the value index, and `SpannedNode::String` at the string value. The value-position assertion is the one with no current equivalent.
- `parse_value_with_positions("(record :verdict pass)")` — assert `SpannedNode::Symbol("pass")` in value position.
- Head and nested positions: `(sub :k :v)` nested inside an outer form — assert `SpannedNode::Symbol` at the head and `SpannedNode::Keyword` at both inner atoms.
- Standalone `:solo` and bare `word` at top level.
- A dotted tail `(:a . :b)` — assert `SpannedNode::Pair` with `Keyword` on both sides.

**Cycle 2 — `into_value` preservation.** Same spec-id and file. Gates: `m-into-kw`, then `m-into-sym`, each reverted before the next.

- For each form in cycle 1, assert the `Value` variant matches the `SpannedNode` variant it came from, rather than only that the tree is a list.
- Both gates must redden this group; a gate that reddens only cycle 1's tests means the preservation assertions are riding on the parse assertions and must be separated.

**Cycle 3 — `to_value` preservation.** Same spec-id and file. Gate: `m-to-value`.

- Exercise the borrowing `to_value` on a keyword- and symbol-bearing form, asserting the same preservation property.
- If `m-to-value` reddens nothing before this cycle is written, record that the `to_value` arms were entirely unpinned — that is the finding this cycle closes.

**Cycle 4 — equivalence inputs.** `covers!([SpecItem::McpToolsParserParseValue, SpecItem::McpToolsParserParseValueWithPositions])`, in `tests/parser_api.rs`. Gate: `m-lex`.

- Extend the input list of `parse_value_with_positions_into_value_matches_parse_value` with `(:k :v)`, `(:k v)` and `(head :k :v)`.
- Expected gate behavior differs here and is the point: `m-lex` hits both parsers, so this test may stay **green** under it. That is not a failure of the cycle — it is the demonstration that equivalence alone cannot catch a symmetric regression, which is why cycles 1–3 exist. Record the observation; do not strengthen this test to compensate.

Do not delete or weaken `spanned_node_recurses_into_spanned_for_lists` — the recursion property it pins is independent of variant discrimination.

## Gate results (recorded 2026-08-30)
<!-- spec-id: mcp-tools-changes/spanned-variant-discrimination/gate-results non-testable -->

| Cycle | Gate | Result | Evidence |
|---|---|---|---|
| 1 — variant positions | `m-lex` | **red** | `left: "Symbol" right: "Keyword"` at the value-position assertion |
| 2 — `into_value` | `m-into-kw` | **red** | `left: String("verdict") right: Keyword("verdict")` |
| 2 — `into_value` | `m-into-sym` | **red** | `left: String("record") right: Symbol("record")` |
| 3 — `to_value` | `m-to-value` (pre-write) | **nothing red** | full suite green, all 13 targets, with *both* arms mutated |
| 3 — `to_value` | `m-to-value` (post-write) | **red** | `left: String("record") right: Symbol("record")` |
| 4 — equivalence | `m-lex` | **green, expected** | both parsers share the mutated site; they agree on the wrong answer |

Two results carry information beyond "the gate worked":

- **`to_value` was entirely unpinned.** Cycle 3's falsify-first step replaced both the `Keyword` and `Symbol` arms of the borrowing `to_value` with `Value::String` and the whole suite stayed green. `into_value` had partial protection through the `Value`-path tests; its twin had none. This is the gap the cycle closed, and it was found by the gate rather than by reading code.
- **`m-into-kw` also reddened three `Value`-path keyword tests** from `546afa7`, which shows `parse_value` routes through `into_value` — the two paths share that code. Useful to know: a single regression there hits both, which is precisely why cycle 4's equivalence check cannot catch it.

Cycle 4 behaved as designed. A green gate there is the argument for cycles 1–3 existing, not a defect to fix, and the test was deliberately left unstrengthened.

## Coverage-tooling caveat
<!-- spec-id: mcp-tools-changes/spanned-variant-discrimination/coverage-caveat non-testable -->

`mcp-tools/parser/spanned-type` is **already reported as covered**, by `spanned_node_recurses_into_spanned_for_lists`. Adding obligations under it will not move the coverage number, and `bin/check_coverage.sh` could not have detected this gap in the first place — the spec-id had a test, that test just asserted a different property.

The falsification gate in § Method is the compensating control: it cannot make the coverage number honest, but it does establish per-obligation that each new test would redden if its property broke — which is the assurance the coverage number was mistakenly being read as providing.

This is the third gap in this family found by reading a consumer rather than by running the coverage script (`get_kw_value`: no spec-id at all, caught by inspection; bare symbol in value position: caught by a consumer message; this one: caught by reading a consumer's diff). The recurring shape is a spec-id whose section states several properties and whose tests pin one of them. Whether that warrants a convention change — finer-grained spec-ids, or per-obligation checklists — is out of scope here and recorded as `q-granularity`.

Unrelated but worth not conflating: repo coverage currently reports 83.0% (39/47) with all 8 uncovered under `mcp-tools/pretty/*`. That is an uncommitted local edit to `bin/check_coverage.sh` dropping `--features format-pretty`, not a real gap.

## Non-goals
<!-- spec-id: mcp-tools-changes/spanned-variant-discrimination/non-goals non-testable -->

- No *committed* change to `SpannedNode`, `Spanned`, the reader, or any public signature. The behavior already holds; this pins it. The § Method mutations touch `src/` transiently and are reverted inside the goal that applies them (`q-mutation-scope`).
- No new spec-id, and no new `##` section in `value-types.md`. See `q-new-spec-id`.
- No charset change. mcp-compose explicitly asked that the loose `is_symbol_continue` keyword charset stay as-is, and `546afa7` pinned it as deliberate.
- Not a response to a reported defect. mcp-compose reports 2053/0 against rev `7942795` and needs no change from us.
- No audit of the remaining `Value`/`SpannedNode` atom variants (`Nil`, `Bool`, numeric, `String`) for the same gap. Plausibly worth doing; not in this change.

## Decisions
<!-- spec-id: mcp-tools-changes/spanned-variant-discrimination/decisions non-testable -->

| Id | Question | Decision | Alternatives considered |
|---|---|---|---|
| `q-new-spec-id` | New spec-id (e.g. `mcp-tools/parser/spanned-variant-discrimination`) or elaborate `mcp-tools/parser/spanned-type`? | **Elaborate the existing one.** The contract is a property of the type already specified there, not a separate concept, and conventions place spec-ids only on `##` headings — a new id means a new sibling section largely restating § Spanned Type. Follows the `2026-08-26-nil-expected-class` precedent (`new-spec-ids: (none)`, obligations added to an existing id). | A new spec-id would give a distinct coverage target and make the gap visible to `check_coverage.sh` (rejected here: fragments a coherent section; the visibility win is real but is better addressed by `q-granularity` than by one ad-hoc split). |
| `q-equivalence-vs-direct` | Is extending the `into_value` equivalence test enough on its own? | **No.** Equivalence proves the two paths agree; it passes if both regress identically, which is exactly what a shared-reader refactor would produce. Direct variant assertions on the spanned path are the actual pin; the equivalence extension is complementary. | Extend only the equivalence test (rejected: does not pin what mcp-compose matches on). |
| `q-granularity` | Three gaps in a row were spec-ids that had a test pinning a different property than the one a consumer relied on. Change the convention? | **Out of scope, recorded not resolved.** Options include finer-grained spec-ids per property, or an explicit obligations checklist per section that review checks. Wants its own change spec. | Decide it here (rejected: it is a repo-wide convention change, not a parser fix, and would dwarf this change). |
| `q-tdd` | The user asked for a TDD approach, but there is no implementation to drive — every test here passes on arrival. What does TDD mean for a characterization change? | **Falsification-gated cycles** (§ Method): write → mutate the parser to break the property → require red → revert → require green. The red phase tests the test, not the code. This targets the exact failure that produced this change spec: a covered spec-id whose test pinned a different property. | Classic red→green with a stubbed-out parser arm (rejected: the stub is the mutation, minus the discipline of reverting it, and invites committing a broken arm). Skip the red phase and write tests directly (rejected: it is what produced the gap being closed, three times running). |
| `q-mutation-scope` | Mutations touch `src/`, which this change otherwise does not. Risk of one being committed? | **Accepted with a boundary check.** Mutations live in the working tree only and are reverted inside the goal that applied them; `git diff --exit-code src/` must be clean at every goal boundary, and the final verify goal re-checks it. | A separate mutation-testing harness or `cargo-mutants` (rejected for this change: four hand-named mutations at known lines, and adding a tool dependency dwarfs the change). Copy the parser to a scratch crate (rejected: the tests must run against the real target). |
| `q-notify` | Tell mcp-compose before or after this lands? | **After**, on the existing `symbols-not-strings` thread, with the rev — matching how `546afa7` was communicated. They are unblocked either way: rev `7942795` remains correct for them and PR #79 is green. | Notify at proposal time (rejected: nothing for them to act on until it lands). |

## Next steps
<!-- spec-id: mcp-tools-changes/spanned-variant-discrimination/next-steps non-testable -->

1. `/make-plan` → `specs/changes/2026-08-30-spanned-variant-discrimination.scm`. Goal order: spec paragraph + cross-reference in `value-types.md` → cycle 1 (variant positions, gate `m-lex`) → cycle 2 (`into_value`, gates `m-into-kw`/`m-into-sym`) → cycle 3 (`to_value`, gate `m-to-value`) → cycle 4 (equivalence inputs, gate `m-lex`, green-under-gate expected) → `cargo test` + `bin/check_coverage.sh` + `git diff --exit-code src/`. Each test goal carries its own falsification gate; none completes on a green-only run.
2. Land as a second commit on `chore/archive-change-specs` / PR #6 rather than amending `546afa7` — mcp-compose already has that rev and may pin it.
3. Post the resulting rev on the `symbols-not-strings` mailbox thread (`q-notify`).
