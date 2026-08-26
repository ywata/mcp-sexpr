# Differential Validation Mode

This document specifies the runtime comparison wrapper that runs both the new parser and `lexpr::from_str` on every parse during the 0.3 release. Discrepancies are deduped, reported via a configurable sink, and never propagated to the consumer.

## Version Applicability
<!-- spec-id: mcp-tools/parser/version-applicability non-testable -->

The default-on policy applies to every crate version `>= 0.3.0, < 0.4.0`. Versions `< 0.3.0` pre-date the differential wrapper and ship no comparison at all; `0.4.0` and later ship `Off` by default. A consumer can determine whether they are inside the window from `Cargo.lock` alone.

## DifferentialMode and DiscrepancySink
<!-- spec-id: mcp-tools/parser/differential-mode -->

```rust
pub enum DifferentialMode {
    Off,
    On {
        sink: DiscrepancySink,
        verbose: bool,
    },
}

pub enum DiscrepancySink {
    Stderr,
    Callback(Arc<dyn Fn(&Discrepancy) + Send + Sync>),
}

pub fn set_differential_mode(mode: DifferentialMode);
pub fn current_differential_mode() -> DifferentialMode;
```

When `On`, every call to `parse_value` and `parse_value_with_positions` invokes both parsers, structurally compares results, and reports the first divergence. The consumer always receives the new parser's result (or its error); the old parser's outcome is consulted only for the comparison.

When `Off`, only the new parser runs. This is the production path for 0.4 onwards.

### Default policy in 0.3

```rust
DifferentialMode::On {
    sink: DiscrepancySink::Stderr,
    verbose: false,
}
```

This is the value `current_differential_mode()` returns at startup unless the env var override sets a different value (see below). The 0.3 release ships **default-on** to maximize signal during the deprecation window. 0.4 ships default-off.

### Configuration order

The mode is resolved at first use according to:

1. Most recent `set_differential_mode` call from the consumer's code (highest priority).
2. Env var `MCP_TOOLS_DIFFERENTIAL_PARSE` (parsed at the first parse call after process start, then cached).
3. Compiled-in default (`On { Stderr, verbose: false }` in 0.3).

The env var accepts:

| Value | Effect |
|---|---|
| `off` (case-insensitive) | `DifferentialMode::Off` |
| `on` (case-insensitive) | `DifferentialMode::On { Stderr, verbose: false }` |
| `verbose` (case-insensitive) | `DifferentialMode::On { Stderr, verbose: true }` |

Any other value emits a one-time warning to stderr and falls through to the compiled-in default.

### Tracing sink

The 0.3 release ships only `Stderr` and `Callback` sinks. A `tracing` sink (routing through the `tracing` crate) is deferred to a later release; consumers needing tracing today wire their own callback that forwards to `tracing::warn!` or similar. Decision recorded as `q-tracing` in the change spec decisions log.

### Thread safety

`set_differential_mode` and `current_differential_mode` are thread-safe. The mode is stored behind an `RwLock`; reads (per parse) take a read lock, writes (rare) take a write lock. The `Callback` variant's closure must be `Send + Sync`.

## Discrepancy Reporting
<!-- spec-id: mcp-tools/parser/discrepancy-reporting -->

```rust
pub struct Discrepancy {
    pub input: DiscrepancyInput,
    pub new_value: Result<Value, ParseErrorRepr>,
    pub lexpr_value: Result<lexpr::Value, String>,
    pub path: StructuralPath,
}

pub enum DiscrepancyInput {
    Hashed { sha256: [u8; 32] },
    Verbose { source: String },
}

pub struct StructuralPath(pub Vec<PathElement>);

pub enum PathElement {
    ListIndex(usize),
    PairCar,
    PairCdr,
    Atom,
}

pub struct ParseErrorRepr {
    pub kind: String,
    pub message: String,
}
```

The `path` field locates the first divergence within the parsed tree:

