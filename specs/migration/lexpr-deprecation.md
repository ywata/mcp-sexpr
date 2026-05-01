# Lexpr Deprecation and Migration

This document specifies the three-release migration that removes `lexpr::Value` from the public surface of `mcp-tools`. It is the consumer-facing source of truth for what changes in 0.3, what changes in 0.4, and what is gone in 1.0.

The phased rollout exists because consumers (notably `mcp-compose` and `mcp-planner`) currently take `mcp-tools` as a git dependency and use `lexpr::Value` throughout. A single-release rip-and-replace would force them all to migrate in lockstep with this crate; the deprecation window gives them a release cycle of overlap.

## Phase 1 — 0.3 Surface
<!-- spec-id: mcp-tools/migration/phase-1-deprecation -->

In the 0.3 release:

- New `Value` and `Spanned` types ship at the crate root (re-exported from `mcp_tools::parser`).
- `parse_value` returns `Value` (signature change — see `specs/parser/api.md`).
- `parse_value_with_positions` is added, returning `Spanned`.
- The previous `lexpr::Value`-returning parser is renamed `parse_value_lexpr` and marked `#[deprecated(note = "use parse_value -> Value; lexpr::Value is removed in 1.0")]`.
- Every keyword/list helper (`get_kw_value`, `get_kw_str`, `require_kw_str`, `iter_list`, `parse_str_list`, `parse_text_ref`) gains a `Value`-based version under the unsuffixed name; the original `lexpr::Value` version is renamed with a `_lexpr` suffix and marked deprecated.
- Bidirectional conversion impls ship: `From<Value> for lexpr::Value` (total) and `TryFrom<lexpr::Value> for Value` (lossy).
- Differential validation runs default-on, sink `Stderr`, hashed inputs.
- `lexpr` remains a runtime dependency in `Cargo.toml` because the differential validator and conversion impls both need it.
- `mcp-tools` does **not** re-export the `lexpr` crate. Consumers that need lexpr during the migration declare it directly: `lexpr = "0.2"`.

Consumer effort to upgrade to 0.3:

| Caller pattern | Required change |
|---|---|
| `parse_value(s)?` returning a `lexpr::Value` | Either migrate to `Value` or rename to `parse_value_lexpr`. |
| `get_kw_value(&v, "k")` where `v: &lexpr::Value` | Rename to `get_kw_value_lexpr`, or convert `v` via `Value::try_from(v.clone())?` and call the unsuffixed name. |
| Pattern-matching `lexpr::Value::Cons(...)` | No change needed — the type still exists. Migration to `Value::List` can be deferred to 0.4 work. |
| Custom code reading source positions | Switch to `parse_value_with_positions` — this is the only way to get spans. |

A workspace that wants to upgrade with zero structural change can add `_lexpr` everywhere and accept the deprecation warnings; the warnings come with `note` text pointing at the new functions.

## Phase 2 — 0.4 Migration Window
<!-- spec-id: mcp-tools/migration/phase-2-window -->

In the 0.4 release:

- All deprecated APIs from 0.3 remain present and remain deprecated.
- Differential mode default flips to `Off`. Consumers who want validation set `MCP_TOOLS_DIFFERENTIAL_PARSE=on` or call `set_differential_mode` explicitly.
- This is a bug-fix-only window for divergences reported during 0.3. New parser behavior is locked; only correctness fixes against the corpus and against new consumer reports land.
- `lexpr` continues to be a runtime dependency.

Decision criteria for cutting 0.4:

