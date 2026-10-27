# Server-identity primitives — shared build-id + state-fingerprint (2026-07-09)

- **Date:** 2026-07-09
- **Status:** deferred (parked) — see **Deferral note** below. Not scheduled; retained as the design of record *if* a second server ever needs build-id.
- **Type:** feature (additive) — a new optional `server-info` feature exposing reusable server-identity primitives. No change to any existing module, feature, or default surface.
- **Impact:** medium. Adds one feature-gated module (`server_info`) plus a build-script helper. Nothing in the default feature set changes; consumers opt in.
- **Supersedes:** none. This does **not** supersede `mcp-compose/specs/changes/2026-06-16-server-self-identification.md`; on reflection (see Deferral note) that **compose-local** spec is the path, and this shared design is parked.
- **Origin:** the staleness hazard is real across all long-running Rust MCP servers: a Rust edit is live only after `cargo build` **plus** the process actually restarting, and a declaration/state edit is live only after re-load — yet there is no way to ask a running server *what it is running*. The initial hypothesis was that centralising the detector would pay off across all three servers; the Deferral note records why that hypothesis was withdrawn.

## Deferral note (2026-07-09)
<!-- spec-id: mcp-tools-changes/server-identity/deferral non-testable -->

This spec is **parked**, not scheduled. Reasoning:

- The mechanism has **two independent signals**. `state-fingerprint` ("did my re-load take?") is only load-bearing for a server holding a **long-lived, re-loadable declaration set** — that is **mcp-compose alone** (`mcp-compose.scm` lingers across a whole session). **mcp-planner** consumes `.plan`/`.scm` and completes them quickly, so its loaded state doesn't linger long enough to go stale; **mcp-mailbox** is essentially stateless. So the *valuable, non-trivial* half is compose-only.
- `build-id` ("did I rebuild+restart?") **is** universal (planner has the same binary-staleness hazard), **but** the detector is ~15 lines of `build.rs` + a one-line accessor. Extracting that into a shared feature and wiring `[build-dependencies]` into three repos barely breaks even.
- The shared crate's real payoff was a **uniform response shape so one skill checks all three servers**. That payoff evaporates the moment only compose exposes the tool — which is the expected outcome.

**Decision:** build both signals **locally in mcp-compose** per `2026-06-16-server-self-identification.md`. Revisit this shared design only when a **second** server actually needs `build-id` (YAGNI — extract on the second real consumer, not speculatively). If revived, the `build-id` + build-script helper is the piece worth sharing; `state-fingerprint` would likely stay compose-local regardless.

## Change Metadata
<!-- spec-id: mcp-tools-changes/server-identity/metadata non-testable -->

| Field | Value |
|---|---|
| Status | deferred (parked) — superseded in intent by the compose-local `2026-06-16-server-self-identification.md`; see Deferral note |
| plan-file | (none) |
| updates | **Cargo.toml**: new `server-info` feature. **build.rs / spec-files.txt**: add `specs/server-info/api.md`. |
| adds | `specs/server-info/api.md` (new permanent spec, written during implementation) |
| obsoletes | (none) |
| merge-into | (none) |
| new-spec-ids | `mcp-tools/server-info/build-id`, `mcp-tools/server-info/build-script-helper`, `mcp-tools/server-info/state-fingerprint`, `mcp-tools/server-info/server-info-record`, `mcp-tools/server-info/server-identity-trait`, `mcp-tools/server-info/feature-gate`, `mcp-tools/server-info/design-rationale` |
| modified-spec-ids | (none — purely additive) |
| retired-spec-ids | (none) |
| non-testable-sections | Origin, Summary, Architecture, Adoption guidance, Non-goals, Design rationale, Cross-references |

## Summary
<!-- spec-id: mcp-tools-changes/server-identity/summary non-testable -->

`mcp-tools` gains a feature-gated `server_info` module providing four reusable primitives and a strategy trait. A server that adopts them can answer one question deterministically: *is the binary I just built, and the state I just loaded, actually the thing running right now?*

```
;; each server exposes its own (server-info) tool; the library fixes the shape:
(success :build-id           "a1b2c3d-2026-07-09T09:00:00Z"
         :state-fingerprint  "sha256:9f0a3b7c…"
         :state-source       "mcp-compose.scm"
         :item-count         21)
```

