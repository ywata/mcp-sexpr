# Parser Public API

This document specifies the public Rust surface for the position-tracking parser in 0.3 — the new entry points, the deprecated `lexpr::Value`-based shims, and the bidirectional conversion impls. The migration phasing across 0.3 / 0.4 / 1.0 is in `specs/migration/lexpr-deprecation.md`.

## parse_value
<!-- spec-id: mcp-tools/parser/parse-value -->

The 0.3 release **changes the return type** of `parse_value`:

```rust
pub fn parse_value(input: &str) -> Result<Value>;
```

This is the only signature change in the public API. The previous `lexpr::Value`-returning version is renamed `parse_value_lexpr` (see "API Deprecation" below) and remains callable through the deprecation window. Callers who do nothing during the upgrade get a compile error pointing them at one of:

- Migrate to the new `Value` type (recommended).
- Switch to `parse_value_lexpr` to keep the old behavior with a deprecation warning.

`parse_value` discards comments and source positions; consumers needing either call `parse_value_with_positions` (below).

### Errors

`parse_value` returns `anyhow::Result<Value>`. The error chain includes a `ParseError` variant with structured fields when the failure originates from this parser:

```rust
pub enum ParseError {
    UnexpectedChar { ch: char, position: Position },
    UnterminatedString { position: Position },
    UnterminatedComment { position: Position },
    InvalidEscape { sequence: String, position: Position },
    OutOfRange { kind: NumericKind, position: Position },
    SourceTooLarge,
    Eof { expected: &'static str, position: Position },
}
```

When a `lexpr::from_str` failure surfaces (during differential validation), it is wrapped via `.context(...)` rather than converted into `ParseError` — the new parser is the source of truth, the old parser's errors are diagnostic only.

## parse_value_with_positions
<!-- spec-id: mcp-tools/parser/parse-value-with-positions -->

```rust
pub fn parse_value_with_positions(input: &str) -> Result<Spanned>;
```

Parses the input and returns the full `Spanned` tree, retaining source positions and adjacent comments per `specs/parser/source-positions.md`. Used by:

- Diagnostic emitters that need to point at a specific sub-expression.
- The future pretty-printer (which needs comment retention).
- The form-shape pattern matcher (which needs spans on each matched node).

This entry point also runs differential validation against `lexpr::from_str` when the differential mode is on (see `specs/parser/differential-mode.md`). Differential validation compares structural shape only; spans and comments are not part of the comparison.

`parse_value_with_positions(input).map(Spanned::into_value)` is observably equivalent to `parse_value(input)`. Use the spanned form when positions matter and the lighter form when they do not.

## Lexpr Conversion
<!-- spec-id: mcp-tools/parser/lexpr-conversion -->

Bidirectional `From` impls bridge the new `Value` type with `lexpr::Value`:

```rust
impl From<Value> for lexpr::Value {
    /// Total: every Value is representable as a lexpr::Value.
    fn from(v: Value) -> lexpr::Value;
}

impl TryFrom<lexpr::Value> for Value {
    type Error = LexprConversionError;
    /// Lossy: rationals, complex, and out-of-range bignums error rather than truncate.
    fn try_from(v: lexpr::Value) -> Result<Value, LexprConversionError>;
}

pub enum LexprConversionError {
    UnsupportedRational { num: i64, den: i64 },
    UnsupportedComplex,
    BignumOutOfRange,
}
```

The `Value -> lexpr::Value` direction is total because `Value`'s numeric tower is a strict subset of `lexpr::Value`'s. The reverse direction errors on:

- Rationals (e.g., `1/2` in the lexpr value).
- Complex numbers.
- Bignums whose value falls outside `i64::MIN..=i64::MAX`.

These are surfaced as errors rather than silently truncated — silent truncation would break invariants for downstream code that round-trips through both types.

`From` (infallible) is **not** implemented for `lexpr::Value -> Value`; consumers use `TryFrom` so the error case is explicit at every call site. The `lexpr::Value -> Value` conversion deliberately requires an explicit `try_from(...)` call to make the lossy direction obvious in code review.

The crate does **not** re-export `lexpr::Value` as `mcp_tools::lexpr::Value`. Consumers that need lexpr during the migration window declare it in their own `Cargo.toml` (typically `lexpr = "0.2"`). Rationale: forwarding a third-party crate's API makes the public surface fragile against `lexpr` version bumps, and consumers are migrating away anyway. Decision recorded as `q-reexport` in the change spec decisions log.

## Keyword Argument Extraction
<!-- spec-id: mcp-tools/parser/kw-value-extraction -->

`get_kw_value(&Value, &str) -> Result<Option<Value>>` looks up a keyword argument in a
tool-call form and returns the raw `Value` occupying its value slot.

```rust
pub fn get_kw_value(root: &Value, key: &str) -> Result<Option<Value>>;
```

The form is read as a head followed by keyword/value pairs:

```
(head :k1 v1 :k2 v2 ...)
```

Scanning rules:

- `root` must be a `Value::List`. Any other variant — including `Pair` — is an error
  (`expected list (tool call form)`).
- Index 0 is the head and is skipped unconditionally; it is never treated as a keyword.
- From index 1 the scan alternates key slot / value slot. A key slot must hold
  `Value::Keyword`; the first key slot holding anything else ends the scan and yields
  `Ok(None)`.
