# iter_list Rejects Non-Lists

- **Date**: 2026-11-09
- **Status**: completed
- **Type**: behavioral bug fix (spec gap + spec/code mismatch). Public signatures unchanged; calls that succeed today on non-list input fail.
- **Impact**: consumers of `mcp_tools::iter_list` and `mcp_tools::parse_str_list`. All known consumers were asked and need nothing (see Consumer Survey).
- **Supersedes**: none

## Change Metadata

| Field | Value |
|---|---|
| Status | completed |
| plan-file | `specs/changes/2026-11-09-iter-list-rejects-non-list.plan` |
| updates | `specs/parser/api.md` (new § List Iteration), `specs/parser/value-types.md` (§ List Representation, last paragraph) |
| adds | (none) |
| obsoletes | (none) |
| merge-into | (none) |
| new-spec-ids | `mcp-tools/parser/list-iteration` |
| modified-spec-ids | `mcp-tools/parser/list-representation` (helper sentence corrected) |
| retired-spec-ids | (none) |
| non-testable-sections | `mcp-tools-changes/iter-list-rejects-non-list/summary`, `.../motivation`, `.../target-list-iteration`, `.../target-list-representation`, `.../consumer-survey`, `.../implementation`, `.../rationale`, `.../next-steps`, `.../completion-record` |
| consumer-notice | granter for mailboxer, composer, planner (asked in msg 1305; answers 1309 grant, 1310 mailboxer, 1311 composer, 1312 planner, relayed in 1313): owed after merge, the completing commit. "we changed iter_list and parse_str_list to return Err for non-lists; you need do nothing" — each consumer confirmed it does not rely on the old behavior; planner and mailboxer asked for the change. Other consumers: (none known) |

## Summary
<!-- spec-id: mcp-tools-changes/iter-list-rejects-non-list/summary non-testable -->

`iter_list(&Value)` returns `Ok` with an empty iterator for any value that is not a `List`, `Nil` or `Pair` (`_ => Vec::new()` in `src/lib.rs`), and walks a `Pair` chain while silently dropping an improper tail: `(a . b)` yields `[a]`. `parse_str_list` inherits both. No spec states the non-list case, and two specs already say `iter_list` rejects a dotted pair. This change states the contract — `List` iterates its items, `Nil` is empty, every other variant is an error — and makes the code follow it. The deprecated `_lexpr` variants are unchanged.

## Motivation
<!-- spec-id: mcp-tools-changes/iter-list-rejects-non-list/motivation non-testable -->

- mailboxer reported (msg 1301, forwarded by granter in 1302) that `(send ... :attach "a")` succeeded with no attachments in mcp-mailbox: `iter_list` turned the scalar `"a"` into an empty list, and the file was silently dropped. Reproduction: `iter_list(&parse_value("\"a\"").unwrap()).unwrap().count() == 0`.
- planner (1312): `(goal "x" :depends "y")` silently drops the dependency today, through `parse_str_list`.
- The signature returns `Result`, so callers reasonably expect `Err` on a shape mismatch.
- `specs/parser/value-types.md` § List Representation says the helpers "return `Some` only for `List`, never for `Pair`", and `specs/match-form/api.md` (Design Rationale, "Why is `Value::Pair` rejected?") says rejecting a dotted pair "matches the behavior of every existing helper (`get_kw_value`, `iter_list`)". The code contradicts both.

## Target: List Iteration
<!-- spec-id: mcp-tools-changes/iter-list-rejects-non-list/target-list-iteration non-testable -->

New `##` section in `specs/parser/api.md`, placed after § Keyword Argument Extraction, with spec-id `mcp-tools/parser/list-iteration` (testable). Content to write:

```rust
pub fn iter_list(value: &Value) -> Result<impl Iterator<Item = Value>>;
pub fn parse_str_list(value: &Value) -> Result<Vec<String>>;
```

