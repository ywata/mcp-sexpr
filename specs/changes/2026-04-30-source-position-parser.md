# Source-Position-Tracking S-Expression Parser

- **Date**: 2026-04-30
- **Status**: proposed
- **Type**: feature (multi-release migration)
- **Impact**: high — eventually removes `lexpr::Value` from the public API in 1.0
- **Supersedes**: none

## Change Metadata

| Field | Value |
|---|---|
| Status | proposed |
| plan-file | (none) |
| updates | (none — this is the first parser spec) |
| adds | `specs/parser/grammar.md`, `specs/parser/value-types.md`, `specs/parser/source-positions.md`, `specs/parser/api.md`, `specs/parser/differential-mode.md`, `specs/migration/lexpr-deprecation.md` |
| obsoletes | (none) |
| merge-into | (none) |
| new-spec-ids | `mcp-tools/parser/grammar`, `mcp-tools/parser/string-escapes`, `mcp-tools/parser/comments`, `mcp-tools/parser/value-type`, `mcp-tools/parser/spanned-type`, `mcp-tools/parser/numeric-tower`, `mcp-tools/parser/keyword-canonicalization`, `mcp-tools/parser/list-representation`, `mcp-tools/parser/spans`, `mcp-tools/parser/comment-retention`, `mcp-tools/parser/parse-value`, `mcp-tools/parser/parse-value-with-positions`, `mcp-tools/parser/lexpr-conversion`, `mcp-tools/parser/api-deprecation`, `mcp-tools/parser/differential-mode`, `mcp-tools/parser/discrepancy-reporting`, `mcp-tools/parser/discrepancy-deduplication`, `mcp-tools/migration/phase-1-deprecation`, `mcp-tools/migration/phase-2-window`, `mcp-tools/migration/phase-3-removal`, `mcp-tools/migration/numeric-tower-loss`, `mcp-tools/migration/lexpr-conversion-lossy`, `mcp-tools/parser/design-rationale`, `mcp-tools/parser/alternatives-considered` |
| modified-spec-ids | (none) |
| retired-spec-ids | (none) |
| non-testable-sections | `mcp-tools/parser/design-rationale`, `mcp-tools/parser/alternatives-considered` |

## Summary
<!-- spec-id: mcp-tools-changes/source-position-parser/summary non-testable -->

Replace `lexpr` as the canonical S-expression parser in `mcp-tools` with a hand-rolled, position-tracking reader that produces a new owned value type. Migration runs across three releases (0.3, 0.4, 1.0). During the first phase, both parsers run side-by-side at runtime; structural disagreements are deduped and reported as bugs in the new parser. Once stable, `lexpr` is removed from the public API and from the dependency tree.

## Motivation
<!-- spec-id: mcp-tools-changes/source-position-parser/motivation non-testable -->

Downstream MCP servers (notably `mcp-compose`) need line/column references in error output to make S-expression diagnostics actionable for both humans and LLM authoring loops. `lexpr::Value` is a closed enum without per-node identity, so positions cannot be attached after the fact; spans must be produced by the parser and carried in the value type. Owning the value type also unblocks comment retention, which is required by the upcoming pretty-printer feature.

Source: `docs/mcp-tools-feature-requests.md` request (1), priority P0.

## Scope
<!-- spec-id: mcp-tools-changes/source-position-parser/scope non-testable -->

**In scope:**
- Hand-rolled position-tracking S-expression reader.
- New owned value types (`Value`, `Spanned`).
- Differential validation mode against `lexpr::from_str`.
- Bidirectional `lexpr::Value` ↔ `Value` conversion.
- `#[deprecated]` annotations on existing `lexpr::Value`-based public API.
- Phased rollout 0.3 → 0.4 → 1.0.

**Out of scope (separate change specs):**
- Pretty-printer (feature request 2).
- Form-shape pattern matcher (feature request 3).
- AST builders / quasiquotation (feature request 4).

## Value Type Design
<!-- spec-id: mcp-tools/parser/value-type -->

Two types, not one:

