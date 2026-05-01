# Parser Grammar

This document specifies the surface grammar accepted by the position-tracking S-expression parser. The grammar is a subset of R7RS-flavored S-expressions sized to match the inputs current `mcp-tools` consumers send through `lexpr`.

## Accepted Forms
<!-- spec-id: mcp-tools/parser/grammar -->

The parser accepts the following productions:

```
value      := atom | list | quoted
atom       := boolean | nil | integer | float | string | symbol | keyword
list       := "(" value* ")"                    ; proper list
            | "(" value+ "." value ")"          ; dotted pair (improper list)
quoted     := "'" value                          ; (quote value)
            | "`" value                          ; (quasiquote value)
            | "," value                          ; (unquote value)
            | ",@" value                         ; (unquote-splicing value)
boolean    := "#t" | "#f"
nil        := "nil" | "()"
integer    := /-?[0-9]+/                         ; base 10, fits in i64
float      := /-?[0-9]+\.[0-9]+([eE][+-]?[0-9]+)?/
            | /-?[0-9]+[eE][+-]?[0-9]+/
string     := "\"" string-char* "\""             ; see "String Escapes" below
symbol     := identifier-start identifier-rest*
keyword    := ":" symbol-name                    ; canonicalized to Keyword(name)
```

Whitespace (space, tab, LF, CR) and comments separate tokens but are not themselves productions. Quote forms desugar at read time:

| Source | Desugars to |
|---|---|
| `'expr` | `(quote expr)` |
| `` `expr `` | `(quasiquote expr)` |
| `,expr` | `(unquote expr)` |
| `,@expr` | `(unquote-splicing expr)` |

`nil` and `()` parse to `Value::Nil`. The dotted form `(a . b)` parses to `Value::Pair(Box::new((a, b)))`. Proper lists `(a b c)` parse to `Value::List(vec![a, b, c])`. Mixed forms `(a b . c)` parse to a list whose tail is a `Pair`.

## String Escapes
<!-- spec-id: mcp-tools/parser/string-escapes -->

Inside string literals, the following backslash escapes are recognized:

| Escape | Produces |
|---|---|
| `\\` | U+005C `\` |
| `\"` | U+0022 `"` |
| `\n` | U+000A LF |
| `\r` | U+000D CR |
| `\t` | U+0009 TAB |

Any other backslash sequence (e.g., `\x41`, `\0`, `\a`) is a parse error. This matches the existing escape policy in `docs/developer-guide.md` and ensures every output produced by `quote_str` round-trips through this parser without loss.

Unescaped control characters (e.g., a literal LF inside a string literal) are accepted verbatim and stored as-is in the resulting `String` value. The parser does not normalize line endings inside strings.

## Comments
<!-- spec-id: mcp-tools/parser/comments -->

Two comment forms are recognized:

- **Line comments** start with `;` and run to the next LF or end of input.
- **Block comments** are delimited by `#|` and `|#` and may nest. A nested `#|` opens a new level; `|#` closes the innermost open level.

Comments are tokens at the lexer level; they never produce `Value` nodes. Their treatment depends on the parser entry point:

- `parse_value` (returns `Value`) discards comments entirely.
- `parse_value_with_positions` (returns `Spanned`) attaches comments to adjacent nodes as `leading_comments` / `trailing_comments`. See `specs/parser/source-positions.md`.

A line comment ending immediately before a newline that precedes a value attaches as a leading comment of that value. A line comment on the same source line as a value, occurring after the value's closing token, attaches as a trailing comment of that value.

## Grammar Design Notes
<!-- spec-id: mcp-tools/parser/grammar-rationale non-testable -->

The grammar is intentionally tighter than full R7RS:

- No `#;` datum comments — no consumer uses them.
- No character literals (`#\a`) — strings cover every consumer use case.
- No vector literals (`#(...)`) — never appeared in consumer corpora.
- No bytevectors, no labeled references — out of scope for MCP S-expression payloads.

If a future consumer needs one of these forms, it can be added as a non-breaking extension since none of the current productions overlap.

## Alternatives Considered
<!-- spec-id: mcp-tools/parser/alternatives-considered non-testable -->

Parser implementation strategies evaluated and rejected:

| Alternative | Rejected because |
|---|---|
| Fork `lexpr` to add spans | Maintenance burden of an external fork; pace of upstream uncertain; we would still own the divergent surface forever. |
| Side-table parser keyed by structural path, with `lexpr` remaining canonical | Two parsers to keep grammar-compatible; structural paths break under tree transforms; the shadow-parser pattern is fragile on a Lisp-reader grammar with non-trivial edge cases. |
| Parser combinator (`chumsky`, `winnow`, `nom`) | Substantial compile-time dependency for a grammar simple enough to recursive-descent in a few hundred LOC; combinator error reporting is hard to tune for the targeted error messages. |
| Single `Value` type with `Option<Span>` on every node | Pollutes the common case (programmatic construction, formatting) with span machinery for the 5% that needs it; `match` discipline at every diagnostic site. |
| Keep `lexpr::Value` in API forever, add positions via newtype wrapper | Cannot attach spans recursively without owning the value type; future features (comments, custom keyword semantics) blocked behind upstream decisions. |

Hand-rolled with two value types is the chosen point in the design space because it minimizes long-term dependency footprint and keeps both the common-case API and the diagnostic-case API ergonomic for their respective use cases.
