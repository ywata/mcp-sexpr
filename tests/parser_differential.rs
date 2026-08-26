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
    set_discrepancy_class_dedup_capacity, set_discrepancy_dedup_capacity, DifferentialMode,
    Discrepancy, DiscrepancyInput, DiscrepancySink,
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

// ---------------------------------------------------------------------------
// Expected discrepancy classes (mcp-tools/parser/expected-discrepancy-classes)
// ---------------------------------------------------------------------------

#[test]
fn keyword_argument_is_an_expected_class_not_a_discrepancy() {
    covers!([SpecItem::McpToolsParserExpectedDiscrepancyClasses]);
    let _guard = test_lock();

    let recorded = install_callback();
    let result = parse_value("(tool :key \"v\")");
    assert!(result.is_ok());
    assert!(
        recorded.lock().unwrap().is_empty(),
        "Keyword vs lexpr Symbol(\":key\") must be suppressed as an expected class"
    );

    set_differential_mode(DifferentialMode::Off);
    flush_discrepancy_dedup();
}

#[test]
fn dotted_keywords_are_an_expected_class() {
    covers!([SpecItem::McpToolsParserExpectedDiscrepancyClasses]);
    let _guard = test_lock();

    let recorded = install_callback();
    let result = parse_value("(:a . :b)");
    assert!(result.is_ok());
    assert!(recorded.lock().unwrap().is_empty());

    set_differential_mode(DifferentialMode::Off);
    flush_discrepancy_dedup();
}

#[test]
fn deeply_nested_keyword_is_an_expected_class() {
    covers!([SpecItem::McpToolsParserExpectedDiscrepancyClasses]);
    let _guard = test_lock();

    let recorded = install_callback();
    let result = parse_value("(a (b (c :deep)))");
    assert!(result.is_ok());
    assert!(recorded.lock().unwrap().is_empty());

    set_differential_mode(DifferentialMode::Off);
    flush_discrepancy_dedup();
}

// ---------------------------------------------------------------------------
// Class-keyed deduplication (mcp-tools/parser/discrepancy-class-deduplication)
// ---------------------------------------------------------------------------

/// Callback sink with both caches at known capacities (input 1024, class as
/// given) and both flushed.
fn install_callback_with_class_capacity(class_capacity: usize) -> Arc<Mutex<Vec<Discrepancy>>> {
    let recorded = install_callback();
    set_discrepancy_dedup_capacity(1024);
    set_discrepancy_class_dedup_capacity(class_capacity);
    flush_discrepancy_dedup();
    recorded
}

fn restore_defaults() {
    set_discrepancy_class_dedup_capacity(256);
    set_differential_mode(DifferentialMode::Off);
    flush_discrepancy_dedup();
}

/// A distinct bignum per `i`: new parser errors (IntegerOutOfRange), lexpr
/// accepts. One class for every `i`.
fn bignum(i: usize) -> String {
    format!("3689348814741910323{i}")
}

#[test]
fn class_dedup_collapses_distinct_inputs_of_one_class() {
    covers!([SpecItem::McpToolsParserDiscrepancyClassDeduplication]);
    let _guard = test_lock();

    let recorded = install_callback_with_class_capacity(256);
    for i in 0..100 {
        let _ = parse_value(&bignum(i));
    }
    assert_eq!(
        recorded.lock().unwrap().len(),
        1,
        "100 distinct inputs of one class must yield exactly one report"
    );

    restore_defaults();
}

#[test]
fn class_dedup_distinguishes_divergence_position() {
    covers!([SpecItem::McpToolsParserDiscrepancyClassDeduplication]);
    let _guard = test_lock();

    let recorded = install_callback_with_class_capacity(256);
    // `nil` is Nil for the new parser and Symbol("nil") for lexpr: a both-Ok
    // divergence whose path is the list position.
    let _ = parse_value("(a nil)"); // 1.atom
    let _ = parse_value("(a b nil)"); // 2.atom

    let captured = recorded.lock().unwrap();
    let paths: Vec<String> = captured.iter().map(|d| d.path.to_string()).collect();
    assert_eq!(paths, vec!["1.atom".to_string(), "2.atom".to_string()]);
    drop(captured);

    restore_defaults();
}

#[test]
fn class_dedup_does_not_double_count_repeated_input() {
    covers!([SpecItem::McpToolsParserDiscrepancyClassDeduplication]);
    let _guard = test_lock();

    let recorded = install_callback_with_class_capacity(256);
    let _ = parse_value(&bignum(0));
    let _ = parse_value(&bignum(0));
    assert_eq!(recorded.lock().unwrap().len(), 1);

    restore_defaults();
}

#[test]
fn class_dedup_capacity_zero_disables_class_dedup() {
    covers!([SpecItem::McpToolsParserDiscrepancyClassDeduplication]);
    let _guard = test_lock();

    let recorded = install_callback_with_class_capacity(0);
    for i in 0..100 {
        let _ = parse_value(&bignum(i));
    }
    assert_eq!(
        recorded.lock().unwrap().len(),
        100,
        "with class dedup disabled, every input-cache miss is dispatched"
    );

    restore_defaults();
}

#[test]
fn class_dedup_evicts_least_recently_seen_class() {
    covers!([SpecItem::McpToolsParserDiscrepancyClassDeduplication]);
    let _guard = test_lock();

    let recorded = install_callback_with_class_capacity(2);
    // Three classes in rotation, each time with a fresh input so the input
    // cache never intervenes: A = 1.atom, B = 2.atom, C = bignum (Err/Ok).
    let _ = parse_value("(a nil)"); // A: reported (1)
    let _ = parse_value("(a b nil)"); // B: reported (2)
    let _ = parse_value(&bignum(0)); // C: reported (3); A evicted
    let _ = parse_value("(x nil)"); // A again: reported (4); B evicted
    let _ = parse_value("(x y nil)"); // B again: reported (5)
    let _ = parse_value("(z nil)"); // A: still cached -> suppressed

    let captured = recorded.lock().unwrap();
    let paths: Vec<String> = captured.iter().map(|d| d.path.to_string()).collect();
    assert_eq!(
        paths,
        vec!["1.atom", "2.atom", ".", "1.atom", "2.atom"]
            .into_iter()
            .map(String::from)
            .collect::<Vec<_>>()
    );
    drop(captured);

    restore_defaults();
}

// ---------------------------------------------------------------------------
// Stderr report budget (mcp-tools/parser/stderr-report-budget)
// ---------------------------------------------------------------------------

#[test]
fn stderr_sink_is_bounded_and_never_blocks_the_caller() {
    covers!([SpecItem::McpToolsParserStderrReportBudget]);
    let _guard = test_lock();

    assert_eq!(mcp_tools::STDERR_REPORT_BUDGET, 64);

    // Push well past the budget through the real Stderr sink (the harness
    // captures stderr). Every parse must return promptly with the new parser's
    // outcome; past the budget the sink drops before formatting.
    set_differential_mode(DifferentialMode::On {
        sink: DiscrepancySink::Stderr,
        verbose: false,
    });
    set_discrepancy_dedup_capacity(1024);
    set_discrepancy_class_dedup_capacity(0);
    flush_discrepancy_dedup();

    for i in 0..(mcp_tools::STDERR_REPORT_BUDGET + 10) {
        let outcome = parse_value(&bignum(i));
        assert!(outcome.is_err(), "caller must still see the new parser's Err");
    }

    restore_defaults();
}