```rust
pub enum Value {
    Nil,
    Bool(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Symbol(String),
    Keyword(String),                 // canonical: no leading colon
    List(Vec<Value>),                // proper lists; common case
    Pair(Box<(Value, Value)>),       // dotted pairs only
}

pub struct Spanned {
    pub value: SpannedNode,
    pub span: Span,
    pub leading_comments: Vec<Comment>,
    pub trailing_comments: Vec<Comment>,
}

pub enum SpannedNode {
    // same shape as Value, but List/Pair recurse into Spanned
}
```

`Value` is for construction, manipulation, and formatting (the common case, no span overhead). `Spanned` is for parsing and diagnostic reporting. `Spanned::into_value()` strips spans and comments.

### Numeric Tower
<!-- spec-id: mcp-tools/parser/numeric-tower -->

Stripped to `Integer(i64)` and `Float(f64)`. Rationals, complex, and big integers are not represented natively. Rationale: no current consumer uses them; halving the surface area is worth the trade-off.

### Keyword Canonicalization
<!-- spec-id: mcp-tools/parser/keyword-canonicalization -->

`:foo` always parses to `Keyword("foo")` (no leading colon stored). The dual-form `lexpr` behavior (sometimes `Keyword`, sometimes `Symbol(":foo")`) is eliminated. The `normalize_kw` helper goes away in the new API.

### List Representation
<!-- spec-id: mcp-tools/parser/list-representation -->

Proper lists are stored as `Vec<Value>`. Cons-cell traversal (`car`/`cdr`) is gone for the common case; consumers walk via indexing, slicing, and iterators. `Pair(Box<(Value, Value)>)` is reserved for genuine dotted pairs.

## Grammar
<!-- spec-id: mcp-tools/parser/grammar -->

The parser accepts a subset of R7RS-flavored S-expressions matching the inputs current consumers send to `lexpr`. Specifically:

- Atoms: integers, floats, strings, symbols, keywords (`:foo`), `#t`/`#f`, `nil`/`()`.
- Lists: proper lists `(a b c)` and dotted pairs `(a . b)`.
- Quotes: `'expr`, `` `expr ``, `,expr`, `,@expr` desugar to standard forms.

### String Escapes
<!-- spec-id: mcp-tools/parser/string-escapes -->

Recognized escapes: `\\`, `\"`, `\n`, `\r`, `\t`. Output through `quote_str` round-trips through the new parser. Other backslash sequences are an error (matches existing escape policy in `developer-guide.md`).

### Comments
<!-- spec-id: mcp-tools/parser/comments -->

Line comments (`; ...`) and block comments (`#| ... |#`) are recognized by the parser. They are discarded by `parse_value` (returning `Value`), and retained on adjacent nodes by `parse_value_with_positions` (returning `Spanned`).

## Source Positions
<!-- spec-id: mcp-tools/parser/spans -->

```rust
pub struct Position {
    pub line: u32,            // 1-indexed for human display
    pub column: u32,          // 1-indexed
    pub byte_offset: u32,
}

pub struct Span {
    pub start: Position,
    pub end: Position,
}
```

Every node in the `Spanned` tree carries a `Span` covering its full source extent (open paren to close paren for lists). All position fields are `u32`; sources >4GB are not supported.

### Comment Retention
<!-- spec-id: mcp-tools/parser/comment-retention -->

Each `Spanned` node carries `leading_comments` (comments preceding the node) and `trailing_comments` (same-line comments following). This data is required by the future pretty-printer and is preserved on parse-format round-trip when consumers use the spanned variant.

## Public API
<!-- spec-id: mcp-tools/parser/parse-value -->

The existing `parse_value(s: &str) -> Result<lexpr::Value>` is **kept** in 0.3 with deprecation, and gets a sibling returning the new type:

```rust
pub fn parse_value(input: &str) -> Result<Value>;                       // new signature in 0.3
pub fn parse_value_with_positions(input: &str) -> Result<Spanned>;      // new in 0.3

#[deprecated(note = "use parse_value -> Value; lexpr::Value is removed in 1.0")]
pub fn parse_value_lexpr(input: &str) -> Result<lexpr::Value>;          // renamed previous parse_value
```

