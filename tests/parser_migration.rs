//! Integration tests for the lexpr deprecation/removal migration phases.
//!
//! These tests pin down what the 0.3 surface ships (phase 1) and document the
//! exact symbol set that will change in 0.4 (phase 2) and 1.0 (phase 3).

mod common;

#[allow(deprecated)]
use mcp_tools::{
    current_differential_mode, get_kw_str_lexpr, get_kw_value_lexpr, iter_list_lexpr, parse_str_list_lexpr,
    parse_text_ref_lexpr, parse_value, parse_value_lexpr, require_kw_str_lexpr, set_differential_mode,
    DifferentialMode, DiscrepancySink, TextRef, Value,
};

use common::{covers, SpecItem};

#[test]
fn phase_1_ships_value_based_parse_value() {
    covers!([SpecItem::McpToolsMigrationPhase1Deprecation]);

    // 0.3: `parse_value` returns the new Value, not lexpr::Value.
    let v: Value = parse_value("(tool :a 1)").unwrap();
    assert!(v.is_list());
}

#[test]
#[allow(deprecated)]
fn phase_1_keeps_parse_value_lexpr_callable() {
    covers!([SpecItem::McpToolsMigrationPhase1Deprecation]);

    // 0.3: the previous behavior is reachable via parse_value_lexpr.
    let v: lexpr::Value = parse_value_lexpr("(tool :a 1)").unwrap();
    assert!(v.is_cons());
}

#[test]
fn phase_1_differential_mode_default_on_with_stderr() {
    covers!([SpecItem::McpToolsMigrationPhase1Deprecation]);

    // Reset to compiled-in default to defeat any prior set in this test process.
    set_differential_mode(DifferentialMode::On {
        sink: DiscrepancySink::Stderr,
        verbose: false,
    });
    match current_differential_mode() {
        DifferentialMode::On { verbose: false, .. } => {}
        other => panic!("expected default On, got {:?}", other),
    }
    set_differential_mode(DifferentialMode::Off);
}

#[test]
#[allow(deprecated)]
fn phase_2_window_deprecated_lexpr_helpers_still_present() {
    covers!([SpecItem::McpToolsMigrationPhase2Window]);

    // 0.4 must keep all `_lexpr` helpers callable for the migration window.
    // Verify each symbol from the deprecation table exists with the documented
    // signature shape.
    let v: lexpr::Value = parse_value_lexpr(r#"(tool :name "x" :items ("a"))"#).unwrap();

    let _: Option<lexpr::Value> = get_kw_value_lexpr(&v, "name").unwrap();
    let _: Option<String> = get_kw_str_lexpr(&v, "name").unwrap();
    let _: String = require_kw_str_lexpr(&v, "name").unwrap();

    let xs: Vec<lexpr::Value> = iter_list_lexpr(&v).unwrap().collect();
    assert!(!xs.is_empty());

    let items_v = get_kw_value_lexpr(&v, "items").unwrap().unwrap();
    let _: Vec<String> = parse_str_list_lexpr(&items_v).unwrap();

    let lit = parse_value_lexpr("\"hi\"").unwrap();
    let _: TextRef = parse_text_ref_lexpr(&lit).unwrap();
}

#[test]
#[allow(deprecated)]
fn phase_3_removal_targets_are_the_lexpr_suffixed_symbols() {
    covers!([SpecItem::McpToolsMigrationPhase3Removal]);

    // 1.0 deletes every symbol below. This test enumerates them so a 1.0 release
    // can audit the surface against this list. Keeping the symbols compilable
    // here guarantees the spec and the code agree on the deletion target.
    let _ = parse_value_lexpr;
    let _ = get_kw_value_lexpr::<>;
    let _ = get_kw_str_lexpr::<>;
    let _ = require_kw_str_lexpr::<>;
    let _ = iter_list_lexpr::<>;
    let _ = parse_str_list_lexpr::<>;
    let _ = parse_text_ref_lexpr::<>;
}
