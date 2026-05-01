# Parser Value Types

This document specifies the value types produced by the position-tracking parser: the lightweight `Value` type used for construction and formatting, and the `Spanned` family used when source positions and comments matter.

## Value Enum
<!-- spec-id: mcp-tools/parser/value-type -->

```rust
pub enum Value {
    Nil,
    Bool(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Symbol(String),
    Keyword(String),
    List(Vec<Value>),
    Pair(Box<(Value, Value)>),
}
```

`Value` is the canonical representation for all programmatic construction, manipulation, and formatting. It carries no source-position metadata and no comment retention — the common case pays no overhead for either.

`Value` provides the following accessor surface (mirroring the `lexpr::Value` API the new parser replaces):

```rust
impl Value {
    pub fn is_nil(&self) -> bool;
    pub fn is_bool(&self) -> bool;
    pub fn is_integer(&self) -> bool;
    pub fn is_float(&self) -> bool;
    pub fn is_string(&self) -> bool;
    pub fn is_symbol(&self) -> bool;
    pub fn is_keyword(&self) -> bool;
    pub fn is_list(&self) -> bool;
    pub fn is_pair(&self) -> bool;

    pub fn as_bool(&self) -> Option<bool>;
    pub fn as_i64(&self) -> Option<i64>;
    pub fn as_f64(&self) -> Option<f64>;
    pub fn as_str(&self) -> Option<&str>;
    pub fn as_symbol(&self) -> Option<&str>;
    pub fn as_keyword(&self) -> Option<&str>;
    pub fn as_list(&self) -> Option<&[Value]>;
    pub fn as_pair(&self) -> Option<(&Value, &Value)>;
}
```

Equality (`PartialEq`/`Eq`) is structural across the enum. `Value` is `Clone` and implements `Debug`. It is not `Copy`. It implements `Hash` only for variants without `f64` (consumers that need to hash floats canonicalize via bit-pattern themselves).

## Spanned Type
<!-- spec-id: mcp-tools/parser/spanned-type -->

```rust
pub struct Spanned {
    pub value: SpannedNode,
    pub span: Span,
    pub leading_comments: Vec<Comment>,
    pub trailing_comments: Vec<Comment>,
}

pub enum SpannedNode {
    Nil,
    Bool(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Symbol(String),
    Keyword(String),
    List(Vec<Spanned>),
    Pair(Box<(Spanned, Spanned)>),
}
```

`SpannedNode` mirrors `Value` shape exactly except that `List` and `Pair` recurse into `Spanned` rather than `Value`. This means every node in a parsed tree carries a span and its adjacent comments — diagnostics can quote any sub-expression with full source coordinates.

`Spanned::into_value(self) -> Value` strips spans and comments, producing the lightweight representation. The reverse direction does not exist — synthesizing positions for a programmatically built tree is meaningless. Code paths that need spans must obtain them by parsing source text.

`Spanned` is `Clone` and `Debug`. It is not `PartialEq` against `Value` directly; consumers comparing structurally call `into_value()` first.

## Numeric Tower
<!-- spec-id: mcp-tools/parser/numeric-tower -->

The parser stores only two numeric variants: `Integer(i64)` and `Float(f64)`. Rationals, complex numbers, and arbitrary-precision integers (bignums) are not represented natively.

Parse-time policy:

- An integer literal that fits in `i64` parses to `Integer(i64)`.
- An integer literal that exceeds `i64` range produces a parse error (`OutOfRange`); it does **not** silently widen to `Float` and does **not** truncate.
- A floating literal parses to `Float(f64)`. Literals outside `f64` range produce `Float(±INFINITY)` per IEEE-754 conversion (no error).
- No syntactic form for rationals (`1/2`) or complex (`1+2i`) is recognized.

Rationale: among current consumers (`mcp-compose`, `mcp-planner`, downstream MCP servers), every numeric value observed in production fits comfortably in `i64` or `f64`. Halving the variant count cuts pattern-match noise everywhere. Consumers that genuinely need rationals encode them as `(rational num denom)` forms — explicit and queryable rather than hidden inside a numeric variant.

This is a deliberate, breaking trade-off relative to `lexpr::Value`. See `specs/migration/lexpr-deprecation.md` for the migration story.

## Keyword Canonicalization
<!-- spec-id: mcp-tools/parser/keyword-canonicalization -->

A source token of the form `:foo` always parses to `Keyword("foo")`. The leading colon is **not** stored in the keyword name. Constructors and accessors operate on the colon-less form:

```rust
let kw = Value::Keyword("foo".to_string());
assert_eq!(kw.as_keyword(), Some("foo"));   // not ":foo"
```

This eliminates the dual-form behavior `lexpr` exhibits, where some code paths produce `Keyword("foo")` and others produce `Symbol(":foo")`. The `normalize_kw` helper present in earlier `mcp-tools` releases is removed in 0.3.

When formatting a `Keyword` back to source via `Display` or `quote_str`, the leading colon is reattached. Round-trip property: `parse_value(format!("{}", v)) == Ok(v.clone())` for every `Value` reachable from parsing source.

## List Representation
<!-- spec-id: mcp-tools/parser/list-representation -->

Proper lists are stored as `Value::List(Vec<Value>)`. Indexing, slicing, iteration, and `len()` are O(1) or O(n) as expected for `Vec`; cons-cell traversal is not part of the API.

`Value::Pair(Box<(Value, Value)>)` is reserved exclusively for genuine dotted pairs `(a . b)` where `b` is not itself a list. A mixed form `(a b . c)` parses to `List([a, b, ?])` where the tail position carries `c`; specifically, the parser produces `List([a, b, Pair(box (sentinel, c))])` is **not** the chosen representation. The chosen representation is:

- `(a b c)` → `List([a, b, c])`
- `(a . b)` → `Pair(box (a, b))` where `b` is any non-list `Value`
- `(a b . c)` → `List([a, b, c])` when `c` parses as a proper list (the dot is redundant); otherwise `Pair(box (a, Pair(box (b, c))))` for genuine improper tails.

In practice, MCP consumer payloads almost never use improper tails beyond the simple `(key . value)` form, so the common case is `List(Vec<Value>)` end-to-end.

The `iter_list` and `as_list` helpers on `Value` return `Some` only for `List`, never for `Pair`. Code that wants to walk improper lists handles `Pair` explicitly.

## Design Rationale
<!-- spec-id: mcp-tools/parser/design-rationale non-testable -->

Two value types instead of `Value` with optional spans:

- The 95% case (formatting, builders, programmatic construction, equality) does not benefit from carrying span machinery on every node.
- `Option<Span>` everywhere is the worst of both worlds: cost in cache-line size on every node, plus a `match` at every diagnostic site.
- `Spanned::into_value()` is the explicit boundary; it is cheap and obvious.

Vec-backed proper lists instead of cons cells:

- Iteration is the dominant operation in consumer code; cons-cell traversal makes that O(n) per access.
- `Vec` is the data structure Rust already optimizes; reusing it costs nothing and gives consumers a familiar surface.
- The `Pair` variant remains for the rare improper case rather than forcing every list to pay cons-cell overhead.