- `ListIndex(i)` — child `i` of a list disagrees.
- `PairCar` / `PairCdr` — head or tail of a dotted pair disagrees.
- `Atom` — the values themselves are atoms that disagree (different variants, different content, or one parsed and the other errored).

The path is a sequence: `[ListIndex(0), ListIndex(2), PairCar]` reads as "child 0, then child 2, then car of the pair".

### Hashed vs verbose inputs

By default, `DiscrepancyInput::Hashed` stores a SHA-256 of the input string. This protects user data: MCP S-expression payloads frequently carry secrets (paths, names, internal IDs). The 32-byte hash is sufficient to deduplicate identical inputs and to confirm reproduction in a controlled environment.

When `DifferentialMode::On { verbose: true, .. }`, the full source string is captured in `DiscrepancyInput::Verbose`. Verbose mode is opt-in and intended for development and bug-bash environments. Decision recorded as `q-verbose` in the change spec decisions log.

### Non-fatal reporting

Reporting a discrepancy never affects the consumer's parse outcome:

- The new parser's `Result<Value>` is returned to the caller verbatim.
- If the new parser succeeds and lexpr fails, the discrepancy is reported and the new parser's `Ok(value)` is returned.
- If the new parser fails and lexpr succeeds, the discrepancy is reported and the new parser's `Err(...)` is returned.
- If both fail, the discrepancy may still be reported (different error messages or kinds count as a discrepancy in `verbose` mode only) but the consumer sees the new parser's error.
- Sink failures (e.g., callback panics, stderr write errors) are caught and swallowed — no panic propagates to the parse caller.

### Stderr sink format

The `Stderr` sink writes one human-readable report per discrepancy. The report text is produced by a pure function, `format_report(&Discrepancy, verbose: bool) -> String`, and the sink does nothing but write that string; tests exercise the formatter directly rather than capturing the process's stderr. The report's shape depends on `verbose`:

```
# verbose: false  (the compiled-in default)
[mcp-tools differential] new=<kind> lexpr=<kind> path=<path>
  input-sha256=<hex>

# verbose: true
[mcp-tools differential] new=<repr> lexpr=<repr> path=<path>
  input=<source>
```

- `<kind>` is the **variant name only**: for an `Ok` value the `Value` / `lexpr::Value` variant (`Keyword`, `Symbol`, `List`, `Cons`, …); for an `Err` on the new-parser side `Err(<ParseErrorRepr.kind>)` (e.g. `Err(IntegerOutOfRange)`); for an `Err` on the lexpr side the bare token `Err`. Never the contents of a string, symbol, keyword, or number, and never an error message.
- `<repr>` is `Ok(<one-line debug>)` or `Err(<kind>: <message>)` — the full form.
- `<path>` is the dotted form `0.2.car` for `[ListIndex(0), ListIndex(2), PairCar]`.
- The leading prefix `[mcp-tools differential]` is fixed so consumers can grep their logs.
- In verbose mode the `input=<source>` line carries the source string raw (no escaping).

**Hashed-mode invariant**: when `verbose` is `false`, no byte of the source string, and no byte derived from it other than its SHA-256, reaches stderr. Error messages fall under this rule because `ParseError` messages and lexpr error strings can quote the offending input. The rationale is the same as for hashing the input in the first place: a payload's secrets survive parsing, so printing the parse tree would defeat the hash.

The `Discrepancy` struct passed to `Callback` sinks is **unchanged** by this rule and carries full values in both modes: a consumer who wires a callback has chosen where those bytes go.

Test obligations:

- Hashed mode, an input containing a sentinel such as `SECRET-9f3a` engineered to diverge (e.g. via a bignum sibling): the formatted report does not contain the sentinel.
- Verbose mode, same input: the formatted report does contain it.
- Hashed mode, an `Err` on one side: the report contains the error *kind* and not the message text.

### Callback sink

