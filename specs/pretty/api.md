# Structural Pretty-Printer

This document specifies the structural pretty-printer for `Value`. The printer produces deterministic, line-wrapped output suitable for committed config files and diff-friendly machine output. It is gated behind the `format-pretty` Cargo feature.

The printer operates on the lightweight `Value` representation; the spanned form (with comment retention on output) is out of scope for 0.3 and is tracked as a separate change spec.

## Options
<!-- spec-id: mcp-tools/pretty/options -->

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrettyOpts {
    /// Maximum target line width before a list is wrapped onto multiple lines.
    /// Default: 80. A value of 0 forces every list to wrap.
    pub max_line_width: usize,

    /// Number of spaces per indent level. Default: 2.
    pub indent: usize,

    /// When wrapping a list whose tail consists of `:keyword value` pairs,
    /// align the values vertically. Default: true.
    pub align_keyword_args: bool,

    /// When the input is a top-level `List` whose elements are themselves
    /// lists ("multiple top-level forms"), separate them with a blank line.
    /// Default: true.
    pub blank_line_between_top_forms: bool,
}

impl Default for PrettyOpts { /* ... */ }
```

`PrettyOpts` is a public struct with public fields so consumers can construct it via record-update syntax: `PrettyOpts { max_line_width: 100, ..Default::default() }`. Defaults match the values documented above.

`max_line_width = 0` is a deliberate corner case meaning "force wrap"; it never panics. Negative widths are not representable (`usize`).

## pretty_print
<!-- spec-id: mcp-tools/pretty/pretty-print -->

```rust
pub fn pretty_print(value: &Value, opts: &PrettyOpts) -> String;
pub fn pretty_print_default(value: &Value) -> String;
```

`pretty_print_default(v)` is exactly `pretty_print(v, &PrettyOpts::default())`.

The output is a `String` containing only ASCII whitespace (spaces and `\n`) for layout — no tabs, no CRLF. The output never contains a trailing newline; consumers writing to a file are expected to append one if their format requires it.

The output is always parseable by `parse_value`. For inputs with no `Float(NaN)` or `Float(±inf)`, `parse_value(pretty_print(&v, opts)).unwrap()` is structurally equal to `v` modulo numeric round-tripping. Integers, booleans, nil, strings, symbols, keywords, and proper lists round-trip exactly. (See `mcp-tools/pretty/round-trip` below.)

## Layout Rules
<!-- spec-id: mcp-tools/pretty/layout-rules -->

The layout algorithm is a single-pass top-down decision tree:

1. **Atoms** (`Nil`, `Bool`, `Integer`, `Float`, `String`, `Symbol`, `Keyword`) emit exactly what `Value::Display` would emit. Strings use `quote_str` for escaping.
2. **Pair** `(a . b)` emits `(<a> . <b>)` on one line if it fits within `max_line_width`; otherwise wraps with the dot on a new line at `indent` spaces past the paren.
3. **List** `(x1 x2 ... xn)`:
   - If the rendered single-line form fits within `max_line_width` (counting from the current column), emit on one line.
   - Otherwise wrap: emit `(`, then the head and the first argument on the same line if both are atoms, then each remaining element on its own line indented `indent` spaces past the open paren of the parent list, then `)` on the line of the last element.
4. **Empty list** `()` emits `()` regardless of width.

The "current column" used for fit calculations is the column at which the open paren would be emitted. Indentation accumulates additively as the printer descends into nested lists.

Integers and floats use `Display`. Strings use `quote_str`. The escape policy is the existing one in `developer-guide.md` (escape `\\`, `\"`, `\n`, `\r`, `\t`).

### Keyword Argument Alignment
<!-- spec-id: mcp-tools/pretty/keyword-alignment -->

When `align_keyword_args` is `true` and a list of length ≥ 2 has the shape `(head pos1 ... posK :k1 v1 :k2 v2 ...)` and the printer has decided to wrap, the keyword/value pairs are emitted with values aligned vertically:

```
(define-tool foo
  :pred (use "p")
  :body (use "b")
  :max  3)
```

