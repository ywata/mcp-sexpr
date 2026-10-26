(goal "covers-must-record"
  :spec (use "specs/changes/2026-10-26-covers-must-record.md")
  :outputs ((doc "coverage-baseline") (file "specs/development/covers-macro.md") (file "spec-files.txt") (file "tests/covers_macro_guard.rs") (file "src/parser/types.rs") (file "src/parser/lexer.rs") (file "src/parser/differential.rs") (file "src/parser/lexpr_compat.rs") (file "src/parser/reader.rs") (file "src/match_form/mod.rs") (file "src/pretty/mod.rs") (file "src/build/mod.rs") (file "CLAUDE.md") (file "docs/spec-trace-guide.md") (doc "verification-report"))
  :goals (
    (goal "measure-baseline"
      :spec "Before any edit, run SPEC_TRACE_DB=<scratch db> cargo test and record sqlite3 \"select count(*), sum(test_name like '%::%'), count(distinct spec_id) from coverage\" and the test pass count. Expected 111 / 0 / 41 with 208 tests (measured at d16e53f)."
      :descr "check"
      :atomic #t
      :outputs ((doc "coverage-baseline")))
    (goal "write-covers-spec"
      :spec "Create specs/development/covers-macro.md per the change spec's 'New Permanent Spec' section: title 'Coverage Macro', intro citing spec-trace spec-id spec-trace-spec/impl/runtime-reporting (covers! calls spec_trace::coverage::mark(item.spec_id()) per item under #[cfg(feature = \"runtime-coverage\")], a cfg evaluated in the calling crate, so this crate's own runtime-coverage feature (default) gates recording; rows are written when SPEC_TRACE_DB is set), '## Covers Must Record' (spec-id mcp-tools/development/covers-must-record), '## Design Rationale' (mcp-tools/development/design-rationale non-testable). Every ## heading gets a spec-id on the next line."
      :descr "realize"
      :atomic #t
      :depends ("measure-baseline")
      :inputs ((doc "coverage-baseline"))
      :outputs ((file "specs/development/covers-macro.md")))
    (goal "register-covers-spec"
      :spec "Add specs/development/covers-macro.md to spec-files.txt (never a specs/changes/ file). Run cargo build so SpecItem gains McpToolsDevelopmentCoversMustRecord. Preserve: [Existing spec-files.txt entries and order unchanged, No change spec listed]"
      :descr "transform"
      :atomic #t
      :depends ("write-covers-spec")
      :inputs ((file "specs/development/covers-macro.md"))
      :outputs ((file "spec-files.txt")))
    (goal "add-guard-test"
      :spec "Create tests/covers_macro_guard.rs (mod common; use common::{covers, SpecItem}; //! header naming specs/development/covers-macro.md). A pure detector fn returns line numbers whose trimmed start is 'macro_rules! covers'; a helper extracts fenced code block contents from markdown. Sweep test (covers McpToolsDevelopmentCoversMustRecord): walk src/ and tests/ *.rs under CARGO_MANIFEST_DIR plus CLAUDE.md code blocks, assert no hits, list path:line on failure; docs/spec-trace-guide.md excluded. Control test (same covers!): detector finds a hit in a fixture with a local definition and none in a fixture using 'use spec_trace::covers;'. Build the forbidden text with concat!/format! so no line of this file starts with 'macro_rules! covers'. The sweep is expected to fail until the 8 local definitions and CLAUDE.md are fixed; the control must pass now."
      :descr "realize"
      :atomic #t
      :depends ("register-covers-spec")
      :inputs ((file "spec-files.txt"))
      :outputs ((file "tests/covers_macro_guard.rs")))
    (goal "replace-local-covers"
      :spec "In src/parser/types.rs, src/parser/lexer.rs, src/parser/differential.rs, src/parser/lexpr_compat.rs, src/parser/reader.rs, src/match_form/mod.rs, src/pretty/mod.rs, src/build/mod.rs: delete the module-local 'macro_rules! covers' block and add 'use spec_trace::covers;' with the module's other uses. Fix any compile issue the recording macro surfaces (e.g. items or methods it needs in scope) and note what was needed. cargo test passes. Preserve: [Every existing covers!() call keeps the same SpecItem list, No test deleted or weakened, Test bodies unchanged]"
      :descr "transform"
      :atomic #t
      :depends ("add-guard-test")
      :inputs ((file "tests/covers_macro_guard.rs"))
      :outputs ((file "src/parser/types.rs") (file "src/parser/lexer.rs") (file "src/parser/differential.rs") (file "src/parser/lexpr_compat.rs") (file "src/parser/reader.rs") (file "src/match_form/mod.rs") (file "src/pretty/mod.rs") (file "src/build/mod.rs")))
    (goal "fix-claude-md-template"
      :spec "CLAUDE.md, section 'For Library Tests (src/ modules)': replace the local macro_rules! covers block with 'use spec_trace::covers;' and add a short 'Never define a local covers! macro' warning naming the failure (tests compile and pass but record zero rows) and pointing at specs/development/covers-macro.md. The guard sweep now passes. Preserve: [All other CLAUDE.md sections unchanged, Integration-test template unchanged]"
      :descr "transform"
      :atomic #t
      :depends ("replace-local-covers")
      :inputs ((file "src/parser/types.rs") (file "src/parser/lexer.rs") (file "src/parser/differential.rs") (file "src/parser/lexpr_compat.rs") (file "src/parser/reader.rs") (file "src/match_form/mod.rs") (file "src/pretty/mod.rs") (file "src/build/mod.rs"))
      :outputs ((file "CLAUDE.md")))
    (goal "refresh-provisioned-guide"
      :spec "Run mcp-planner setup --update. Confirm docs/spec-trace-guide.md no longer contains 'macro_rules! covers'. If setup reports a merge pending for a customizable file, run /merge-customizations for it (trivial merge: accept; conflicts: stop and ask the user). Do not hand-edit the guide. Report every tracked file setup changed. Preserve: [Customized project-conventions.md files keep their customizations]"
      :descr "transform"
      :atomic #t
      :depends ("fix-claude-md-template")
      :inputs ((file "CLAUDE.md"))
      :outputs ((file "docs/spec-trace-guide.md")))
    (goal "measure-and-verify"
      :spec "Rerun the baseline measurement on a fresh db: SPEC_TRACE_DB=<scratch db> cargo test, then the same sqlite3 query; the '%::%' count must rise from 0. Then cargo test --all-features, cargo clippy -- -D warnings, cargo fmt --check, RUSTDOCFLAGS='-D warnings' cargo doc --no-deps. Record before/after numbers in the change spec's Completion Record and set its Status to completed (do not archive)."
      :descr "check"
      :atomic #t
      :depends ("refresh-provisioned-guide")
      :inputs ((file "docs/spec-trace-guide.md"))
      :outputs ((doc "verification-report")))))