The `Callback` variant invokes the closure synchronously on the parse-calling thread, holding no internal lock. The closure is responsible for any rate limiting beyond the LRU dedup. Callbacks must not call `parse_value` recursively — doing so leads to a re-entrant lock (the discrepancy reporter does not hold a lock during callback dispatch, but a callback that re-parses and triggers another discrepancy can spam unboundedly).

## Expected Discrepancy Classes
<!-- spec-id: mcp-tools/parser/expected-discrepancy-classes -->

An *expected class* is a `(new, lexpr)` value pair that differs only in representation, where the new parser's reading is the documented canonical form. Expected classes are **not discrepancies**: `compare_values` returns `None` for them, so nothing is hashed, deduplicated, or dispatched. The rule table lives in one pure function, `is_expected_class(&Value, &lexpr::Value) -> bool`, consulted by `compare_values` before its variant-mismatch fallthrough.

The 0.3 expected classes:

| New parser | lexpr | Rule |
|---|---|---|
| `Value::Keyword(k)` | `lexpr::Value::Symbol(s)` where `s == format!(":{k}")` | equal |
| `Value::Keyword(k)` | `lexpr::Value::Keyword(k)` | equal (already) |
| `Value::Nil` | `lexpr::Value::Symbol("nil")` | equal |
| `Value::List(items)` | improper cons chain whose final cdr is `Symbol("nil")` | compared element-wise as a proper list (tail accepted like `Null`) |

The two `nil` rows exist because `grammar.md` reserves the bare token `nil` (`nil := "nil" | "()"`, both `Value::Nil`) while lexpr's reader has no such rule and yields `Symbol("nil")`. In atom position that is the first row. In tail position — `(a . nil)`, which the grammar reads as `(a . ())` = `(a)` — the new parser produces a proper `List` while lexpr produces an improper cons ending in `Symbol("nil")`; the second row lets `compare_list` accept that tail exactly as it accepts `Null`. Both matches are byte-exact, as in the grammar: `NIL` is an ordinary symbol to both parsers and never diverges. (`()` needs no row — lexpr reads it as `Null`, which the comparison already treats as equal to `Nil`.) An improper chain ending in any *other* symbol is still a structural discrepancy, reported at `atom` as before.

Nothing else is in the table. In particular the two directions are **not** symmetric: new-parser `Symbol(":k")` vs lexpr `Keyword("k")` remains a reported discrepancy at `atom`, because it would mean the new parser failed to canonicalise a keyword — exactly the class of bug the mode exists to catch. Likewise new-parser `Symbol("nil")` vs lexpr `Nil` or `Null` remains reported at `atom`: it would mean the new parser failed to apply its own grammar.

### Test obligations

- `(tool :key "v")` produces no discrepancy (`compare` returns `None`).
- `(:a . :b)` dotted keywords produce no discrepancy.
- A keyword nested three lists deep produces no discrepancy.
- The reverse pair (new `Symbol(":k")`, lexpr `Keyword("k")`), constructed directly on the `compare_values` API, **is** reported at `atom`.
- `(a nil)`, `(a (b (c nil)))`, `(nil . nil)`, `(a . nil)`, and `(a b . nil)` produce no discrepancy.
- The reverse pairs (new `Symbol("nil")`, lexpr `Nil`) and (new `Symbol("nil")`, lexpr `Null`), constructed directly on `compare_values`, **are** reported at `atom`.
- A `List` against an improper chain ending in `Symbol("other")`, constructed directly on `compare_values`, **is** still reported at `atom`.
- The rule is byte-exact: `is_expected_class(Nil, Symbol("NIL"))` is false. `(a NIL)` produces no discrepancy for a different reason — `NIL` is an ordinary symbol to both parsers — so the byte-exactness obligation is on the rule function, not on a parse.

## Discrepancy Deduplication
<!-- spec-id: mcp-tools/parser/discrepancy-deduplication -->

