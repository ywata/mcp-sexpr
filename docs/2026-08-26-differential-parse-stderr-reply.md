# Reply: differential-parse reporting (D1–D3, A1)

- **Date**: 2026-08-26
- **In reply to**: `docs/2026-08-26-differential-parse-stderr.md` (deep-rev / `mcp-iter`)
- **Change of record**: `specs/changes/2026-08-26-differential-report-hazards.md`
- **Rev to pin**: the merge commit of https://github.com/ywata/mcp-sexpr/pull/3 once merged (implementation commit `dab838a` on `feature/differential-report-hazards` until then); crate version `0.3.0`

Thanks for the report — every claim reproduced against `f09e3f9`, and the misfiled-hang story is
exactly the failure mode we'd rather find in a report than in production. All four items are
addressed. No parse result changes; `parse_value` returns are bit-identical before and after.

## D1 — keyword class no longer reported — **fixed**

`Keyword("k")` (new parser) vs `Symbol(":k")` (lexpr) is now an *expected class*: the comparison
treats the pair as equal, so nothing is hashed, deduplicated, or dispatched. A keyword-bearing
payload produces **zero** reports. The rule is deliberately one-directional — a new-parser
`Symbol(":k")` against a lexpr `Keyword("k")` is still reported, since that would be a real
canonicalisation bug.

Spec: `specs/parser/differential-mode.md` § Expected Discrepancy Classes.

## D2 — deduplication by discrepancy class — **fixed**

A second bounded LRU (default capacity 256) keyed by `(path, new-kind, lexpr-kind)` runs after
the input-hash LRU. N distinct payloads exhibiting one grammar-level difference now yield one
report. Your 203 would have been 1. `set_discrepancy_class_dedup_capacity(0)` disables it;
`flush_discrepancy_dedup()` clears both caches.

Spec: § Discrepancy Class Deduplication.

## D3 — report contents and stderr volume — **fixed, both halves**

**Contents.** In hashed mode (the default) the stderr line now prints variant kinds only:

```
[mcp-tools differential] new=Err(IntegerOutOfRange) lexpr=Cons path=.
  input-sha256=…
```

No string, symbol, keyword, number, or error-message text reaches stderr — the invariant is that
nothing derived from the source other than its SHA-256 is written. Your captured filesystem path
would not appear today. `verbose` mode (opt-in) keeps the full trees and the raw input.

**Volume.** The `Stderr` sink is capped at **64 reports per process**. On the 65th it writes one
terminal line (`report budget (64) exhausted; further reports suppressed. …`) and then drops every
later report *before formatting*. Total stderr output from the crate is bounded under 16 KB — a
quarter of the smallest common pipe buffer — so an undrained stderr cannot block your request
loop, regardless of input volume and independently of whether D1/D2 fire. The counter is never
reset, including across `set_differential_mode` calls.

Spec: § Stderr sink format, § Stderr Report Budget.

## A1 — which version is default-on — **fixed**

`differential-mode.md` now opens with a **Version Applicability** section: default-on applies to
`>= 0.3.0, < 0.4.0`; `< 0.3.0` ships no comparison; `0.4.0+` ships `Off`. The crate is now
`0.3.0` — the `0.2.0` you saw was a pre-bump label on what was already the 0.3 surface. You can
answer "does this affect us?" from `Cargo.lock` alone.

## On your ask to default the sink to `Off`

We kept `On { Stderr, verbose: false }` as the compiled-in default. With D3 landed the sink is
bounded and content-free, which removes the hazard; defaulting to `Off` would end the 0.3 signal
window early, and your own closing note says you'd rather have the mode back on once D1/D3 are
fixed. That is the state now. `MCP_TOOLS_DIFFERENTIAL_PARSE=off` remains available and you can
drop it from your sandbox image when you pick up `0.3.0`.

## Recommendation for a stdio MCP server

If you want reports beyond the 64-report budget, or routed through your own logger, wire a
`Callback` sink at startup — it is unbudgeted, receives the full `Discrepancy` (both trees, hashed
or verbose input), and runs synchronously on the parsing thread:

```rust
use std::sync::Arc;
use mcp_tools::{set_differential_mode, DifferentialMode, DiscrepancySink};

set_differential_mode(DifferentialMode::On {
    sink: DiscrepancySink::Callback(Arc::new(|d| {
        tracing::warn!(path = %d.path, "mcp-tools differential discrepancy");
    })),
    verbose: false,
});
```

Do not call `parse_value` from inside the callback.
