//! Integration tests for the build script's output location and the
//! generated `SpecItem` path.
//!
//! Spec: `specs/packaging/build-script.md`.

mod common;

use common::{covers, SpecItem};
use std::path::Path;

#[test]
fn generated_code_is_written_under_out_dir_only() {
    covers!([SpecItem::McpToolsPackagingGeneratedCodeLocation]);
    let generated = Path::new(env!("OUT_DIR")).join("traceability_gen.rs");
    let in_source_tree = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/traceability_gen.rs");

    assert!(
        generated.is_file(),
        "expected generated code at {}",
        generated.display()
    );
    assert!(
        !in_source_tree.exists(),
        "build script must not write into the source tree, found {}",
        in_source_tree.display()
    );
}

#[test]
fn spec_item_is_reachable_from_crate_root() {
    covers!([SpecItem::McpToolsPackagingSpecItemPath]);
    let item: mcp_tools::SpecItem = mcp_tools::SpecItem::McpToolsParserGrammar;

    assert_eq!(item, SpecItem::McpToolsParserGrammar);
}