Each discrepancy is reported at most once per process lifetime per unique input. Uniqueness is measured by SHA-256 of the input string (computed once and reused for both deduplication and the optional `Hashed` reporting form). A second, independent key — the discrepancy's *class* — is applied after this one; see **Discrepancy Class Deduplication**.

```rust
pub fn set_discrepancy_dedup_capacity(capacity: usize);
pub fn flush_discrepancy_dedup();
```

The dedup cache is a bounded LRU with a default capacity of **1024 entries**. When full, the least-recently-seen hash is evicted; that input may be reported again on a subsequent parse.

`flush_discrepancy_dedup` clears the cache; the next parse of any input — even one previously reported — will report again. Long-running processes (e.g., persistent MCP servers) call this on a schedule (per hour, per N parses) if they want fresh signal without restart.

`set_discrepancy_dedup_capacity` resizes the cache. Reducing capacity below the current entry count evicts least-recently-seen entries until the cache fits. Setting capacity to zero disables deduplication entirely (every parse reports every discrepancy).

### Cache scope

The dedup cache is process-global, behind a `Mutex<LruCache<[u8; 32], ()>>`. There is no per-thread cache and no per-mode cache; switching `DifferentialMode::Off` and back to `On` does not flush the cache. Test harnesses that need a clean slate call `flush_discrepancy_dedup()` in test setup.

### Cost

The cache lookup is O(1). Hashing the input is O(n) in input length but happens only when differential mode is `On` — turning it off is a single read-lock check before any work.

## Discrepancy Class Deduplication
<!-- spec-id: mcp-tools/parser/discrepancy-class-deduplication -->

Input-hash deduplication suppresses a *repeated input*; it cannot suppress a *repeated class*. A consumer that parses many small, distinct payloads — an MCP tool server — would otherwise get one report per payload for a single grammar-level difference. A second cache, keyed by the discrepancy's class, collapses those to one.

A discrepancy's **class** is the triple:

```rust
struct DiscrepancyClass {
    path: StructuralPath,      // as reported, list indices included
    new_kind: NewKind,         // ValueKind for Ok, or the ParseErrorRepr.kind for Err
    lexpr_kind: LexprKind,     // lexpr::Value variant for Ok, or Err
}

pub fn set_discrepancy_class_dedup_capacity(capacity: usize);
```

`ValueKind` and `LexprKind` are small enums enumerating the variant names of `Value` and `lexpr::Value` respectively — enums, not strings, with an exhaustive match so that adding a `Value` variant is a compile-time error here rather than a silently unclassified report. The same enums drive the `<kind>` tokens in the hashed-mode stderr line.

Suppression rule — a discrepancy is dispatched only if **both** hold:

1. its input hash is not in the input LRU (existing rule, default capacity 1024, unchanged), and
2. its class is not in the class LRU (default capacity **256**).

Both caches are updated whenever their lookup misses, before dispatch. `set_discrepancy_dedup_capacity` continues to resize the input cache only. `set_discrepancy_class_dedup_capacity` resizes the class cache; `0` disables class deduplication (every input-cache miss is dispatched — the pre-0.3.0 behaviour). `flush_discrepancy_dedup` clears both caches.

The path keeps its list indices: `1.atom` and `2.atom` are different classes. Collapsing indices would hide a divergence that appears at a new position in a structurally different payload, and the remaining volume is already bounded by the stderr report budget.

### Test obligations

- 100 distinct inputs that all diverge as `(1.atom, Integer, Number)` (bignum siblings): the `Callback` sink is invoked exactly once.
- Two inputs diverging at `1.atom` and `2.atom` respectively: invoked twice.
- The same input twice: invoked once (input LRU still applies; class LRU does not double-count).
- `set_discrepancy_class_dedup_capacity(0)` then 100 distinct inputs of one class: invoked 100 times.
- Class LRU eviction: capacity 2, three classes in rotation — the first class is reported again after eviction.