The alignment column is the position of the longest keyword (including the leading `:`) plus one space. The keyword is left-padded with spaces if needed; the value follows. If `align_keyword_args` is `false`, every keyword/value pair is emitted on its own line with a single space between them and no padding.

A list with positional and keyword args mixed on the same wrapped block keeps positionals on their own lines (without alignment) before the keyword block begins.

### Blank Line Between Top Forms
<!-- spec-id: mcp-tools/pretty/blank-line-between-top-forms -->

`pretty_print` operates on a single `Value`. The "blank line between top forms" rule applies only when the caller intends to print multiple forms and uses the convenience function:

```rust
pub fn pretty_print_top_forms(values: &[Value], opts: &PrettyOpts) -> String;
```

This function pretty-prints each value with `opts` and joins them. If `blank_line_between_top_forms` is `true` (the default), consecutive top-level forms are separated by exactly one blank line (`\n\n`). If `false`, they are separated by a single newline (`\n`). The output never starts or ends with a newline.

`pretty_print_top_forms(&[], _)` returns the empty string.

## Determinism
<!-- spec-id: mcp-tools/pretty/determinism -->

Given the same `(value, opts)` pair, the printer produces byte-identical output across runs, threads, processes, and platforms. There is no use of `HashMap` or other randomized iteration order in the layout decisions; lists are walked left-to-right in their stored order.

This is testable by running the printer twice on the same input and comparing the byte buffers; the property test in `tests/pretty.rs` does this for a corpus of inputs.

## Round-Trip
<!-- spec-id: mcp-tools/pretty/round-trip -->

For every `Value` that does not contain `Float(NaN)` or `Float(±inf)` in any subtree, the following holds:

```rust
let printed = pretty_print(&value, opts);
let reparsed = parse_value(&printed).unwrap();
assert_eq!(value, reparsed);
```

`Float(NaN)` is excluded because `NaN != NaN` by IEEE-754. `Float(±inf)` is excluded because the tokens `+inf.0` / `-inf.0` are emitted by `Value::Display` but parser support is not guaranteed at the time of writing; the round-trip test skips these cases. (If parser support is later added, the carve-out can be lifted.)

Integers, booleans, nil, strings, symbols, keywords, proper lists, and dotted pairs round-trip exactly. Floats round-trip when their `Display` form is parseable as a float.

## Feature Gate
<!-- spec-id: mcp-tools/pretty/feature-gate -->

The pretty-printer is behind a new Cargo feature `format-pretty`. It is disabled by default. Enabling it adds the `pretty` module at the crate root:

```toml
[dependencies]
mcp-tools = { version = "0.3", features = ["format-pretty"] }
```

```rust
use mcp_tools::pretty::{pretty_print, pretty_print_default, pretty_print_top_forms, PrettyOpts};
```

`format-pretty` does not depend on or imply the existing `format` feature. They are independent and may be enabled in any combination.

## Design Rationale
<!-- spec-id: mcp-tools/pretty/design-rationale non-testable -->

**Why a hand-rolled algorithm rather than Wadler-style group/break?**
The use cases (config files, codegen output) do not need the optimality guarantees of group/break. A simple "fits on one line, otherwise break each child onto its own line" rule produces output that is easy to predict and review, and easier to implement correctly. If a future consumer needs sophisticated layout (e.g., `let`-binding alignment), it can be added on top.

**Why operate on `Value` rather than `Spanned`?**
The 95% case (config files written from machine-built data structures, not parsed from source) does not have comments to preserve. Adding `Spanned` support is a separate change spec; doing it together would conflate two unrelated design decisions and double the test surface.

**Why `align_keyword_args = true` by default?**
Without alignment, wrapped keyword blocks look ragged and are harder to scan visually. Alignment is cheap (one extra pass to compute the longest keyword). The opt-out exists for tools that want strictly mechanical formatting.

**Why no `\t` for indentation?**
Tabs render inconsistently across editors and break alignment. Pretty-printing is the wrong place to express tab-stop preferences.