- **The value slot is consumed positionally, whatever its type.** After a key slot is
  matched, the immediately following item is taken as that key's value with no type
  inspection. A keyword-valued argument such as `(record :verdict :pass)` therefore
  yields `verdict -> Keyword("pass")`; the `:pass` in value position is never
  re-interpreted as the next key. Scanning resumes at the item after the value slot,
  so later keys in the same form still resolve. The same holds for a bare word in
  value position: `(record :verdict pass)` yields `verdict -> Symbol("pass")`, and
  `Symbol` is returned as-is rather than being coerced to a string.
- A key slot with no following item is an error
  (`expected value after keyword :<name>`); the form is malformed, not simply missing
  the requested key.
- A well-formed scan that never matches `key` yields `Ok(None)`.

The returned `Value` is cloned; `root` is left untouched.

This positional rule is contractual: consumers such as `mcp-compose` encode enumerated
values as keywords (`:pass` / `:fail`, agent levels, record keys) and rely on a
keyword in value position parsing as a value rather than shifting the key/value
alignment for the rest of the form. The bare-word spelling is relied on for the same
enumerations — a `symbol`-typed field reported as `(record :verdict pass)` — so
`Keyword` and `Symbol` in value position are both load-bearing and neither is
normalized into the other.

`get_kw_str` and `require_kw_str` are thin wrappers over `get_kw_value` and inherit
these scanning rules; they add only the string type-check on the extracted value.

## API Deprecation
<!-- spec-id: mcp-tools/parser/api-deprecation -->

Every existing function whose signature mentions `lexpr::Value` gets a `Value`-based counterpart in 0.3, and the `lexpr::Value` version is marked `#[deprecated]`. The deprecation note points at the replacement.

### Rename of the old parser

```rust
#[deprecated(note = "use parse_value -> Value; lexpr::Value is removed in 1.0")]
pub fn parse_value_lexpr(input: &str) -> Result<lexpr::Value>;
```

This is the previous body of `parse_value` exactly, renamed.

### Counterpart pairs

| Lexpr-based (deprecated in 0.3) | Value-based (new in 0.3) |
|---|---|
| `get_kw_value(&lexpr::Value, &str) -> Result<Option<lexpr::Value>>` | `get_kw_value(&Value, &str) -> Result<Option<Value>>` |
| `get_kw_str(&lexpr::Value, &str) -> Result<Option<String>>` | `get_kw_str(&Value, &str) -> Result<Option<String>>` |
| `require_kw_str(&lexpr::Value, &str) -> Result<String>` | `require_kw_str(&Value, &str) -> Result<String>` |
| `iter_list(&lexpr::Value) -> Result<impl Iterator<...>>` | `iter_list(&Value) -> Result<impl Iterator<Item = Value>>` |
| `parse_str_list(&lexpr::Value) -> Result<Vec<String>>` | `parse_str_list(&Value) -> Result<Vec<String>>` |
| `parse_text_ref(&lexpr::Value) -> Result<TextRef>` | `parse_text_ref(&Value) -> Result<TextRef>` |
| `render_text_ref(&TextRef) -> String` | (unchanged — does not take `lexpr::Value`) |

The signatures with the same name and a different argument type are **type-overloaded across two distinct functions**; Rust does not have function overloading. The implementation chosen is:

- The old (`lexpr::Value`-taking) functions are renamed with a `_lexpr` suffix and marked `#[deprecated]`. Example: `get_kw_value_lexpr`, `iter_list_lexpr`.
- The new (`Value`-taking) functions take the unsuffixed name. Example: `get_kw_value`, `iter_list`.

This means any caller passing a `&lexpr::Value` to `get_kw_value` gets a compile error in 0.3 directing them either to `get_kw_value_lexpr` (deprecated) or to convert via `Value::try_from(lexpr_value)` and call the new function. The breaking-change behavior is identical to the `parse_value` rename above.

### Deprecation note format

Each deprecated function carries:

```rust
#[deprecated(note = "use the Value-based <name>; lexpr::Value is removed in 1.0")]
```

The note is uniform so consumers can grep for `lexpr::Value is removed in 1.0` to find every use site.

### Internal callers

`src/extract/args.rs` and `src/format/response.rs` contain internal callers of the deprecated functions. In 0.3, these callers continue using the `_lexpr`-suffixed forms (with `#[allow(deprecated)]`) so the workspace compiles cleanly under `-D warnings`. They are migrated to the `Value`-based API as part of 0.4 work, not 0.3.

### TextRef

`TextRef` itself is unchanged — it is a `mcp-tools` type, not a `lexpr` type. Only the `parse_text_ref` signature changes.

## Convenience Re-exports
<!-- spec-id: mcp-tools/parser/api-reexports non-testable -->

`mcp_tools::Value`, `mcp_tools::Spanned`, `mcp_tools::Position`, `mcp_tools::Span`, `mcp_tools::Comment`, `mcp_tools::ParseError`, and `mcp_tools::LexprConversionError` are re-exported at the crate root from the `parser` module. Consumers of the new API never reach into `mcp_tools::parser::types` directly.

## Stability Promise
<!-- spec-id: mcp-tools/parser/api-stability non-testable -->

The 0.3 surface is the stable public API for the new parser. The only further breaking changes between 0.3 and 1.0 are:

1. Deletion of every `_lexpr`-suffixed function.
2. Deletion of `From<Value> for lexpr::Value`, `TryFrom<lexpr::Value> for Value`, and `LexprConversionError`.
3. Removal of `lexpr` from `Cargo.toml`.

No new method names are added to `Value` between 0.3 and 1.0 except as additive (non-breaking) extensions. The numeric tower is frozen at `Integer(i64) | Float(f64)` for the entire deprecation window.
