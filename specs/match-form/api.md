# Form-Shape Pattern Matching

This document specifies the form-shape pattern matching helpers (`FormMatch`, `match_form`) used to lower S-expressions of shape `(head <positional...> :k1 v1 :k2 v2 ...)` to typed values. The helpers operate on the new `Value` type and are part of the default feature set.

The current keyword extraction helpers (`get_kw_value`, `require_kw_str`) cover individual lookups. The matcher centralizes the surrounding "validate the form shape" logic so consumers don't repeat it at every call site.

## match_form
<!-- spec-id: mcp-tools/match-form/match-form -->

```rust
pub fn match_form<'a>(
    value: &'a Value,
    expected_head: &str,
) -> Result<FormMatch<'a>>;
```

Matches `value` against the shape `(<expected_head> <positional...> :k1 v1 :k2 v2 ...)` and returns a `FormMatch` over borrowed sub-values.

`match_form` succeeds when:
1. `value` is a `Value::List` with at least one element.
2. The first element is a `Value::Symbol` whose name equals `expected_head`.
3. The remaining elements are partitioned into a positional prefix (zero or more non-`Keyword` values) followed by zero or more `(Keyword, Value)` pairs. Once a `Keyword` is seen, all subsequent positions must alternate `Keyword`, value, `Keyword`, value …; a non-`Keyword` after the first `Keyword` is an error.
4. Each `Keyword` is followed by exactly one value. A trailing `Keyword` with no value is an error.

A `Value::Pair` is **not** a list and never matches. A `Value::List` whose first element is not a `Value::Symbol`, or whose first symbol name does not equal `expected_head`, is an error.

Duplicate keywords are not rejected by `match_form`. Lookup via `keyword(name)` returns the first match. Consumers that want to reject duplicates can do so explicitly by checking length of positional + kw count vs. expected.

## FormMatch
<!-- spec-id: mcp-tools/match-form/form-match-type -->

```rust
pub struct FormMatch<'a> {
    /* private */
}

impl<'a> FormMatch<'a> {
    pub fn head(&self) -> &str;
    pub fn positional(&self) -> &[&'a Value];
    pub fn positional_at(&self, idx: usize) -> Result<&'a Value>;
    pub fn keyword(&self, name: &str) -> Option<&'a Value>;
    pub fn require_keyword(&self, name: &str) -> Result<&'a Value>;
}
```

`FormMatch` borrows from the input `Value`; it does not allocate copies of sub-values. The lifetime parameter `'a` is the lifetime of the input `Value`. Internally, `FormMatch` stores the head string slice, a `Vec<&'a Value>` of positional borrows, and a `Vec<(String, &'a Value)>` of keyword/value pairs in source order.

### Lifetime Rules
<!-- spec-id: mcp-tools/match-form/lifetime -->

`FormMatch<'a>` borrows from the input `Value`; it cannot outlive that input. All getters return references with lifetime `'a` so callers can store the returned values in their own data structures bounded by the same lifetime. There is no owning/cloning variant; consumers who want owned values clone the returned `&Value` explicitly.

The `Vec<&'a Value>` and `Vec<(String, &'a Value)>` fields are private; they can be replaced with a different layout (e.g., `SmallVec`, `(start, len)` indices into the original list slice) without API churn.

### Head
<!-- spec-id: mcp-tools/match-form/head -->

```rust
pub fn head(&self) -> &str;
```

Returns the head symbol's name. By construction this always equals the `expected_head` passed to `match_form`. The accessor exists so callers can pass `FormMatch` to subsequent routines without re-threading the head.

### Positional
<!-- spec-id: mcp-tools/match-form/positional -->

```rust
pub fn positional(&self) -> &[&'a Value];
pub fn positional_at(&self, idx: usize) -> Result<&'a Value>;
```

`positional()` returns the slice of positional borrows in source order. `positional_at(idx)` returns the `idx`-th positional or an error mentioning the head and the missing index. `positional()` returns an empty slice for forms with no positional args.

