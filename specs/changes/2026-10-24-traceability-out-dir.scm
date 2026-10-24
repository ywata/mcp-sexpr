(goal "traceability-out-dir"
  :spec (use "specs/changes/2026-10-24-traceability-out-dir.md")
  :outputs ((file "specs/packaging/build-script.md") (file "spec-files.txt") (file "tests/packaging_build_script.rs") (file "build.rs") (file "src/lib.rs") (file ".gitignore") (file ".github/workflows/ci.yml") (doc "verification-report"))
  :goals (
    (goal "write-packaging-spec"
      :spec "Create specs/packaging/build-script.md per the change spec's 'New Permanent Spec' section: title, intro, '## Generated Code Location' (spec-id mcp-tools/packaging/generated-code-location), '## SpecItem Public Path' (mcp-tools/packaging/spec-item-path), '## Design Rationale' (mcp-tools/packaging/design-rationale non-testable). Every ## heading gets a spec-id on the next line."
      :descr "realize"
      :atomic #t
      :outputs ((file "specs/packaging/build-script.md")))
    (goal "register-packaging-spec"
      :spec "Add specs/packaging/build-script.md to spec-files.txt (never a specs/changes/ file). Run cargo build so SpecItem gains McpToolsPackagingGeneratedCodeLocation and McpToolsPackagingSpecItemPath. (specs/README.md does not exist in this repo; nothing to index.). Preserve: [Existing spec-files.txt entries and order unchanged, No change spec listed]"
      :descr "transform"
      :atomic #t
      :depends ("write-packaging-spec")
      :inputs ((file "specs/packaging/build-script.md"))
      :outputs ((file "spec-files.txt")))
    (goal "add-location-test"
      :spec "Create tests/packaging_build_script.rs. Test 1 covers!([SpecItem::McpToolsPackagingGeneratedCodeLocation]): Path::new(env!(\"OUT_DIR\")).join(\"traceability_gen.rs\") exists and Path::new(env!(\"CARGO_MANIFEST_DIR\")).join(\"src/traceability_gen.rs\") does not exist. Test 2 covers!([SpecItem::McpToolsPackagingSpecItemPath]): mcp_tools::SpecItem::McpToolsParserGrammar is usable via the crate-root path. Use the integration-test pattern of tests/build_api.rs: 'mod common; use common::{covers, SpecItem};' plus a //! header naming specs/packaging/build-script.md. Test 1 is expected to fail until the build.rs change lands."
      :descr "realize"
      :atomic #t
      :depends ("register-packaging-spec")
      :inputs ((file "spec-files.txt"))
      :outputs ((file "tests/packaging_build_script.rs")))
    (goal "move-output-to-out-dir"
      :spec "build.rs: write generated code to PathBuf::from(env::var_os(\"OUT_DIR\")).join(\"traceability_gen.rs\"); after generate(), rewrite that OUT_DIR file without its leading #![...] inner-attribute lines (stay inside OUT_DIR). Reword the PermissionDenied panic message for the new location (unreadable listed spec file or unwritable OUT_DIR/target dir), still naming the absolute output path. src/lib.rs: replace '#[rustfmt::skip] mod traceability_gen;' with '#[allow(missing_docs)] mod traceability_gen { include!(concat!(env!(\"OUT_DIR\"), \"/traceability_gen.rs\")); }' and update the comment; keep 'pub use traceability_gen::SpecItem;'. Remove the src/traceability_gen.rs line from .gitignore and delete the stale local src/traceability_gen.rs. Preserve: [mcp_tools::SpecItem public path and variant names unchanged, traceability_gen module stays private, Builder still uses require_prefix(\"mcp-tools\") and spec-files.txt docs, rerun-if-changed directives kept]"
      :descr "transform"
      :atomic #t
      :depends ("add-location-test")
      :inputs ((file "tests/packaging_build_script.rs"))
      :outputs ((file "build.rs") (file "src/lib.rs") (file ".gitignore")))
    (goal "add-readonly-ci-guard"
      :spec "In .github/workflows/ci.yml, in one job before cargo build/test, make the source tree read-only (chmod -R a-w src specs tests spec-files.txt build.rs Cargo.toml) so any build-time write into the source tree fails CI. Preserve: [Existing ssh-agent deploy-key steps and CARGO_NET_GIT_FETCH_WITH_CLI kept, Other jobs and steps unchanged]"
      :descr "transform"
      :atomic #t
      :depends ("move-output-to-out-dir")
      :inputs ((file "build.rs") (file "src/lib.rs") (file ".gitignore"))
      :outputs ((file ".github/workflows/ci.yml")))
    (goal "verify-readonly-build"
      :spec "Locally, mirroring CI: cargo clean -p mcp-tools; chmod -R a-w the same paths as the CI guard; cargo build && cargo test --all-features (all pass, incl. tests/packaging_build_script.rs); restore write permission (chmod -R u+w) even on failure; confirm src/traceability_gen.rs was not recreated; cargo clippy -- -D warnings; cargo clippy --all-targets -- -D warnings; cargo fmt --check; cargo doc --no-deps. Then set the change spec Status to completed."
      :descr "check"
      :atomic #t
      :depends ("add-readonly-ci-guard")
      :inputs ((file ".github/workflows/ci.yml"))
      :outputs ((doc "verification-report")))))