## Stderr Report Budget
<!-- spec-id: mcp-tools/parser/stderr-report-budget -->

The `Stderr` sink is bounded per process. A stdio MCP server's client is not obliged to read the server's stderr, so an unbounded diagnostic stream can fill the pipe buffer and block the server's request loop on `eprintln!` — silently, mid-request. The budget makes that impossible regardless of input volume.

```rust
pub const STDERR_REPORT_BUDGET: usize = 64;

enum BudgetDecision { Emit, EmitExhausted, Drop }

fn budget_decision(count_before: usize, budget: usize) -> BudgetDecision;
```

The sink holds a process-wide monotonic counter of reports it has been asked to write. For each report, `count_before` is the counter value prior to this report, and:

- `count_before < budget` → `Emit`: the report is written as specified in **Stderr sink format**.
- `count_before == budget` → `EmitExhausted`: the sink writes exactly one terminal line and nothing else:
  ```
  [mcp-tools differential] report budget (64) exhausted; further reports suppressed. Use DiscrepancySink::Callback or MCP_TOOLS_DIFFERENTIAL_PARSE=off.
  ```
- `count_before > budget` → `Drop`: the report is dropped **before formatting**. No further stderr writes occur from this module for the life of the process.

Rules:

- The counter is **never reset** by `set_differential_mode`. Switching the sink to `Callback` and back does not refill the budget; the budget exists to bound total bytes on a stream the consumer may never read, and a refill would reopen the hazard.
- The budget applies to the `Stderr` sink only. `Callback` dispatch is unlimited — the consumer owns that path.
- The budget is checked **after** both deduplication caches (input-hash and class), so a budgeted report always corresponds to a fresh `(input, class)` pair. A deduplicated report never consumes budget.
- `STDERR_REPORT_BUDGET` is a constant, not a setter — a configurable budget would invite consumers to reopen the hazard.

Bound: 64 hashed-mode reports of ≤ ~200 bytes each plus the terminal line total under 16 KB, less than a quarter of the smallest common pipe buffer (64 KB). This is the property that removes the deadlock, and it holds independently of whether expected-class suppression or class deduplication fire.

### Test obligations

- Drive `budget_decision` over `count_before` in `0..70` with budget 64: exactly 64 `Emit`, 1 `EmitExhausted`, 5 `Drop`. The decision is a pure function so the test never touches the real stderr.
- Switching the mode to `Callback` and back after exhaustion: the counter is unchanged and the next stderr report is still `Drop`.
- Dedup-before-budget: a duplicate input submitted after 63 fresh reports does not advance the counter.

## Operating Notes
<!-- spec-id: mcp-tools/parser/differential-operating-notes non-testable -->

Expected discrepancy categories during 0.3:

- **Numeric out-of-range**: lexpr accepts bignums; the new parser errors. Reported as `new=Err(OutOfRange) lexpr=Ok(...)`.
- **Rational/complex**: same shape — lexpr accepts, new parser errors at lex time.
- **Comment positions**: not compared (differential ignores spans/comments).

Suppressed by rule (never reported — see **Expected Discrepancy Classes**):

- **Keyword normalization**: the new parser canonicalizes keywords to `Keyword("foo")`; lexpr reads the same token as `Symbol(":foo")`. The pair is an expected class and is treated as equal by the comparison.
- **`nil` token**: the grammar reserves `nil` as `Value::Nil`; lexpr reads it as `Symbol("nil")`. Expected class, treated as equal (byte-exact — `NIL` is a symbol to both).

Categories that should produce zero reports:

- Boolean values, nil, lists with i64 / f64 / string atoms.
- Quote desugaring (both parsers desugar identically).
- String escape handling (the new parser's escape policy is a strict subset of lexpr's, but every escape the new parser accepts, lexpr also accepts).

Any non-zero report in the second category is a bug in the new parser and should be filed against this crate.
