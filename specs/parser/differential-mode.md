# Differential Validation Mode

This document specifies the runtime comparison wrapper that runs both the new parser and `lexpr::from_str` on every parse during the 0.3 release. Discrepancies are deduped, reported via a configurable sink, and never propagated to the consumer.

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

The `Stderr` sink writes one human-readable line per discrepancy:

```
[mcp-tools differential] new=<repr> lexpr=<repr> path=<path>
```

`<repr>` is `Ok(<one-line debug>)` or `Err(<kind>: <message>)`. `<path>` is the dotted form `0.2.car` for `[ListIndex(0), ListIndex(2), PairCar]`. The leading prefix `[mcp-tools differential]` is fixed so consumers can grep their logs.

In verbose mode, an additional indented line follows:

```
  input=<source>
```

with the source string raw (no escaping). The hashed mode does not emit the input at all, only the hash:

```
  input-sha256=<hex>
```

### Callback sink

The `Callback` variant invokes the closure synchronously on the parse-calling thread, holding no internal lock. The closure is responsible for any rate limiting beyond the LRU dedup. Callbacks must not call `parse_value` recursively — doing so leads to a re-entrant lock (the discrepancy reporter does not hold a lock during callback dispatch, but a callback that re-parses and triggers another discrepancy can spam unboundedly).

## Discrepancy Deduplication
<!-- spec-id: mcp-tools/parser/discrepancy-deduplication -->

Each discrepancy is reported at most once per process lifetime per unique input. Uniqueness is measured by SHA-256 of the input string (computed once and reused for both deduplication and the optional `Hashed` reporting form).

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

## Operating Notes
<!-- spec-id: mcp-tools/parser/differential-operating-notes non-testable -->

Expected discrepancy categories during 0.3:

- **Numeric out-of-range**: lexpr accepts bignums; the new parser errors. Reported as `new=Err(OutOfRange) lexpr=Ok(...)`.
- **Rational/complex**: same shape — lexpr accepts, new parser errors at lex time.
- **Keyword normalization**: the new parser canonicalizes keywords; lexpr's mixed `Symbol(":foo")` / `Keyword("foo")` may disagree at the symbol/keyword variant boundary.
- **Comment positions**: not compared (differential ignores spans/comments).

Categories that should produce zero reports:

- Boolean values, nil, lists with i64 / f64 / string atoms.
- Quote desugaring (both parsers desugar identically).
- String escape handling (the new parser's escape policy is a strict subset of lexpr's, but every escape the new parser accepts, lexpr also accepts).

Any non-zero report in the second category is a bug in the new parser and should be filed against this crate.