The signature change to `parse_value` is the breaking change — existing callers either migrate to the new `Value` type or switch to `parse_value_lexpr` for the deprecation window.

### Lexpr Conversion
<!-- spec-id: mcp-tools/parser/lexpr-conversion -->

```rust
impl From<lexpr::Value> for Value { /* lossy on numeric tower */ }
impl From<Value> for lexpr::Value { /* total */ }
```

The `From<lexpr::Value>` direction is **lossy**: lexpr rationals, complex numbers, and bignums beyond `i64` range produce an error (not silent truncation). The reverse direction is total.

### API Deprecation
<!-- spec-id: mcp-tools/parser/api-deprecation -->

All existing functions returning or accepting `lexpr::Value` (`get_kw_value`, `get_kw_str`, `require_kw_str`, `iter_list`, `parse_str_list`, `parse_text_ref`, `render_text_ref`) are duplicated for `Value` and the `lexpr::Value` versions are marked `#[deprecated(note = "use Value-based API; lexpr::Value is removed in 1.0")]`.

## Differential Validation Mode
<!-- spec-id: mcp-tools/parser/differential-mode -->

In 0.3, every call to `parse_value` and `parse_value_with_positions` runs both parsers and structurally compares results. The consumer always receives the new parser's result; discrepancies are reported via a configurable sink without affecting program behavior.

```rust
pub enum DifferentialMode {
    Off,
    On { sink: DiscrepancySink },
}

pub enum DiscrepancySink {
    Stderr,
    Callback(Arc<dyn Fn(&Discrepancy) + Send + Sync>),
}

pub fn set_differential_mode(mode: DifferentialMode);
```

Default: `On { sink: Stderr }`. Override via env var `MCP_TOOLS_DIFFERENTIAL_PARSE=on|off`.

### Discrepancy Reporting
<!-- spec-id: mcp-tools/parser/discrepancy-reporting -->

A `Discrepancy` carries the input string (hashed by default for privacy; full string only with verbose flag), both parsed values, and the structural path to the first divergence. Reporting is non-fatal — the consumer never observes the discrepancy as an error.

### Discrepancy Deduplication
<!-- spec-id: mcp-tools/parser/discrepancy-deduplication -->

Reporter maintains a bounded LRU of input hashes (default 1024 entries). Each unique input is reported at most once per process lifetime. Configurable bound; flushable via API for long-running processes.

## Migration Phasing
<!-- spec-id: mcp-tools/migration/phase-1-deprecation -->

**0.3 — new parser, new types, lexpr deprecated.**
- New `Value` and `Spanned` types ship.
- New `parse_value` returns `Value`; `parse_value_with_positions` returns `Spanned`.
- Previous `lexpr::Value`-returning function renamed `parse_value_lexpr`, marked deprecated.
- All keyword-extraction helpers gain `Value`-based counterparts; `lexpr::Value` versions deprecated.
- Differential mode default-on, stderr sink, hashed inputs.
- `lexpr` remains in `Cargo.toml` as a runtime dep (used by differential validation and conversion).

<!-- spec-id: mcp-tools/migration/phase-2-window -->
**0.4 — migration window.**
- Deprecated APIs still present.
- Differential mode default-off; opt-in via env var or API.
- Bug-fix-only window for parser divergences reported during 0.3.

<!-- spec-id: mcp-tools/migration/phase-3-removal -->
**1.0 — lexpr removed.**
- All `lexpr::Value`-based APIs deleted.
- `lexpr` removed from `Cargo.toml`.
- Bidirectional conversion functions removed.
- Drop criteria: zero open discrepancy bugs; full release cycle of 0.3 in production with no consumer-reported issues; differential CI corpus passing.

### Known Lossy Conversions
<!-- spec-id: mcp-tools/migration/numeric-tower-loss -->

`lexpr::Value` containing rationals, complex numbers, or integers outside `i64` range cannot round-trip through `Value`. The `From<lexpr::Value> for Value` conversion errors on these inputs rather than silently truncating. Documented as a precondition for migration.

