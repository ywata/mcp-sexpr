# Source Positions and Comment Retention

This document specifies how the position-tracking parser records source locations and retains comments adjacent to parsed nodes. These are the data structures consumed by diagnostic emitters and the future pretty-printer.

## Position and Span
<!-- spec-id: mcp-tools/parser/spans -->

```rust
pub struct Position {
    pub line: u32,
    pub column: u32,
    pub byte_offset: u32,
}

pub struct Span {
    pub start: Position,
    pub end: Position,
}
```

### Indexing convention

`line` and `column` are **1-indexed** for human display. The first character of the source is at `line: 1, column: 1, byte_offset: 0`. This matches the convention used by virtually every error reporter humans read directly (Rust compiler, GCC, Clang, Python tracebacks).

For LSP consumers, which expect 0-indexed positions, the parser provides:

```rust
impl Position {
    /// Returns (line, column) 0-indexed, suitable for LSP `Position { line, character }`.
    pub fn lsp(&self) -> (u32, u32) {
        (self.line.saturating_sub(1), self.column.saturating_sub(1))
    }
}
```

The 1-indexed values are the canonical fields stored. The `lsp()` accessor is the one-way bridge — there is no constructor that takes 0-indexed input. Sources may not have a `line: 0` or `column: 0`; the parser guarantees `line >= 1` and `column >= 1` for every position it emits.

### Column counting

A column is a count of **Unicode scalar values** (Rust `char`) since the start of the current line, not bytes and not grapheme clusters. The first character on a line is `column: 1`, the second is `column: 2`, and so on, regardless of the byte width of preceding characters.

Tab characters (U+0009) advance the column by exactly 1; the parser does not interpret tab stops.

### Line counting

A line break is any of LF (`\n`), CR (`\r`), or CRLF (`\r\n`). All three increment `line` by 1 and reset the next character's column to 1. CRLF counts as a single line break, not two.

### byte_offset

`byte_offset` is a 0-indexed byte offset into the original input string. It is sufficient to slice the original source: `&input[start.byte_offset as usize .. end.byte_offset as usize]` yields the source text covered by the span.

`byte_offset` advances by the UTF-8 byte width of each character — 1 for ASCII, up to 4 for higher code points. Combined with `line`/`column`, this lets consumers convert between byte slicing (for source extraction) and human display (for diagnostics) without re-walking the source.

### Span coverage

For atom values, `span` covers the literal token: opening quote of a string through closing quote, first to last digit of a number, etc.

For lists `(a b c)`, `span.start` is the position of the open paren `(` and `span.end` is the position **after** the close paren `)`. The end is exclusive: a span `[start..end]` slices to the inclusive token range. Sub-elements `a`, `b`, `c` carry their own spans nested inside the parent's.

For dotted pairs `(a . b)`, the parent `Pair` span runs from the open paren through the close paren. The children carry spans of their respective sub-expressions; the `.` token does not appear as a span.

For quoted forms — e.g., `'expr` — the parser desugars to `(quote expr)` and synthesizes a `List` span that covers the entire source range from the quote character through the end of `expr`. The synthesized `Symbol("quote")` head carries a zero-width span at the quote character (so consumers walking the synthesized list still see a position but never report the synthetic symbol as covering source).

### Width limit

All position fields are `u32`. Sources larger than 4 GiB are not supported; the parser returns `ParseError::SourceTooLarge` if a `byte_offset` would overflow `u32::MAX`. This is well above any plausible MCP S-expression payload (4 GiB of S-expressions is tens of millions of nodes).

## Comment Retention
<!-- spec-id: mcp-tools/parser/comment-retention -->

```rust
pub struct Comment {
    pub kind: CommentKind,
    pub text: String,
    pub span: Span,
}

pub enum CommentKind {
    Line,   // ; ...
    Block,  // #| ... |#
}
```

Each `Spanned` node carries two comment vectors:

- `leading_comments`: comments preceding the node, in source order.
- `trailing_comments`: same-line comments following the node, in source order.

### Attachment rules

A comment attaches to a node according to the following rules, evaluated in order:

1. **Trailing**: A comment on the same source line as a value, occurring after the value's last token, attaches as a trailing comment of that value. "Same line" means no LF/CR/CRLF appears between the value's end and the comment's start.
2. **Leading**: A comment that does not match rule 1, occurring before a value with only whitespace and other comments between them, attaches as a leading comment of that value.
3. **Trailing-of-parent**: A comment after the closing token of a list `)`, on the same source line as the close paren, attaches as a trailing comment of the parent list — not the last child.
4. **Floating**: A comment that has no value following it before EOF (i.e., a trailing comment at the end of the file with no value to attach to) is dropped. Top-level trailing-of-file comments are not preserved.

Comments inside a list (between siblings) attach to the immediately following sibling as leading comments. The exception is rule 3: a comment between the last child and the close paren attaches as a trailing comment of the **last child**, not of the parent.

### Round-trip property

For sources that contain no trailing-of-file comments and no exotic whitespace patterns, the future pretty-printer is required to reproduce every comment in its original attachment position. The data captured here is sufficient for that round-trip; the pretty-printer is not in scope for this change.

### parse_value behavior

`parse_value` (returning `Value`) discards comments entirely — `Value` has no field to hold them. Consumers who need comments use `parse_value_with_positions` and walk the `Spanned` tree.

## Limits and Edge Cases
<!-- spec-id: mcp-tools/parser/position-limits non-testable -->

- **BOM**: A leading UTF-8 BOM (`\u{FEFF}`) at byte offset 0 is silently consumed and contributes 0 to column counting. A BOM elsewhere in the source is treated as a regular character (in practice, only valid inside a string literal — outside, it produces a parse error).
- **Mixed line endings**: A source with mixed LF / CRLF / lone CR line endings is parsed correctly per the rules above. The parser does not enforce uniform line endings.
- **Form feed / vertical tab**: U+000C and U+000B are treated as whitespace but do not advance line counts. They are rare in MCP payloads; consumers needing line counts in such sources should preprocess.