- **`build-id`** — embedded at compile time; **changes when the binary is rebuilt from changed source** → detects a stale (un-restarted) binary.
- **`state-fingerprint`** — a pure function of the server's loaded state → **changes when reloaded state differs** → detects a stale (un-reloaded) declaration/registry.

The field names are deliberately **server-neutral** (`state-*`, not `declaration-*`) so a single liveness-check skill can hit all three servers uniformly; each server documents what *its* state means.

## Motivation
<!-- spec-id: mcp-tools-changes/server-identity/motivation non-testable -->

All three MCP servers share the same silent-staleness failure:

- **mcp-compose** — a `.scm` re-import that didn't take, or an un-restarted binary after a Rust change, both look applied but have no effect.
- **mcp-planner** — language/runtime changes are equally live-only-after-rebuild+restart; registered goal-trees can be stale relative to the source.
- **mcp-mailbox** — mostly stateless, but the un-restarted-binary hazard is identical.

Three independent `server-info` implementations would duplicate the build-id embedding and the fingerprint hashing verbatim, then drift in field names and sentinel handling — defeating the goal of *uniform* liveness checks. Centralising the primitives makes the three tools thin and keeps their response shapes identical by construction.

## Scope
<!-- spec-id: mcp-tools-changes/server-identity/scope non-testable -->

