# Build Script

`mcp-tools` has a build script (`build.rs`) that generates the traceability enums `SpecDoc` and `SpecItem` from the spec documents listed in `spec-files.txt`, using `spec_trace::builder::Builder` with the required spec-id prefix `mcp-tools`.

Consumers take `mcp-tools` as a git dependency, so the build script also runs inside cargo's checkout of this repository (`~/.cargo/git/checkouts/mcp-sexpr-…/<rev>/`). That checkout belongs to cargo and may be read-only, for example in a confined agent environment. This document specifies where the build script may write and how the generated code is exposed.

## Generated Code Location
<!-- spec-id: mcp-tools/packaging/generated-code-location -->

- The build script writes the generated traceability code to `$OUT_DIR/traceability_gen.rs`. It writes no other file.
- It writes nothing inside the crate source tree (`CARGO_MANIFEST_DIR`). In particular, it never creates `src/traceability_gen.rs`.
- It reads only `spec-files.txt` and the spec documents that file lists.

As a result, building `mcp-tools`, whether as a git dependency, a path dependency, or in this repository, succeeds when the crate source tree is read-only.

## SpecItem Public Path
<!-- spec-id: mcp-tools/packaging/spec-item-path -->

- The generated enum is available as `mcp_tools::SpecItem`, re-exported at the crate root.
- It has one variant per spec-id in the listed documents, `non-testable` ones included. The variant name is the spec-id in UpperCamelCase with its separators removed; for example, `mcp-tools/parser/grammar` becomes `SpecItem::McpToolsParserGrammar`.
- The module holding the generated code (`traceability_gen`) is private. Only `SpecItem` is part of the public surface.

## Design Rationale
<!-- spec-id: mcp-tools/packaging/design-rationale non-testable -->

- **Why `OUT_DIR`.** Cargo designates `OUT_DIR` for build-script output, and it is always writable during a build. The source tree is not: when `mcp-tools` is a git dependency, the source tree is cargo's checkout. Before this rule, the build script regenerated `src/traceability_gen.rs` in place, and confined builds failed with `PermissionDenied` (reported by granter, msg 938). Committing the generated file would also have avoided the write, but it would let the enum drift from `specs/` and would need a manual regeneration step.
- **Inner attribute.** The spec-trace generator starts the file with `#![allow(missing_docs)]`. A file pulled in with `include!` inside `mod traceability_gen { … }` cannot contain inner attributes. So the build script removes the leading inner-attribute lines from its `OUT_DIR` output, and `src/lib.rs` puts `#[allow(missing_docs)]` on the module as an outer attribute instead. The rewrite stays inside `OUT_DIR`. If spec-trace stops emitting the attribute, the step does nothing.
- **Read-only CI guard.** The test for this spec checks only the one known file. CI also builds and tests with the source tree made read-only, so any write into the source tree, including ones added later, fails the build.
