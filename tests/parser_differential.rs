//! Integration tests for the differential validation mode.
//!
//! These tests share global state (the differential mode + dedup cache), so each
//! test acquires `TEST_LOCK` before touching it. Multiple tests running in
//! parallel would otherwise interleave their `set_differential_mode` calls and
//! their dedup-cache contents.

mod common;

use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

use common::{covers, SpecItem};
use mcp_tools::{
    current_differential_mode, flush_discrepancy_dedup, parse_value, set_differential_mode,
    set_discrepancy_dedup_capacity, DifferentialMode, Discrepancy, DiscrepancyInput,
    DiscrepancySink,
};

fn test_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
}

fn install_callback() -> Arc<Mutex<Vec<Discrepancy>>> {
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let recorded_for_cb = Arc::clone(&recorded);
    set_differential_mode(DifferentialMode::On {
        sink: DiscrepancySink::Callback(Arc::new(move |d: &Discrepancy| {
            recorded_for_cb.lock().unwrap().push(d.clone());
        })),
        verbose: false,
    });
    flush_discrepancy_dedup();
    recorded
}

#[test]
fn off_mode_skips_validation_entirely() {
    covers!([SpecItem::McpToolsParserDifferentialMode]);
    let _guard = test_lock();

    let recorded = Arc::new(Mutex::new(Vec::new()));
    let recorded_for_cb = Arc::clone(&recorded);
    set_differential_mode(DifferentialMode::On {
        sink: DiscrepancySink::Callback(Arc::new(move |d: &Discrepancy| {
            recorded_for_cb.lock().unwrap().push(d.clone());
        })),
        verbose: false,
    });
    flush_discrepancy_dedup();

    set_differential_mode(DifferentialMode::Off);
    flush_discrepancy_dedup();

    // Even an input that diverges between parsers must produce no callback when off.
    let _ = parse_value("36893488147419103232"); // bignum: new errors, lexpr accepts
    assert!(recorded.lock().unwrap().is_empty());
}

#[test]
fn on_mode_with_callback_dispatches_for_diverging_input() {
    covers!([
        SpecItem::McpToolsParserDifferentialMode,
        SpecItem::McpToolsParserDiscrepancyReporting,
    ]);
    let _guard = test_lock();

    let recorded = install_callback();

    // bignum input: new parser errors, lexpr accepts — discrepancy expected.
    let _ = parse_value("36893488147419103232");

    let captured = recorded.lock().unwrap();
    assert!(
        !captured.is_empty(),
        "expected at least one discrepancy report"
    );

    // Restore Off so other tests don't fight over the sink.
    set_differential_mode(DifferentialMode::Off);
    flush_discrepancy_dedup();
}

#[test]
fn lru_dedup_reports_each_input_once() {
    covers!([SpecItem::McpToolsParserDiscrepancyDeduplication]);
    let _guard = test_lock();

    let recorded = install_callback();
    set_discrepancy_dedup_capacity(1024);

    // Same diverging input parsed multiple times — only the first should report.
    for _ in 0..5 {
        let _ = parse_value("36893488147419103232");
    }
    let captured = recorded.lock().unwrap();
    assert_eq!(
        captured.len(),
        1,
        "dedup should report identical input only once, got {}",
        captured.len()
    );
    drop(captured);

    set_differential_mode(DifferentialMode::Off);
    flush_discrepancy_dedup();
}

#[test]
fn flush_discrepancy_dedup_allows_re_reporting() {
    covers!([SpecItem::McpToolsParserDiscrepancyDeduplication]);
    let _guard = test_lock();

    let recorded = install_callback();

    let _ = parse_value("36893488147419103232");
    let initial = recorded.lock().unwrap().len();
    drop(recorded);

    flush_discrepancy_dedup();
    let recorded = install_callback();
    let _ = parse_value("36893488147419103232");
    let after_flush = recorded.lock().unwrap().len();
    assert!(
        after_flush >= initial,
        "after flush, repeated input should report again"
    );

    set_differential_mode(DifferentialMode::Off);
    flush_discrepancy_dedup();
}

#[test]
fn verbose_mode_includes_full_input() {
    covers!([SpecItem::McpToolsParserDiscrepancyReporting]);
    let _guard = test_lock();

    let recorded = Arc::new(Mutex::new(Vec::new()));
    let recorded_for_cb = Arc::clone(&recorded);
    set_differential_mode(DifferentialMode::On {
        sink: DiscrepancySink::Callback(Arc::new(move |d: &Discrepancy| {
            recorded_for_cb.lock().unwrap().push(d.clone());
        })),
        verbose: true,
    });
    flush_discrepancy_dedup();

    let _ = parse_value("36893488147419103232");
    let captured = recorded.lock().unwrap();
    assert_eq!(captured.len(), 1);
    match &captured[0].input {
        DiscrepancyInput::Verbose { source } => {
            assert_eq!(source, "36893488147419103232");
        }
        DiscrepancyInput::Hashed { .. } => {
            panic!("verbose mode should produce DiscrepancyInput::Verbose")
        }
    }
    drop(captured);

    set_differential_mode(DifferentialMode::Off);
    flush_discrepancy_dedup();
}

#[test]
fn hashed_mode_omits_input_string() {
    covers!([SpecItem::McpToolsParserDiscrepancyReporting]);
    let _guard = test_lock();

    let recorded = install_callback();
    let _ = parse_value("36893488147419103232");
    let captured = recorded.lock().unwrap();
    assert_eq!(captured.len(), 1);
    match &captured[0].input {
        DiscrepancyInput::Hashed { sha256 } => {
            // SHA-256 is fixed-length; any 32-byte hash is acceptable.
            assert_eq!(sha256.len(), 32);
        }
        DiscrepancyInput::Verbose { .. } => {
            panic!("hashed mode should not produce DiscrepancyInput::Verbose")
        }
    }
    drop(captured);

    set_differential_mode(DifferentialMode::Off);
    flush_discrepancy_dedup();
}

#[test]
fn mode_is_settable_and_readable() {
    covers!([SpecItem::McpToolsParserDifferentialMode]);
    let _guard = test_lock();

    set_differential_mode(DifferentialMode::Off);
    matches!(current_differential_mode(), DifferentialMode::Off);

    set_differential_mode(DifferentialMode::On {
        sink: DiscrepancySink::Stderr,
        verbose: true,
    });
    matches!(
        current_differential_mode(),
        DifferentialMode::On { verbose: true, .. }
    );

    set_differential_mode(DifferentialMode::Off);
}

#[test]
fn reporting_is_non_fatal_caller_always_receives_new_parser_result() {
    covers!([SpecItem::McpToolsParserDiscrepancyReporting]);
    let _guard = test_lock();

    install_callback();

    // Diverging input: caller still gets the new parser's outcome (an Err for
    // the bignum), unaffected by the discrepancy report.
    let result = parse_value("36893488147419103232");
    assert!(result.is_err());

    // Non-diverging input: caller still gets Ok, no discrepancy.
    let result = parse_value("(a b)");
    assert!(result.is_ok());

    set_differential_mode(DifferentialMode::Off);
    flush_discrepancy_dedup();
}