**In scope (this spec, library-side):**
- `build-id` embedding: a build-script helper (called from each consumer's `build.rs`) plus a compile-time accessor macro.
- `state-fingerprint`: a pure hashing function over a canonical `(name, definition)` set.
- `ServerInfo` record type plus its canonical `Value` serialization (fixed keyword names).
- `ServerIdentity` strategy trait with a provided `server_info()` assembler.
- The `server-info` Cargo feature gating all of the above.

**Out of scope (belongs to each consumer's adoption change spec):**
- Registering and dispatching the actual `(server-info)` MCP tool in each server's rmcp router — the tool enums differ per server and cannot be shared.
- **What** each server fingerprints (`loaded_items` / `state_source` bodies) — that is the per-server strategy.
- Any auto-reload behavior — this mechanism only *reports* staleness.

## Architecture: the generic / strategy split
<!-- spec-id: mcp-tools-changes/server-identity/architecture non-testable -->

`mcp-tools` is a **library**, so it can ship everything except the two things that are irreducibly per-server: the tool wiring and the choice of what state to hash. The trait is the seam.

```rust
pub trait ServerIdentity {
    /// The server's loaded state, as canonical (name, definition) pairs.
    /// compose → registered command/module defs; planner → namespaces + goal-trees;
    /// mailbox → declared parties + pipe rules (or empty → sentinel).
    fn loaded_items(&self) -> Vec<(String, String)>;

    /// Where the loaded state came from, if it has a source path. Default: None.
    fn state_source(&self) -> Option<String> { None }

    /// Provided — identical on every server; assembles the canonical record.
    fn server_info(&self) -> ServerInfo {
        let items = self.loaded_items();
        ServerInfo {
            build_id: build_id!(),                 // compile-time const, see build-id
            item_count: items.len(),
            state_fingerprint: fingerprint(&items),
            state_source: self.state_source(),
        }
    }
}
```

A consumer's whole obligation is: `impl ServerIdentity for MySession`, wire a `(server-info)` tool that calls `.server_info().to_value()`, and add the build-script helper to `build.rs`.

## build-id
<!-- spec-id: mcp-tools/server-info/build-id -->

`build-id` is a string embedded into the **consumer** binary at compile time and read back at runtime. Its contract, independent of exact derivation:

- **Non-empty and well-formed** for every build.
- **Stable** across repeated reads within one running process (it is a compile-time constant).
- **Distinct whenever the binary differs** — a binary built from different source has a different `build-id`. This is the load-bearing property that makes a stale, un-restarted binary detectable.

The runtime accessor is a macro, `build_id!()`, exported from `mcp-tools`. It expands **in the consumer crate's** compilation context to `env!("MCP_TOOLS_BUILD_ID", "…add the mcp-tools server-info build-script helper to build.rs")`, so the value reflects the consumer's build, and a consumer that forgets the build-script wiring fails to compile with an actionable message rather than silently reporting a blank id.

Testable (library level): with `MCP_TOOLS_BUILD_ID` set for a test build, `build_id!()` yields the exact set value, is non-empty, and is byte-identical across repeated reads in one process. The "different source ⇒ different id" property is guaranteed **by construction** in the build-script helper (below) and covered by a helper-level test, not a runtime unit test.

## build-script helper
<!-- spec-id: mcp-tools/server-info/build-script-helper -->

`mcp-tools` exposes a function callable from a consumer's `build.rs` (the consumer adds `mcp-tools` with the `server-info` feature to `[build-dependencies]`):

```rust
// in the consumer's build.rs
fn main() {
    mcp_tools::server_info::build::emit_build_id();
    // …consumer's existing build.rs (e.g. spec-trace codegen)…
}
```

`emit_build_id()`:

- Computes a build identity that folds in a **per-build input** — the git `HEAD` commit (short) plus a build timestamp, degrading gracefully to timestamp-only when git metadata is unavailable (e.g. a source tarball with no `.git`).
- Prints `cargo:rustc-env=MCP_TOOLS_BUILD_ID=<value>` so the consumer crate's `build_id!()` picks it up.
- Emits the `cargo:rerun-if-changed` / `cargo:rerun-if-env-changed` directives needed so a source change re-runs the script and produces a fresh id.

Contract: distinct inputs (different commit, dirty tree, or rebuild after a source change) produce **distinct** ids; identical inputs produce identical ids. The helper never panics — missing git metadata is a fallback, not an error.

Testable: a helper-level test drives the id-derivation function with distinct synthetic inputs and asserts distinct outputs, and with identical inputs asserts identical outputs; the git-absent path returns a well-formed timestamp-only id rather than erroring.

## state-fingerprint
<!-- spec-id: mcp-tools/server-info/state-fingerprint -->

```rust
pub fn fingerprint<I>(items: I) -> String
where I: IntoIterator<Item = (String, String)>;   // (name, canonical-definition)
```

`fingerprint` is a pure function of the supplied `(name, definition)` set. Contract:

- **Deterministic** — the same set always produces the same string; it depends on the *set only*, not on process identity or time, so it is comparable **across server restarts**.
- **Order-independent** — the same pairs in a different order produce the **same** fingerprint (the function canonicalises, e.g. sorts by name, before hashing).
- **Sensitive** — adding, removing, or modifying any one pair produces a **different** fingerprint.
- **Empty sentinel** — an empty input yields a fixed, documented sentinel value (`sha256:empty` — not an error, not a blank string).
- **Prefixed & well-formed** — output is `sha256:<hex>` (or the sentinel), so consumers can render it verbatim.

Testable: deterministic and order-independent over the same set; changes on add / modify / remove of a pair; empty input returns the documented sentinel; two distinct sets return distinct fingerprints.

## ServerInfo record
<!-- spec-id: mcp-tools/server-info/server-info-record -->

```rust
pub struct ServerInfo {
    pub build_id: &'static str,
    pub state_fingerprint: String,
    pub state_source: Option<String>,
    pub item_count: usize,
}
impl ServerInfo { pub fn to_value(&self) -> Value; }
```

`to_value()` produces the canonical response body using the existing `build` constructors, with **fixed keyword names** so every adopting server reports the identical shape:

```
(success :build-id           <string>
         :state-fingerprint   <string>
         :state-source        <string|nil>
         :item-count          <int>)
```

- `state-source` renders as `nil` (not omitted) when `None`, so the record's arity is constant.
- The library fixes the keyword names precisely so one liveness-check skill parses all three servers' responses identically.

Testable: `to_value()` on a known `ServerInfo` produces the exact `Value` shape with all four keywords present and correctly typed; `state_source: None` renders `:state-source nil`; the produced `Value` round-trips through `parse_value` to an equal value.

## ServerIdentity trait
<!-- spec-id: mcp-tools/server-info/server-identity-trait -->

The trait (shown under Architecture) requires only `loaded_items`; `state_source` defaults to `None` and `server_info` is fully provided. The provided `server_info()`:

- Calls `loaded_items()` **once**, using its length for `item_count` and its contents for the fingerprint (so count and fingerprint are always consistent).
- Reads `build_id!()` and `state_source()`.
- Returns a `ServerInfo`; it performs **no** I/O and **no** mutation of the implementor.

Testable: a stub implementor with a fixed `loaded_items` yields a `ServerInfo` whose `item_count` equals the stub's length and whose `state_fingerprint` equals `fingerprint(stub_items)`; a stub returning an empty vec yields the empty sentinel and `item_count == 0`; two stubs with divergent items yield divergent fingerprints; overriding `state_source` is reflected in the result.

## Feature gate
<!-- spec-id: mcp-tools/server-info/feature-gate -->

All of the above is gated behind a new `server-info` Cargo feature, **off by default**. The feature pulls in the hashing dependency (a `sha2`-family hash) and depends on the `build` feature/module for `Value` construction. The default feature set is unchanged; a consumer that only parses S-expressions pays nothing.

Testable: the crate builds with `--no-default-features` (module absent) and with `--features server-info` (module present and usable); the `server_info` symbols are not reachable without the feature.

## Adoption guidance (per-server, follow-up specs)
<!-- spec-id: mcp-tools-changes/server-identity/adoption non-testable -->

Each server lands its own thin change spec after this ships. The pattern is the same everywhere:

1. Add `mcp-tools` with `features = [… , "server-info"]` to `[dependencies]` **and** `[build-dependencies]`.
2. Call `mcp_tools::server_info::build::emit_build_id()` at the top of `build.rs`.
3. `impl ServerIdentity` for the session/registry type — supply `loaded_items` (and `state_source` where a source path exists).
4. Register a read-only `(server-info)` tool in the server's rmcp router that returns `self.server_info().to_value()` wrapped in the server's success envelope.

Per-server `loaded_items`:
- **mcp-compose** — registered command/module names + their canonical (pretty-printed) definitions; `state_source` = last imported `.scm` path.
- **mcp-planner** — registered namespace/tree names + canonical goal-tree text; `state_source` = plan file if one was loaded, else `None`.
- **mcp-mailbox** — declared parties + active pipe rules, or empty (→ sentinel) if identity-only; build-id remains its primary signal.

## Non-goals
<!-- spec-id: mcp-tools-changes/server-identity/non-goals non-testable -->

- **Not** the MCP tool itself — the library ships primitives; each server wires its own `(server-info)`.
- **Not** a health/uptime probe — it reports *identity*, not process/dependency liveness.
- **Not** version negotiation or a capability handshake — read-only reporting, no client contract.
- **Not** auto-reload — reporting only; rebuild+restart and re-import remain the manual remedies.
- **Not** a cryptographic integrity guarantee — detects *accidental* staleness, not adversarial tampering.

## Design rationale
<!-- spec-id: mcp-tools/server-info/design-rationale non-testable -->

**Why `mcp-tools` and not a new crate?** All three servers already depend on `mcp-tools`; a feature flag adds the mechanism with zero new dependency wiring, and `mcp-tools` already carries the `build` constructors and `format` helpers the record serializer needs. A dedicated crate would buy a cleaner boundary at the cost of a second published git dependency to version across three repos — not worth it for four primitives.

**Why a build-script helper + macro, not a plain const?** `build-id` must reflect the *consumer* binary's source, which the library cannot see at its own compile time. The rustc-env is set during the consumer's build and read by a macro that expands in the consumer's crate — the only arrangement that captures the consumer's identity.

**Why server-neutral field names (`state-*`)?** The whole point of centralising is that one skill checks liveness across all three servers. If compose said `declaration-fingerprint` and planner said `tree-fingerprint`, the skill would branch per server. Fixed neutral names in the shared serializer make the responses uniform by construction; each server documents its own meaning of "state" in prose.

**Why is the tool wiring out of scope?** It is the one part that genuinely cannot be shared: each server has a distinct rmcp tool enum and success envelope. Forcing it into the library would drag the library into knowing about every server's protocol surface. Keeping it in each adoption spec is the correct boundary.

## Cross-references
<!-- spec-id: mcp-tools-changes/server-identity/cross-references non-testable -->

- `mcp-compose/specs/changes/2026-06-16-server-self-identification.md` — the compose-only precursor; to be re-scoped to adopt this mechanism.
- `specs/build/api.md` — the `Value` constructors the record serializer reuses.
- `specs/server-info/api.md` — the permanent spec written during implementation, where the `mcp-tools/server-info/*` spec-ids land.