<!-- spec-id: mcp-tools/migration/lexpr-conversion-lossy -->

Consumers who currently rely on lexpr's numeric tower must either switch to encoding numbers as strings or as `(rational num denom)` forms before migrating. No automated migration is provided — this is a known limitation accepted at design time.

## Design Rationale
<!-- spec-id: mcp-tools/parser/design-rationale non-testable -->

Why hand-rolled rather than fork lexpr or pull in a parser combinator:

- **Forking lexpr** was rejected to avoid maintaining a fork of an external dependency.
- **Side-table approach** (parsing with our own lexer alongside lexpr) creates a two-parsers-to-keep-in-sync problem on a grammar (Lisp reader) with non-trivial edge cases. The shadow-parser pattern is fragile.
- **Parser combinators** (chumsky, winnow, nom) add a substantial compile-time dep for a grammar simple enough to recursive-descent in a few hundred LOC.
- **Hand-rolled** keeps deps tight, gives us full control over comment retention and span semantics, and produces code that's straightforward to maintain.

Why two value types rather than one with optional spans:

- 95% of operations (formatting, building, walking) don't care about spans; forcing `Option<Span>` everywhere is noise.
- Constructing `Value` programmatically (for codegen, builders) stays trivial.
- The 5% that does care explicitly opts in via `Spanned`.

Why default-on for differential mode in 0.3:

- Opt-in shadow modes notoriously gather no signal.
- 2x parse cost is acceptable for one release cycle in a crate where parsing is rarely the bottleneck.
- Without real consumer traffic, the lexpr-removal decision in 1.0 has no data to ground it.

## Alternatives Considered
<!-- spec-id: mcp-tools/parser/alternatives-considered non-testable -->

| Alternative | Rejected because |
|---|---|
| Fork lexpr to add spans | Maintenance of an external fork; pace-of-upstream uncertain. |
| Side-table parser keyed by structural path, lexpr remains canonical | Two parsers to keep grammar-compatible; structural paths break under tree transforms. |
| Parser combinator (chumsky/winnow/nom) | Heavy dep for a grammar that's straightforward recursive-descent. |
| Single `Value` type with `Option<Span>` | Pollutes the common case (programmatic construction, formatting) with span machinery for the 5% case. |
| Keep `lexpr::Value` in API forever, add positions via newtype wrapper | Cannot attach spans recursively without owning the value type; future features (comments, custom keyword semantics) blocked. |

## Open Questions
<!-- spec-id: mcp-tools-changes/source-position-parser/open-questions non-testable -->

1. **Spec infrastructure bootstrap.** This change spec is the first in `mcp-sexpr`; `specs/`, `spec-files.txt`, `build.rs`, and the `spec-trace` build dependency are not yet set up. Decide whether to bootstrap them as part of this plan or as a precursor change spec.
2. **Public re-export of `lexpr::Value`.** During 0.3, should `mcp_tools::lexpr` re-export the lexpr crate so consumers don't need a separate Cargo dependency during migration? Or is it cleaner to require them to add `lexpr` themselves?
3. **Verbose discrepancy mode interface.** Env var, API, or both? `MCP_TOOLS_DIFFERENTIAL_PARSE=verbose` is one option.
4. **Discrepancy reporting in async contexts.** Stderr sink is fine for sync MCP servers; tokio-based servers may want to route through `tracing`. Consider a `tracing` feature flag for the sink.
5. **Position indexing convention.** 1-indexed line/column for human display is the most common choice. LSP servers downstream may expect 0-indexed; document the conversion or provide both.

## Next Steps
<!-- spec-id: mcp-tools-changes/source-position-parser/next-steps non-testable -->

1. Resolve open questions (especially infrastructure bootstrap and `lexpr` re-export).
2. Run `/make-plan` against this change spec to generate the goal tree (`specs/changes/2026-04-30-source-position-parser.scm`).
3. Plan execution adds the permanent spec files listed in `adds`, registers their paths in `spec-files.txt`, and produces the implementation under `src/parser/`.
4. Differential CI corpus is seeded from existing fixtures; expand as consumer code is migrated.
