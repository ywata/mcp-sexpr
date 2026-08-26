# Change Spec: differential-parse reporting is default-on, unbounded on our input shape, and can deadlock a stdio consumer

- **Date**: 2026-08-26
- **Status**: proposed — **external request**, raised by the deep-rev project. Filed here for
  hand-over; mcp-tools owns whether and how any of it lands.
- **Type**: three defect reports (D1–D3) and one ask (A1)
- **Impact**: `src/parser/differential.rs` — the default mode, the deduplication key, and the
  contents of a stderr report. Nothing here asks for a change to the parser itself, or to any
  parse result. **No consumer-visible parse behaviour is in scope**: the wrapper already never
  propagates a discrepancy, and we are not disputing that.
- **Severity, in one line**: this cost us a misfiled defect. We recorded a hang as *our* server's
  size ceiling, chased it, and found it was diagnostics filling an undrained pipe. Details in D3.

## Build tested

- **Crate**: `mcp-tools`, `version = "0.2.0"`, resolved by our `Cargo.lock` to
  `git+ssh://git@github.com/ywata/mcp-sexpr.git#f09e3f9dfa520c0d3f0a2fc680c3406676c2c001`.
- **Consumer**: `mcp-iter` (deep-rev), an MCP server on the **stdio** transport (`rmcp` 0.9),
  driven over JSON-RPC by a test harness and by Claude Code.
- **Observed**: 2026-08-26, on a debug build, with `MCP_TOOLS_DIFFERENTIAL_PARSE` unset.
- All measurements below are from captured `stderr` files, not from recollection.

**A version-identification problem, and it is why we are unsure whether any of this is already
known.** `Cargo.toml` in the pinned rev says `version = "0.2.0"`, while
`specs/parser/differential-mode.md` in the *same* rev describes the policy as belonging to "the 0.3
release" — "the 0.3 release ships **default-on** to maximize signal during the deprecation window.
0.4 ships default-off." A consumer reading the version it depends on therefore cannot tell whether
it is inside the default-on window. If 0.2.0 is simply pre-bump, saying so in the spec would settle
it.

## D1 — the discrepancy fires on every keyword-bearing parse, and always at the same path

Every report we captured is the same class: the new parser produces a `Keyword`, the `lexpr` path
produces a `Symbol` whose name carries the leading colon. Verbatim, for
`(begin-iteration :strategy "scout")`:

```
[mcp-tools differential] new=Ok(List([Symbol("begin-iteration"), Keyword("strategy"), String("scout")]))
                         lexpr=Ok(Cons((Symbol("begin-iteration") . Cons((Symbol(":strategy") . Cons((String("scout") . Null)))))))
                         path=1.atom
```

Across a single `tools/call` carrying 100 records we captured **203 reports, all `path=1.atom`** —
one per parse, and our tool re-parses each record individually, so a batch of N records is N+1
parses.

Every tool in our surface takes keyword arguments, so **every parse we perform diverges.** If this
is a known representational difference rather than a parser bug — and `Keyword("strategy")` versus
`Symbol(":strategy")` reads like a deliberate improvement, not a defect — then the comparison is
reporting a difference it is not meant to catch, on 100% of inputs, and the mode produces no signal
for consumers whose s-expressions use keywords.

**Ask**: either normalise a keyword to its `lexpr` spelling before comparing, or classify this path
as an expected difference and stop reporting it.

## D2 — deduplication is keyed by the input, but the divergence is a property of the grammar

`specs/parser/differential-mode.md` specifies a bounded LRU keyed by the input (hash by default,
capacity 1024). That suppresses a *repeated input*. It cannot suppress a *repeated class*: our 203
reports were 203 distinct inputs, so each was a cache miss, and each printed in full. A consumer
that parses many small distinct payloads — which is what an MCP tool server is — gets one report per
payload, unbounded, for one underlying difference.

**Ask**: dedup on the discrepancy class as well as the input — `report.path` plus the pair of node
kinds would already collapse our 203 to 1.

## D3 — the report writes payload contents to stderr, which defeats the hashing and can deadlock a stdio consumer

Two separate problems, both in what the sink emits.

**It prints what the hash was chosen to protect.** The spec's rationale for hashing the input is
explicit: *"MCP S-expression payloads frequently carry secrets (paths, names, internal IDs). The
32-byte hash is sufficient to deduplicate identical inputs."* But the report prints
`new={}` and `lexpr={}` — **both parse trees, verbatim** — beside the hash. The capture in D1's
sibling case contains a full filesystem path from our machine, twice. The input is hashed and the
parse of that input is not, so the protection does not hold for any payload whose secret survives
parsing, which is all of them.

**It can deadlock the consumer.** `DiscrepancySink::Stderr` is the compiled-in default, and a large
class of consumers are **stdio MCP servers**, whose client is not obliged to read stderr — the MCP
stdio transport treats it as optional. Measured on our server:

| | stderr emitted |
|---|---|
| one `tools/call` with 100 records | **181,495 bytes** |
| one investigation sweep (1,129 records) | **2,223,984 bytes** |

With a client that does not drain the pipe, the server blocks on `eprintln!` once the 64 KB pipe
buffer fills — **mid-batch, after a partial commit**, with no error and no timeout. Reproduced
deterministically:

| batch | client drains stderr | client leaves stderr undrained |
|---|---|---|
| 20 records / 2.8 KB | ok, 0.0 s | ok, 0.0 s |
| 50 records / 7.1 KB | ok, 0.1 s | **hangs forever**, 21 of 50 rows committed |
| 400 records / 57 KB | ok, 0.5 s | **hangs forever**, 0 rows committed |

We first recorded this as a defect in *our* writer — "a batch write of ~50 records hangs over stdio,
after a partial commit" — because from inside the harness that is exactly what it looks like: a
reproducible hang at a size threshold, in the write path. It took a separate experiment, changing
one variable, to find that the blocked write was a diagnostic. **A library that can block its
consumer's request loop by writing to a stream the consumer never promised to read is a hazard
independent of what it writes.**

**Ask**: default the sink to `Off` for library consumers, or land the `tracing` sink the spec defers
(`q-tracing`) and default to it, so the output goes somewhere the consumer has already wired.
Failing either, `verbose: false` should mean the trees are omitted, not just the source string.

## A1 — say in the spec which released version is default-on

Purely documentation, and it is what would have let us answer "does this affect us?" without
reading `default_mode()`. See the note under **Build tested**.

## What we did locally, and why it is not a fix

Our sandbox image now sets `ENV MCP_TOOLS_DIFFERENTIAL_PARSE=off`, with the measurements above in
the comment. That silences the symptom for us and gives up the mode's signal entirely — which is a
poor trade if D1 is fixed, since then the reports would be worth reading. We would rather have D1
and D3 addressed and the mode back on.

## Reproducer

No part of this needs our project. Any consumer of `mcp_tools::parse_value` reproduces D1 and D2:

```rust
// stderr receives one report per call, all path=1.atom
for i in 0..100 {
    let _ = mcp_tools::parse_value(&format!("(tool :key \"value-{}\")", i));
}
```

D3's deadlock needs only a parent process that opens the child's stderr as a pipe and does not read
it, which is the default for most process-spawning libraries when stderr is captured at all.
