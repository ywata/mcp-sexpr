# mcp-tools Feature Requests

Feature requests for the `mcp-tools` crate (repo: `mcp-sexpr`) that would benefit any MCP server building S-expression-based tooling. Each request is general — `mcp-compose` is the motivating consumer, but every MCP server that validates user-provided S-expr or produces S-expr diagnostic output has the same needs.

## Priority Summary

| # | Request | Priority | Notes |
|---|---|---|---|
| 1 | Source position tracking on parse | P0 (blocking) | Required for line/col references in error output |
| 2 | Structural pretty-printer | P1 | Stable formatting for diffs and round-tripping |
| 3 | Form-shape pattern matching helpers | P2 | Reduce boilerplate duplicated in every consumer |
| 4 | AST builders / quasiquotation | P3 | Quality-of-life for code generation; can be deferred |

Items are independent. (1) is the only one blocking concrete work in mcp-compose; (2)–(4) can land in any order.

---

## 1. Source Position Tracking on Parse

### Problem

`lexpr::Value` does not preserve source positions on parse. Consumers that want to report errors like *"expected `:body` after `:pred` at line 7, col 12"* cannot do so without re-parsing manually. For tools producing machine-consumed diagnostics (LLM authoring loops, LSP integration, CI), errors with structural paths only are much harder to act on than errors carrying line/col.

### Desired API

A position-preserving parse returning a position-decorated value:

```rust
pub struct Position {
    pub line: u32,
    pub col: u32,
    pub byte_offset: usize,
}

pub struct Span {
    pub start: Position,
    pub end: Position,
}

pub struct Spanned<T> {
    pub value: T,
    pub span: Span,
}

pub fn parse_value_with_positions(input: &str) -> Result<Spanned<lexpr::Value>>;
```

Every `cons` cell, atom, and literal in the parsed tree should carry a `Span`. Two viable strategies:

- **Decorated AST** — `Spanned<lexpr::Value>` recursively, spans on every node. Larger footprint, simpler API.
- **Side-table** — `(lexpr::Value, SourceMap)` mapping node identity to `Span`. Smaller footprint, marginally more API surface.

The decorated AST is more ergonomic; prefer it unless memory becomes a concern.

### Compatibility

Existing `parse_value` remains unchanged. The new API is additive.

### Feature gating

Default feature set, alongside `parse_value`. Gate behind a `positions` feature only if binary size is a real concern.

---

## 2. Structural Pretty-Printer

### Problem

`lexpr::to_string` produces output without stable layout — multiple top-level forms on one line, keyword args un-aligned, no indentation control. For consumers shipping S-expr as a config file format or generating S-expr from machine sources, unstable formatting causes:

- Noisy version-control diffs
- "Find this form in source" failing because output formatting drifts between runs
- Inconsistent output across tools in the same workspace

### Desired API

```rust
pub struct PrettyOpts {
    pub max_line_width: usize,     // default 80
    pub indent: usize,              // default 2 spaces
    pub align_keyword_args: bool,   // default true
    pub blank_line_between_top_forms: bool, // default true
}

impl Default for PrettyOpts { /* sensible defaults */ }

pub fn pretty_print(value: &lexpr::Value, opts: &PrettyOpts) -> String;
pub fn pretty_print_default(value: &lexpr::Value) -> String;
```

Layout rules to aim for:

- One top-level form per logical group; blank lines between top-level definitions
- Keyword args aligned vertically when wrapped onto multiple lines
- Lists wrapped at `max_line_width` with consistent indent
- Strings escaped via existing `quote_str` (round-trippable through `parse_value`)

Output must be deterministic: same input → same output, byte-for-byte.

### Feature gating

A new feature `format-pretty`. The existing `format` feature is for response formatting and should stay separate.

---

## 3. Form-Shape Pattern Matching Helpers

### Problem

Lowering S-expr to a typed AST involves boilerplate every consumer duplicates: check the head symbol, count positional args, look up keyword args, validate types. The current keyword-arg helpers (`get_kw_value`, `require_kw_str`) cover part of this; the surrounding "match this form shape" logic is rewritten everywhere.

### Desired API

A matcher for the common case of `(head <positional...> :k1 v1 :k2 v2 ...)`:

```rust
pub struct FormMatch<'a> {
    // internals: head + slice of positional + keyword index
}

impl<'a> FormMatch<'a> {
    pub fn head(&self) -> &str;
    pub fn positional(&self) -> &[&'a lexpr::Value];
    pub fn positional_at(&self, idx: usize) -> Result<&'a lexpr::Value>;
    pub fn keyword(&self, name: &str) -> Result<Option<&'a lexpr::Value>>;
    pub fn require_keyword(&self, name: &str) -> Result<&'a lexpr::Value>;
    // type-converting variants (compose with `extract` feature):
    pub fn keyword_str(&self, name: &str) -> Result<Option<String>>;
    pub fn require_keyword_str(&self, name: &str) -> Result<String>;
}

pub fn match_form<'a>(value: &'a lexpr::Value, expected_head: &str) -> Result<FormMatch<'a>>;
```

Usage:

```rust
let m = match_form(&value, "define")?;
let name = m.positional_at(0)?.as_symbol().context("define name")?;
let body = m.positional_at(1)?;
let max  = m.require_keyword("max")?;
```

### Feature gating

Could land in the default feature set (small surface) or under `extract` (since it composes with type-converting extraction). Author's call.

---

## 4. AST Builders / Quasiquotation

### Problem

Programmatic construction of S-expr (for desugaring, codegen, error suggestions) currently relies on string concatenation or manual `lexpr::Value` construction. String concat misses escaping; manual construction is verbose.

### Desired API

Two complementary forms — builder functions are simpler to ship, quasiquotation is more ergonomic.

**Builder functions** (small, no proc-macro):

```rust
pub fn cons(car: lexpr::Value, cdr: lexpr::Value) -> lexpr::Value;
pub fn list(items: Vec<lexpr::Value>) -> lexpr::Value;
pub fn keyword(name: &str) -> lexpr::Value;
pub fn symbol(name: &str) -> lexpr::Value;
pub fn string(s: &str) -> lexpr::Value;
pub fn integer(n: i64) -> lexpr::Value;
```

**Quasiquotation macro** (more ergonomic, proc-macro):

```rust
let define_form = sexpr!((define ~name ~body));
let lambda_form = sexpr!((lambda (~@params) ~@body_exprs));
```

`~expr` interpolates a value; `~@expr` splices a list. Modeled after Rust's `quote!` macro.

### Feature gating

- Builder functions in default feature set (small, no extra deps).
- `sexpr!` macro under a `quote` feature (proc-macro adds compile time).

### Priority

Lowest of the four. Consumers can work around with verbose but functional `lexpr` construction; this is purely ergonomic improvement.

---

## Notes on Adoption

These features are additive — none break existing API. mcp-compose will adopt each as it lands:

- (1) unblocks the type-checker error format work; consumed at parse time.
- (2) becomes the canonical formatter for the workflow file format.
- (3) reduces boilerplate in the AST lowering pass.
- (4) used in combinator desugaring (e.g. `:while` → recursive lambda).

No other consumer in this workspace is blocked on these.
