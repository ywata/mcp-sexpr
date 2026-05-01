# AST Builder Functions

This document specifies a small set of constructor functions that produce `Value` instances. The functions are part of the default feature set and have no extra dependencies. They reduce the boilerplate of programmatic `Value` construction (codegen, desugaring, error suggestions).

A complementary `sexpr!` quasiquotation proc-macro is **out of scope** for this change spec; it requires a separate proc-macro crate and will be addressed in a future change.

## cons
<!-- spec-id: mcp-tools/build/cons -->

```rust
pub fn cons(car: Value, cdr: Value) -> Value;
```

Constructs a dotted pair `(car . cdr)`. Always returns `Value::Pair(Box::new((car, cdr)))`. The function is deliberately literal — it does **not** detect that the result represents a proper list and re-shape it to `Value::List`. Callers wanting proper lists call `list(...)` directly.

A consumer who needs traditional cons-list semantics (where `cons(a, cons(b, Nil))` represents `(a b)`) is expected to handle the conversion themselves. Mixing `Pair` and `List` representations is rejected by `parse_value`-comparable equality, so this caveat matters.

## list
<!-- spec-id: mcp-tools/build/list -->

```rust
pub fn list(items: Vec<Value>) -> Value;
```

Constructs a proper list `(item1 item2 ... itemN)` from the supplied vector. `list(vec![])` returns `Value::Nil` (matching the convention that the empty list and `nil` are the same value).

`list` is intentionally not `impl<I: IntoIterator<Item = Value>>` to keep monomorphization cost predictable and the API surface minimal. A future iterator-taking variant (`list_from_iter`) can be added if a consumer demonstrates need.

## keyword
<!-- spec-id: mcp-tools/build/keyword -->

```rust
pub fn keyword(name: &str) -> Value;
```

Constructs `Value::Keyword(name.to_string())`. The `name` argument is **without** the leading colon; passing `keyword(":foo")` produces `Value::Keyword(":foo".to_string())`, which renders as `::foo` and is wrong.

The function does not validate that `name` is a syntactically valid keyword (no whitespace, no `(`, no `)`, etc.). Callers passing user-controlled strings should validate first.

## symbol
<!-- spec-id: mcp-tools/build/symbol -->

```rust
pub fn symbol(name: &str) -> Value;
```

Constructs `Value::Symbol(name.to_string())`. The function does not validate that `name` is a syntactically valid symbol. Empty strings, strings containing whitespace, and strings starting with a digit will produce `Value`s that do not round-trip through `parse_value`.

## string
<!-- spec-id: mcp-tools/build/string -->

```rust
pub fn string(s: &str) -> Value;
```

Constructs `Value::String(s.to_string())`. The string content is stored verbatim; escaping is applied at render time by `quote_str` / `Value::Display` / `pretty_print`, not at construction.

## integer
<!-- spec-id: mcp-tools/build/integer -->

```rust
pub fn integer(n: i64) -> Value;
```

Constructs `Value::Integer(n)`. There is no `unsigned` variant; consumers convert via `i64::try_from(u)?` at the call site so the overflow case is explicit.

There is no `float` builder in 0.3. `Value::Float(x)` is constructed directly when needed; deferring this avoids a question about NaN handling that has no obvious answer (silently allow? reject? clamp?).

## Design Rationale
<!-- spec-id: mcp-tools/build/design-rationale non-testable -->

**Why standalone functions, not `Value::list(...)` methods?**
The feature request specifies free functions, and the call sites read better: `list(vec![symbol("define"), symbol("x"), integer(42)])` is clearly a constructor expression, while `Value::list(vec![Value::symbol("define"), ...])` is repetitive. Both styles are equally idiomatic in Rust; the request's preference is honored.

**Why no `nil()`, `boolean(b)`, `float(x)`?**
These don't earn their keep — `Value::Nil` and `Value::Bool(b)` are already short. Adding constructors for them would dilute the set without saving keystrokes. `float` is omitted to defer the NaN/inf question.

**Why no validation in `keyword`/`symbol`?**
The `Value` type does not enforce syntactic constraints on symbol or keyword names — `Value::Symbol(" ".into())` is constructible today and represents an unparseable form. Adding validation in the builders would be inconsistent with the type's permissiveness. Callers who need validation can do it themselves; a future `valid_symbol_name(&str) -> bool` helper in the parser module is a possible follow-up.

**Why is `cons` not smart about list flattening?**
A consumer writing `cons(a, cons(b, Nil))` and expecting `Value::List(vec![a, b])` would be surprised when `parse_value("(a b)")` produces `Value::List(vec![a, b])` and equality fails because the two are differently-shaped. Making `cons` smart hides the distinction in one direction but not the other; it is cleaner to keep `cons` literal and let consumers use `list` for proper lists.