- Differential CI corpus passes with zero discrepancies on the locked parser.
- All open consumer-reported discrepancy bugs are closed.
- At least one full release cycle of 0.3 has shipped to a downstream consumer that exercised differential mode in a non-trivial way (mcp-compose's compile pipeline qualifies).

Consumer effort during 0.4:

- Migrate every `_lexpr`-suffixed call site to the `Value`-based equivalent.
- Replace `lexpr::Value::Cons(_)` pattern matches with `Value::List(_)` / `Value::Pair(_)` matches.
- Remove `lexpr = "0.2"` from `Cargo.toml` once the migration is complete.

## Phase 3 — 1.0 Removal
<!-- spec-id: mcp-tools/migration/phase-3-removal -->

In the 1.0 release:

- Every `_lexpr`-suffixed function is deleted.
- `From<Value> for lexpr::Value`, `TryFrom<lexpr::Value> for Value`, and `LexprConversionError` are deleted.
- `DifferentialMode`, `DiscrepancySink`, `Discrepancy`, `set_differential_mode`, `current_differential_mode`, `set_discrepancy_dedup_capacity`, and `flush_discrepancy_dedup` are deleted.
- `lexpr` is removed from `Cargo.toml`. `mcp-tools` no longer pulls `lexpr` into the dependency graph.

### Drop criteria

1.0 is gated on:

- Zero open discrepancy bugs against the new parser.
- A full 0.3 release cycle in production use across at least two downstream consumers, with no consumer-reported parser correctness issues.
- A passing differential CI corpus that exercises the lexpr-comparison path one final time before lexpr is removed.

If any of these is unmet, 1.0 is held; 0.5 / 0.6 etc. extend the migration window. There is no fixed calendar deadline.

## Numeric Tower Loss
<!-- spec-id: mcp-tools/migration/numeric-tower-loss -->

`lexpr::Value` admits four numeric variants beyond `i64` and `f64`:

- `Number::Rational { num, den }`
- `Number::Complex { re, im }` (in some lexpr feature combinations)
- `Number::BigInteger(..)` (integer literals beyond `i64`)
- Negative-zero floats and NaN (representable as `f64`, but with subtle equality semantics)

The new `Value` type stores only `Integer(i64)` and `Float(f64)`. `TryFrom<lexpr::Value> for Value` errors on any of the first three. NaN and negative zero round-trip through the `f64` variant unchanged.

When a consumer's source contains one of these forms (e.g., `1/2` in a payload), the conversion error is:

```rust
LexprConversionError::UnsupportedRational { num: 1, den: 2 }
LexprConversionError::UnsupportedComplex
LexprConversionError::BignumOutOfRange
```

These are surfaced through the parse `Result` chain when the conversion happens during differential validation, and through the explicit `try_from` call when the consumer is converting an existing `lexpr::Value`.

The new parser itself never produces these forms because its grammar does not recognize the syntax. A `lexpr::Value` containing one of them can only enter `mcp-tools` via:

- An explicit `try_from(lexpr_value)` from consumer code.
- A round-trip through `lexpr::from_str` outside this crate, then handed in (e.g., to differential validation).

## Lexpr Conversion Lossy Direction
<!-- spec-id: mcp-tools/migration/lexpr-conversion-lossy -->

Consumers who currently rely on lexpr's numeric tower must change their wire format before migrating. The crate provides no automated migration path because no one-size-fits-all conversion is correct:

| Original lexpr form | Recommended replacement |
|---|---|
| Rational `1/2` | Encode as `(rational 1 2)` form, or as a string `"1/2"` parsed application-side. |
| Complex `1+2i` | Encode as `(complex 1 2)` form, or as a string. |
| Bignum (e.g., 2^65) | Encode as a string `"36893488147419103232"`, or as a list of i64-fitting limbs. |

The `(rational ...)` style is preferable because it remains queryable: consumers can extract the parts via the same `iter_list` / index-based patterns they use elsewhere. The string-encoded style requires re-parsing application-side and loses S-expression-level structure.

Consumers should:

1. Audit S-expression payloads sent to and received from `mcp-tools` for any of the lossy forms.
2. Update producers to emit the chosen replacement form.
3. Update consumers to parse the replacement form (typically a small `iter_list` walk).
4. Run with `MCP_TOOLS_DIFFERENTIAL_PARSE=verbose` for one production cycle to confirm no lossy forms remain.
5. Upgrade the `mcp-tools` dependency to 0.3.

This is documented as a known limitation accepted at design time: the value of a tight numeric tower (simpler pattern matches, no bignum dependency, no rational semantics to argue about) outweighs the migration cost for the small number of consumers who use these forms.

## Consumer Migration Checklist
<!-- spec-id: mcp-tools/migration/consumer-checklist non-testable -->

- [ ] Bump `mcp-tools` to 0.3 in `Cargo.toml`.
- [ ] If you used `mcp_tools::parse_value`, decide: migrate to `Value` (recommended) or rename to `parse_value_lexpr`.
- [ ] Add `lexpr = "0.2"` to your own `Cargo.toml` if you need direct lexpr access (was previously transitive through `mcp-tools`).
- [ ] Audit S-expression payloads for lossy numeric forms (rationals, complex, bignums); migrate to `(rational ...)` / `(complex ...)` / string forms before turning differential mode off.
- [ ] Run with differential mode default-on for one production cycle; address any reported discrepancies.
- [ ] When upgrading to 0.4: migrate every `_lexpr`-suffixed call site to the unsuffixed (`Value`-based) version.
- [ ] When upgrading to 1.0: confirm no `_lexpr` symbols remain in your code; remove `lexpr` from your `Cargo.toml` if no longer needed for non-`mcp-tools` reasons.
