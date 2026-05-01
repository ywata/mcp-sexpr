# Source-Position-Tracking Parser — Decisions Log

Decisions resolving the open questions in `2026-04-30-source-position-parser.md`.
Recorded at plan-execution time as the artifact for the `(doc "*-decision")` outputs.

## 1. Bootstrap Spec-Trace Infrastructure (q-bootstrap)

**Decision:** Bootstrap in this plan.

The `specs/`, `spec-files.txt`, `build.rs`, and `spec-trace` build+runtime deps are
created as part of Phase B of this plan. A precursor change spec would be pure
overhead — there's no work to defer.

## 2. `lexpr` Re-export (q-reexport)

**Decision:** Do **not** re-export `lexpr` as `mcp_tools::lexpr`.

Consumers already declare `lexpr = "0.2"` in their own `Cargo.toml`. A re-export
would create an ambiguous import surface (`mcp_tools::lexpr::Value` vs the real
`lexpr::Value`) and complicate the 1.0 removal. Migration window is short enough
that an extra Cargo entry is not a real burden.

## 3. Verbose Discrepancy-Mode Interface (q-verbose)

**Decision:** Support both env var and API.

- Env var: `MCP_TOOLS_DIFFERENTIAL_PARSE=verbose` for ops-side toggling without
  redeploy.
- API: `set_differential_mode(DifferentialMode::On { sink, verbose: true })` for
  programmatic control.

Cost is one extra match arm in the parser of the env var and one extra struct
field on `DifferentialMode::On`. Worth it.

## 4. `tracing` Sink in 0.3 (q-tracing)

**Decision:** Defer. Ship only `Stderr` and `Callback` sinks in 0.3.

Consumers using `tracing` wrap their own callback that forwards to the appropriate
`tracing::event!`. Adding a built-in tracing sink would add a heavyweight optional
dep to a crate already at 9 feature flags. If real demand emerges, revisit in 0.4.

## 5. Position Indexing Convention (q-indexing)

**Decision:** Store 1-indexed canonical line/column. Provide an explicit
`Position::lsp() -> (u32, u32)` accessor that returns 0-indexed.

Most error-message use cases (terminal diagnostics, compiler-style errors) want
1-indexed; that becomes the canonical form. LSP servers and editor-position
consumers call `lsp()` to get the form they need. Storing two forms in parallel
fields is wasteful when one is a trivial conversion of the other.

`Position` fields:
```rust
pub struct Position {
    pub line: u32,        // 1-indexed
    pub column: u32,      // 1-indexed
    pub byte_offset: u32,
}

impl Position {
    pub fn lsp(&self) -> (u32, u32) {
        (self.line.saturating_sub(1), self.column.saturating_sub(1))
    }
}
```
