# Pretty-Printer, Form-Match, and AST Builders

- **Date**: 2026-05-01
- **Status**: proposed
- **Type**: feature (additive, three independent items)
- **Impact**: medium — new modules and one new optional feature; no breaking changes
- **Supersedes**: none

## Change Metadata

| Field | Value |
|---|---|
| Status | proposed |
| plan-file | (none) |
| updates | (none) |
| adds | `specs/pretty/api.md`, `specs/match-form/api.md`, `specs/build/api.md` |
| obsoletes | (none) |
| merge-into | (none) |
| new-spec-ids | `mcp-tools/pretty/options`, `mcp-tools/pretty/pretty-print`, `mcp-tools/pretty/layout-rules`, `mcp-tools/pretty/keyword-alignment`, `mcp-tools/pretty/blank-line-between-top-forms`, `mcp-tools/pretty/determinism`, `mcp-tools/pretty/round-trip`, `mcp-tools/pretty/feature-gate`, `mcp-tools/pretty/design-rationale`, `mcp-tools/match-form/match-form`, `mcp-tools/match-form/form-match-type`, `mcp-tools/match-form/head`, `mcp-tools/match-form/positional`, `mcp-tools/match-form/keyword`, `mcp-tools/match-form/error-cases`, `mcp-tools/match-form/lifetime`, `mcp-tools/match-form/design-rationale`, `mcp-tools/build/cons`, `mcp-tools/build/list`, `mcp-tools/build/keyword`, `mcp-tools/build/symbol`, `mcp-tools/build/string`, `mcp-tools/build/integer`, `mcp-tools/build/design-rationale` |
| modified-spec-ids | (none) |
| retired-spec-ids | (none) |
| non-testable-sections | `mcp-tools/pretty/design-rationale`, `mcp-tools/match-form/design-rationale`, `mcp-tools/build/design-rationale` |

## Summary
<!-- spec-id: mcp-tools-changes/pretty-match-build/summary non-testable -->

Implement feature requests P1 (structural pretty-printer), P2 (form-shape matcher), and P3 (AST builder functions) from `docs/mcp-tools-feature-requests.md`. Three independent additive items, grouped here for one-pass landing. P3's `sexpr!` quasiquotation macro is **deferred** — proc-macros require a separate sub-crate and bring tooling complexity disproportionate to this batch.

## Motivation
<!-- spec-id: mcp-tools-changes/pretty-match-build/motivation non-testable -->

- **P1 (pretty-printer)**: `Value`'s `Display` impl produces compact, single-line output unsuitable for human-readable config files. Workflow definitions in `mcp-compose` and elsewhere need stable layout for diffs and round-tripping.
- **P2 (form-match)**: Every consumer that lowers S-expr to a typed AST repeats the same pattern: check head symbol, count positionals, look up keyword args, validate types. Centralising this removes a class of off-by-one bugs and gives consistent error messages.
- **P3 (builders)**: Programmatic construction of `Value` (codegen, desugaring, error suggestions) is currently verbose: `Value::List(vec![Value::Symbol("define".into()), Value::Symbol("x".into()), Value::Integer(42)])`. A handful of constructor helpers cut that to one line per node.

Source: `docs/mcp-tools-feature-requests.md` requests (1)–(4); request (1) was the previous change spec.

## Scope
<!-- spec-id: mcp-tools-changes/pretty-match-build/scope non-testable -->

**In scope:**
- Pretty-printer (P1) operating on `Value`, behind new `format-pretty` feature.
- Form-shape matcher (P2) operating on `Value`, in default feature set.
- AST builder functions (P3) producing `Value`, in default feature set.

**Out of scope (deferred):**
- `sexpr!` quasiquotation proc-macro (separate change spec).
- Pretty-printer support for `Spanned` with comment retention on output (separate change spec).
- Type-converting `FormMatch` variants that depend on the `extract` feature.
- Pretty-printer support for `lexpr::Value` (the type is being removed in 1.0; not worth wiring).

## Design Rationale
<!-- spec-id: mcp-tools-changes/pretty-match-build/rationale non-testable -->

Three features bundled in one change spec because:
- They are all small (one module each) and independent — bundling avoids three near-identical change-spec scaffolds.
- They share no interface; failure of one to land does not block the others.
- All three are additive; no migration concerns to coordinate.

Each feature gets its own permanent spec doc (`specs/pretty/api.md`, `specs/match-form/api.md`, `specs/build/api.md`) so the permanent documentation remains topical.

`format-pretty` is a separate Cargo feature (not folded into the existing `format` feature) because:
- The existing `format` feature is for response-message templates (`format_success`, `format_error`); it is a different concern.
- Pretty-printing is opt-in; consumers who only parse and pattern-match should not pay for layout code.

`match_form` and `build` go into the default feature set because:
- They have no extra dependencies.
- They are useful to every consumer of the parser — gating them behind a feature would just generate friction without benefit.