- `iter_list`:
  - `Value::List(items)` → `Ok`, iterating clones of `items` in order.
  - `Value::Nil` → `Ok`, empty (the empty list `()`).
  - Any other variant — `Pair` (dotted pair or improper list), `Bool`, `Integer`, `Float`, `String`, `Symbol`, `Keyword`, and any numeric-tower variant — → `Err` with the message `expected list, got <variant>`, where `<variant>` is the same lowercase name `match_form` errors use (`"string"`, `"symbol"`, `"pair"`, …). No item is yielded and nothing is truncated.
- `parse_str_list` is `iter_list` followed by a per-item string check. It inherits the rule above (the `expected list` error, with context naming `parse_str_list` permitted), and keeps its existing error for a non-string item (`expected string item in list`).
- A proper list written with a redundant dot, `(a b . (c))`, parses to `List` (§ List Representation) and therefore iterates normally.
- The deprecated `iter_list_lexpr` / `parse_str_list_lexpr` keep their current behavior; they are removed in 1.0.

Test obligations (each test carries `covers!([SpecItem::McpToolsParserListIteration])`):

1. `(a b c)` → 3 items, in order; `()` → `Ok`, 0 items.
2. Each scalar variant (`"a"`, `1`, `1.5`, `#t`, `foo`, `:k`) → `Err` whose message contains `expected list` and the variant name.
3. `(a . b)` → `Err` naming `pair`; an improper list `(a b . c)` → `Err` (no partial `[a, b]`).
4. `parse_str_list`: `("a" "b")` → `["a","b"]`; `"a"` → `Err` (the mailboxer reproduction, inverted); `(a . b)` → `Err`; `("a" 1)` → `Err` `expected string item in list`.
5. The rustdoc on `iter_list` states the rule (replace "For non-list inputs this returns an empty iterator").

## Target: List Representation
<!-- spec-id: mcp-tools-changes/iter-list-rejects-non-list/target-list-representation non-testable -->

In `specs/parser/value-types.md` § List Representation (`mcp-tools/parser/list-representation`), replace the last paragraph

> The `iter_list` and `as_list` helpers on `Value` return `Some` only for `List`, never for `Pair`. Code that wants to walk improper lists handles `Pair` explicitly.

with

> `Value::as_list` returns `Some` only for `List`, never for `Pair`. The free function `iter_list` accepts `List` and `Nil` and returns an error for every other variant, `Pair` included (`specs/parser/api.md` § List Iteration). Code that wants to walk improper lists handles `Pair` explicitly.

`specs/match-form/api.md`'s "Why is `Value::Pair` rejected?" paragraph becomes true as written; no edit.

## Consumer Survey
<!-- spec-id: mcp-tools-changes/iter-list-rejects-non-list/consumer-survey non-testable -->

Asked through granter (lisper 1305): does any caller (1) pass a non-list and expect `Ok` with no items, or (2) pass a dotted pair or improper list and expect its elements? All answered no to both.

- **mcp-grant** (1309, ada4c6e): calls neither function.
- **mcp-mailbox** (1310, 2c8d785): no `parse_str_list`; `iter_list` at `src/server.rs:581` behind a guard that already refuses non-lists (made redundant), and `src/slot_render.rs:173` on its own server's answers. Wants a dotted pair to become an error.
- **mcp-compose** (1311, fdce902, pins 546afa7): one `iter_list` call in a test, on a proper list; its own `s_items` copy is untouched and only its comment will be reworded at the next re-pin.
- **mcp-planner** (1312, pins f5d0503): all uses go through `parse_str_list_compat`, which already maps `Err` to `InvalidFieldType`; improper lists reach `iter_list` as nested `Pair` and are truncated today. Asked for the change; the re-pin is planner's.

## Implementation Notes
<!-- spec-id: mcp-tools-changes/iter-list-rejects-non-list/implementation non-testable -->

