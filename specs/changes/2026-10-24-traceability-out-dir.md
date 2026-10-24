# Generate Traceability Code Under OUT_DIR

- **Date**: 2026-10-24
- **Status**: completed
- **Type**: build (packaging fix; no API change)
- **Impact**: low — public surface unchanged; consumers pick it up only when they bump their lock/rev
- **Supersedes**: none

## Change Metadata

| Field | Value |
|---|---|
| Status | completed |
| plan-file | `specs/changes/2026-10-24-traceability-out-dir.scm` |
| updates | (none) |
| adds | `specs/packaging/build-script.md` |
| obsoletes | (none) |
| merge-into | (none) |
| new-spec-ids | `mcp-tools/packaging/generated-code-location`, `mcp-tools/packaging/spec-item-path`, `mcp-tools/packaging/design-rationale` |
| modified-spec-ids | (none) |
| retired-spec-ids | (none) |
| non-testable-sections | `mcp-tools/packaging/design-rationale` |
| consumer-notice | granter (requester, msg 938; scheduled in msg 941) — owed: report the merge commit so mcp-grant can bump its lock. Other consumers: (none) — planner and compose pin a `rev`, mailbox locks f5d0503, none use `SpecItem`, and the sentence would read "we changed where the build writes, you need do nothing" |

## Summary
<!-- spec-id: mcp-tools-changes/traceability-out-dir/summary non-testable -->

`build.rs` currently regenerates `src/traceability_gen.rs` inside the crate source tree on every run. When `mcp-tools` is built as a git dependency, that source tree is cargo's checkout (`~/.cargo/git/checkouts/mcp-sexpr-…/<rev>/`), and a confined build environment denies the write with `PermissionDenied`. This change moves the generated file to `$OUT_DIR` and includes it from there, so building `mcp-tools` writes nothing outside `OUT_DIR`. `mcp_tools::SpecItem` keeps its path and shape.

## Motivation
<!-- spec-id: mcp-tools-changes/traceability-out-dir/motivation non-testable -->