`positional_at` errors are formatted as: `"<head>: missing positional argument at index <idx>"`.

### Keyword
<!-- spec-id: mcp-tools/match-form/keyword -->

```rust
pub fn keyword(&self, name: &str) -> Option<&'a Value>;
pub fn require_keyword(&self, name: &str) -> Result<&'a Value>;
```

`keyword(name)` returns the value for the first `:name` pair, or `None` if not present. `require_keyword(name)` returns the value or errors if the keyword is missing.

`require_keyword` errors are formatted as: `"<head>: missing required keyword :<name>"`.

The `name` argument is **without** the leading colon (consistent with `get_kw_str` and the `Value::Keyword` representation). Calling `require_keyword(":foo")` will not find a keyword written `:foo` in source — pass `"foo"` instead.

## Error Cases
<!-- spec-id: mcp-tools/match-form/error-cases -->

`match_form` returns an error in the following cases. All errors include enough context to identify the failure mode without needing source positions.

| Case | Error message format |
|---|---|
| Input is not a `Value::List` | `"expected list form (head ...), got <variant>"` |
| List is empty | `"expected list form (<expected_head> ...), got ()"` |
| Head is not a `Value::Symbol` | `"expected symbol head in form, got <variant>"` |
| Head symbol name ≠ `expected_head` | `"expected form head '<expected_head>', got '<actual>'"` |
| Positional arg appears after a keyword | `"<head>: positional argument follows keyword :<prev_kw>"` |
| Keyword has no value | `"<head>: keyword :<name> has no value"` |

These are `anyhow::Error`s produced via `anyhow!(...)`. Consumers wanting structured errors can downcast or wrap.

The error variant strings (`<variant>`) come from a small helper that returns one of `"nil"`, `"bool"`, `"integer"`, `"float"`, `"string"`, `"symbol"`, `"keyword"`, `"list"`, `"pair"`. Source positions are not attached because `match_form` operates on `Value` (no spans). Consumers who want positions can match against `Spanned` themselves or pre-convert to `Value` after capturing the relevant span.

## Design Rationale
<!-- spec-id: mcp-tools/match-form/design-rationale non-testable -->

**Why borrowed `&'a Value` rather than owned `Value`?**
The matcher is on the hot path for parser-time validation. Cloning every sub-value would double the allocation cost of every form match. Borrowing keeps the matcher free.

**Why `keyword` returns `Option`, not `Result<Option<_>>`?**
`keyword` does not perform type validation — it just looks up. There is no fallible step in the lookup itself; the only failure modes (malformed form shape) are caught at `match_form` construction time. The previous helper `get_kw_value` returns `Result<Option<_>>` because it does not assume a pre-validated form; `FormMatch::keyword` operates after validation, so the outer `Result` is gone.

**Why no built-in type-converting variants like `keyword_str`?**
The desired API in the feature request includes `keyword_str` and `require_keyword_str` that compose with the `extract` feature's typed extraction. Wiring them in 0.3 ties this matcher to that feature; it is cleaner to ship the basic shape first and add a `match-form-extract` integration in a follow-up. Consumers wanting strings today can write:

```rust
let m = match_form(&v, "define")?;
let name = m.require_keyword("name")?.as_str()
    .ok_or_else(|| anyhow!(":name must be a string"))?;
```

— two lines, no wasted machinery.

**Why is `Value::Pair` rejected?**
`(a . b)` is a dotted pair, not a list. Form shapes used by MCP tools are always proper lists, and treating a dotted pair as a list would silently accept malformed input. Rejecting matches the behavior of every existing helper (`get_kw_value`, `iter_list`).

**Why no anchor on the number of positional args?**
A consumer wanting "exactly 2 positionals" calls `m.positional_at(0)?` and `m.positional_at(1)?` and additionally checks `m.positional().len() == 2`. Building this into `match_form` would either require a parameter (verbose) or omit it (then consumers re-check anyway). Leaving it explicit matches the philosophy of the rest of the parser API.