1. Spec: add § List Iteration to `specs/parser/api.md`; edit the § List Representation paragraph; `cargo build` regenerates `SpecItem` with `McpToolsParserListIteration`.
2. Tests first (they fail against the current code for obligations 2–4).
3. Code: `iter_list` matches `List` and `Nil`, otherwise `Err(anyhow!("expected list, got {}", variant_name(value)))`. Reuse the existing `variant_name` in `src/match_form/mod.rs` (make it `pub(crate)`, or move it beside `Value`) rather than adding a second copy. Update the rustdoc.
4. Verify: `cargo test --all-features`, `cargo clippy -- -D warnings`, `cargo fmt --check`, `cargo doc --no-deps` (`-D warnings`), `bin/check_coverage.sh` (100%).

No crate version bump: consumers pin by git revision.

## Design Rationale
<!-- spec-id: mcp-tools-changes/iter-list-rejects-non-list/rationale non-testable -->

- **`Err`, not empty.** An absent keyword is already `Ok(None)` from `get_kw_value`, so callers have a separate, explicit way to say "missing means empty". A scalar where a list belongs is malformed input, and both reported failures were silent data loss.
- **`Pair` rejected, not walked.** Walking a pair chain and dropping the tail loses data silently, and the existing specs already promise rejection. mailboxer and planner both prefer the error.
- **`Nil` accepted.** `()` is the empty list; rejecting it would break every caller that passes an empty list.
- **`_lexpr` variants left alone.** They are deprecated and removed in 1.0; changing them would widen this change for no consumer.

## Next Steps
<!-- spec-id: mcp-tools-changes/iter-list-rejects-non-list/next-steps non-testable -->

1. Review this proposal, then `/make-plan` (or implement by hand with `Status: completed` in the finalizing commit).
2. After merge, send granter the completing commit for mailboxer, composer and planner (consumer-notice). Archiving waits for the maintainer.

## Completion Record (2026-11-09)
<!-- spec-id: mcp-tools-changes/iter-list-rejects-non-list/completion-record non-testable -->

- **Plan**: namespace `4646167e-14f8-491e-ab68-ab51e7125617`, tree `main`, 4 atomic goals, all complete (run via mcp-compose `plan-and-run`).
- **Spec**: `specs/parser/api.md` gained § List Iteration (`mcp-tools/parser/list-iteration`); the § List Representation paragraph in `specs/parser/value-types.md` was corrected. `spec-files.txt` unchanged.
- **Tests**: `tests/list_iteration.rs`, 10 tests, each with `covers!([SpecItem::McpToolsParserListIteration])`. Against the old code 5 failed — `iter_list_rejects_every_scalar_variant_by_name`, `iter_list_rejects_dotted_pair`, `iter_list_rejects_improper_list_without_truncating`, `parse_str_list_rejects_scalar`, `parse_str_list_rejects_dotted_pair` — and 5 passed (the behavior kept: lists, `()`, redundant dot, string lists, non-string item).
- **Code**: `iter_list` matches `List` and `Nil`; every other variant is `expected list, got <variant>` using `match_form`'s `variant_name` (now `pub(crate)`). The Pair-walking branch is gone; the rustdoc states the rule and its example shows the error. `parse_str_list` and the `_lexpr` variants are unchanged in source.
- **Checks**: `cargo test --all-features` 326 passed, 0 failed (316 before + 10); doctests 13 passed; `cargo clippy -- -D warnings`, `cargo fmt --check`, `cargo doc --no-deps` (`-D warnings`) clean; `bin/check_coverage.sh`: check-macros clean, 50 / 50 testable specs = 100.0%; reference check finds no change-spec path outside `specs/changes/`. (`cargo clippy --all-features --tests -- -D warnings`, which CI does not run, reports 9 lints in `pretty`, `prompt` and `interactive` code this change does not touch.)
- **Consumer notice**: owed after merge — the completing commit, to granter for mailboxer, composer and planner. Archive waits for the maintainer.