- granter msg 938: mcp-grant's role runs `cargo build`/`cargo test` confined; on a fresh build (trial `self-hosted` variant, run 20261024-005958-ad71) the `mcp-tools` build script fails writing `~/.cargo/git/checkouts/mcp-sexpr-…/f5d0503/src/traceability_gen.rs`. Everyday checkouts hide it only because `target/` already holds the build script output; a `cargo clean` or an `mcp-tools` bump reproduces it.
- lisper msg 810 had already named this move as the fix that removes the write (the PR #8 panic message only diagnoses it).
- Writing into the source tree from a build script is also against cargo's guidance: build scripts should write only under `OUT_DIR`.

## Scope
<!-- spec-id: mcp-tools-changes/traceability-out-dir/scope non-testable -->

**In scope:**
- `build.rs` writes the generated code to `$OUT_DIR/traceability_gen.rs` only.
- `src/lib.rs` includes it from `OUT_DIR`; `pub use traceability_gen::SpecItem` unchanged.
- Remove the now-unneeded `#[rustfmt::skip]` on the module and the `src/traceability_gen.rs` line in `.gitignore`.
- Update the `PermissionDenied` panic message in `build.rs` (it currently says the script writes inside the crate source tree); after this change a denial can only come from reading a spec file or from `OUT_DIR` itself.
- New permanent spec `specs/packaging/build-script.md`, added to `spec-files.txt`.
- A test pinning the location, and a CI guard that builds with the source tree read-only.

**Out of scope:**
- `docs/spec-trace-guide.md` (lines 67, 487–492 describe `src/traceability_gen.rs`): it is provider-owned (installed and overwritten by `mcp-planner setup`), so it is not edited here. If its wording should change, that is a request to its provider via granter.
- Changing spec-trace itself (e.g. an option to omit the generated file's inner attribute) — see Design Rationale.
- Customizing `.claude/skills/spec/project-conventions.md` (still template text) — separate follow-up, per the user.

## New Permanent Spec: `specs/packaging/build-script.md`
<!-- spec-id: mcp-tools-changes/traceability-out-dir/new-spec non-testable -->

Title: **Build Script**. Intro: what `build.rs` does (generates the `SpecDoc`/`SpecItem` enums from the docs listed in `spec-files.txt` using `spec_trace::builder::Builder` with required prefix `mcp-tools`) and the fact that `mcp-tools` is consumed as a git dependency, so the build script runs inside cargo's checkout.

### `## Generated Code Location` — `mcp-tools/packaging/generated-code-location` (testable)

- The build script writes the generated traceability code to `$OUT_DIR/traceability_gen.rs` and writes no other file.
- It writes nothing inside the crate source tree (`CARGO_MANIFEST_DIR`); in particular `src/traceability_gen.rs` is never created.
- It only reads `spec-files.txt` and the spec documents it lists.
- Consequence stated for consumers: building `mcp-tools` (as a git or path dependency) succeeds when the crate source tree is read-only.

### `## SpecItem Public Path` — `mcp-tools/packaging/spec-item-path` (testable)

- The generated enum is reachable as `mcp_tools::SpecItem` (crate root re-export), with one variant per testable spec-id in the listed docs, named as before (e.g. `mcp-tools/parser/grammar` → `SpecItem::McpToolsParserGrammar`).
- The generated module itself stays private (`mod traceability_gen`, not `pub mod`).

### `## Design Rationale` — `mcp-tools/packaging/design-rationale` (non-testable)

Why `OUT_DIR`; why the inner-attribute handling below; why the read-only CI guard.

## Implementation Notes
<!-- spec-id: mcp-tools-changes/traceability-out-dir/implementation non-testable -->

1. **`build.rs`**: output path becomes `PathBuf::from(env::var_os("OUT_DIR")).join("traceability_gen.rs")` (`Builder::output` takes `Into<PathBuf>`).
2. **Inner attribute**: the spec-trace generator emits `#![allow(missing_docs)]` at the top of the file. An `include!` inside `mod traceability_gen { … }` rejects inner attributes (verified: "an inner attribute is not permitted in this context"). So after `generate()`, `build.rs` rewrites the `OUT_DIR` file without leading `#![…]` lines, and `lib.rs` carries the lint as an outer attribute:
   ```rust
   // Generated traceability enum (build.rs writes it under OUT_DIR). Always available.
   #[allow(missing_docs)]
   mod traceability_gen {
       include!(concat!(env!("OUT_DIR"), "/traceability_gen.rs"));
   }
   pub use traceability_gen::SpecItem;
   ```
   The rewrite stays inside `OUT_DIR`, so the location rule holds. If spec-trace later stops emitting the inner attribute, the strip becomes a no-op.
3. **Panic message**: keep the `PermissionDenied` branch, naming the absolute output path, but reword it for the new location (read of a listed spec file, or an unwritable `OUT_DIR`/target dir).
4. **Cleanup**: delete stale local `src/traceability_gen.rs`; drop it from `.gitignore`; drop `#[rustfmt::skip]` (rustfmt does not descend into `include!`d files).
5. **Test** (`tests/packaging_build_script.rs`, `covers!` both testable ids): `Path::new(env!("OUT_DIR")).join("traceability_gen.rs")` exists; `Path::new(env!("CARGO_MANIFEST_DIR")).join("src/traceability_gen.rs")` does not; `mcp_tools::SpecItem::McpToolsParserGrammar` compiles.
6. **CI guard** (`.github/workflows/ci.yml`): one job makes the source tree read-only (`chmod -R a-w src specs spec-files.txt build.rs`) before `cargo build`/`cargo test`, so any future write into the source tree fails CI.
7. **Local verification**: `cargo clean -p mcp-tools`, read-only source tree, `cargo build && cargo test`; then `cargo fmt --check`, `cargo clippy --all-targets`.

## Design Rationale
<!-- spec-id: mcp-tools-changes/traceability-out-dir/rationale non-testable -->

- **`OUT_DIR` over a checked-in generated file**: committing `src/traceability_gen.rs` would also stop the write, but it reintroduces drift between `specs/` and the enum and needs a regeneration discipline; `OUT_DIR` keeps the generation automatic and is the location cargo designates for build-script output.
- **Strip in `build.rs` over an upstream spec-trace change**: the strip is local, small, and harmless if upstream changes. An upstream option (omit inner attributes / emit an includable file) would be cleaner and can be requested via granter later, but it is a cross-project dependency that need not block granter's I2 step.
- **Rejected: `#[path]` trick** (a generated wrapper `mod` with an absolute `#[path]`): works, but adds a second generated file and an absolute path in generated code for no gain over the strip.
- **CI guard**: the test in step 5 only checks the one known file; making the source tree read-only in CI catches any write, including future ones.

## Next Steps
<!-- spec-id: mcp-tools-changes/traceability-out-dir/next-steps non-testable -->

1. Review this proposal; then `/make-plan` (or implement directly with `Status` set to `completed` in the finalizing commit).
2. Order: write `specs/packaging/build-script.md` + add to `spec-files.txt` → `cargo build` → test with `covers!` → `build.rs`/`lib.rs` change → cleanup → CI guard → verification.
3. Check `docs/implied-tasks.md` (not present in this repo — nothing to do).
4. After merge, report the merge commit to granter (consumer-notice) and record the message id in the row.

## Completion Record (2026-10-24)
<!-- spec-id: mcp-tools-changes/traceability-out-dir/completion-record non-testable -->

- **Plan**: namespace `24ae4abd-4242-485c-a356-aaa816099e36`, tree `main`, 6 atomic goals, all complete (run via mcp-compose `plan-and-run`).
- **Spec-ids**: `mcp-tools/packaging/generated-code-location`, `mcp-tools/packaging/spec-item-path`, `mcp-tools/packaging/design-rationale` present in `specs/packaging/build-script.md`, which is listed in `spec-files.txt`; this change spec is not.
- **Tests**: `cargo clean -p mcp-tools`, then with `src specs tests spec-files.txt build.rs Cargo.toml` read-only: `cargo build` + `cargo test --all-features` — 316 passed, 0 failed; `src/traceability_gen.rs` not recreated. `cargo clippy -- -D warnings`, `cargo fmt --check`, `cargo doc --no-deps` (`-D warnings`) clean.
- **Known, out of scope**: `cargo clippy --all-targets -- -D warnings` fails on main already (`tests/parser_grammar.rs` approx_constant, `tests/parser_api.rs` type_complexity); CI does not run it.
- **Consumer notice**: granter told in flight (msg 941: scheduled, merge commit to follow). Still owed: the merge commit, after merge — send it before `/spec-change-archive`.
