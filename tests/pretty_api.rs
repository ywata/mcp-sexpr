//! Integration tests for `mcp_tools::pretty`.
//!
//! Spec: `specs/pretty/api.md`. Gated behind the `format-pretty` Cargo feature;
//! when the feature is disabled, this file is empty and contributes no tests
//! and no coverage.

#![cfg(feature = "format-pretty")]

mod common;

use common::{covers, SpecItem};
use mcp_tools::parse_value;
use mcp_tools::pretty::{pretty_print, pretty_print_default, pretty_print_top_forms, PrettyOpts};
use mcp_tools::Value;

fn opts(width: usize) -> PrettyOpts {
    PrettyOpts { max_line_width: width, ..PrettyOpts::default() }
}

#[test]
fn options_defaults_match_spec() {
    covers!([SpecItem::McpToolsPrettyOptions]);
    let d = PrettyOpts::default();
    assert_eq!(d.max_line_width, 80);
    assert_eq!(d.indent, 2);
    assert!(d.align_keyword_args);
    assert!(d.blank_line_between_top_forms);
}

#[test]
fn options_record_update() {
    covers!([SpecItem::McpToolsPrettyOptions]);
    let o = PrettyOpts { max_line_width: 40, indent: 4, ..PrettyOpts::default() };
    assert_eq!(o.max_line_width, 40);
    assert_eq!(o.indent, 4);
    assert!(o.align_keyword_args); // inherited
}

#[test]
fn options_zero_width_no_panic() {
    covers!([SpecItem::McpToolsPrettyOptions]);
    let v = parse_value("(a b c)").unwrap();
    let _ = pretty_print(&v, &opts(0));
}

#[test]
fn pretty_print_default_matches_default_opts() {
    covers!([SpecItem::McpToolsPrettyPrettyPrint]);
    let v = parse_value("(a b c)").unwrap();
    assert_eq!(pretty_print_default(&v), pretty_print(&v, &PrettyOpts::default()));
}

#[test]
fn pretty_print_no_leading_or_trailing_newline() {
    covers!([SpecItem::McpToolsPrettyPrettyPrint]);
    let v = parse_value("(a (b c) (d e))").unwrap();
    for w in [5usize, 80] {
        let s = pretty_print(&v, &opts(w));
        assert!(!s.starts_with('\n'), "leading newline: {:?}", s);
        assert!(!s.ends_with('\n'), "trailing newline: {:?}", s);
    }
}

#[test]
fn layout_atom_and_short_list_emit_single_line() {
    covers!([SpecItem::McpToolsPrettyLayoutRules]);
    assert_eq!(pretty_print_default(&parse_value("42").unwrap()), "42");
    assert_eq!(pretty_print_default(&parse_value("foo").unwrap()), "foo");
    assert_eq!(pretty_print_default(&parse_value("(a b c)").unwrap()), "(a b c)");
    assert_eq!(pretty_print(&Value::Nil, &opts(0)), "()");
}

#[test]
fn layout_long_list_wraps() {
    covers!([SpecItem::McpToolsPrettyLayoutRules]);
    let v = parse_value("(define x (lambda (y) (+ y 1)))").unwrap();
    let s = pretty_print(&v, &opts(15));
    assert!(s.contains('\n'));
    let r = parse_value(&s).unwrap();
    assert_eq!(r, v);
}

#[test]
fn layout_pair_wraps_with_dot_on_new_line() {
    covers!([SpecItem::McpToolsPrettyLayoutRules]);
    let v = parse_value("(aaaa . bbbb)").unwrap();
    let s = pretty_print(&v, &opts(5));
    assert!(s.contains('\n'));
    assert!(s.contains(". "));
    let r = parse_value(&s).unwrap();
    assert_eq!(r, v);
}

#[test]
fn layout_head_and_first_atom_share_line() {
    covers!([SpecItem::McpToolsPrettyLayoutRules]);
    let v = parse_value("(define x (lambda () 1))").unwrap();
    let s = pretty_print(&v, &opts(15));
    assert_eq!(s.lines().next().unwrap(), "(define x");
}

#[test]
fn keyword_alignment_pads_to_max() {
    covers!([SpecItem::McpToolsPrettyKeywordAlignment]);
    let v = parse_value(r#"(define-tool foo :pred (use "p") :body (use "b") :max 3)"#).unwrap();
    let s = pretty_print(&v, &opts(40));
    let lines: Vec<&str> = s.lines().collect();
    assert_eq!(lines[0], "(define-tool foo");
    assert_eq!(lines[1], r#"  :pred (use "p")"#);
    assert_eq!(lines[2], r#"  :body (use "b")"#);
    assert_eq!(lines[3], "  :max  3)");
}

#[test]
fn keyword_alignment_disabled_keeps_kw_value_pair() {
    covers!([SpecItem::McpToolsPrettyKeywordAlignment]);
    let v = parse_value(r#"(f :a 1 :bb 2)"#).unwrap();
    let s = pretty_print(
        &v,
        &PrettyOpts { max_line_width: 5, align_keyword_args: false, ..PrettyOpts::default() },
    );
    assert!(s.contains("  :a 1"), "got: {:?}", s);
    assert!(s.contains("  :bb 2"), "got: {:?}", s);
    // No extra padding on the shorter keyword.
    assert!(!s.contains(":a  1"));
}

#[test]
fn keyword_block_starts_when_first_rest_is_kw() {
    covers!([SpecItem::McpToolsPrettyKeywordAlignment]);
    let v = parse_value("(config :a 1 :b 2)").unwrap();
    let s = pretty_print(&v, &opts(10));
    let first = s.lines().next().unwrap();
    // Head must be alone — first arg is a keyword, not a positional, so don't share line.
    assert_eq!(first, "(config");
}

#[test]
fn top_forms_blank_line_default() {
    covers!([SpecItem::McpToolsPrettyBlankLineBetweenTopForms]);
    let a = parse_value("(define x 1)").unwrap();
    let b = parse_value("(define y 2)").unwrap();
    let s = pretty_print_top_forms(&[a, b], &PrettyOpts::default());
    assert_eq!(s, "(define x 1)\n\n(define y 2)");
}

#[test]
fn top_forms_single_newline_when_disabled() {
    covers!([SpecItem::McpToolsPrettyBlankLineBetweenTopForms]);
    let a = parse_value("(define x 1)").unwrap();
    let b = parse_value("(define y 2)").unwrap();
    let opts = PrettyOpts { blank_line_between_top_forms: false, ..PrettyOpts::default() };
    let s = pretty_print_top_forms(&[a, b], &opts);
    assert_eq!(s, "(define x 1)\n(define y 2)");
}

#[test]
fn top_forms_empty_returns_empty() {
    covers!([SpecItem::McpToolsPrettyBlankLineBetweenTopForms]);
    assert_eq!(pretty_print_top_forms(&[], &PrettyOpts::default()), "");
}

#[test]
fn determinism_byte_identical_across_runs() {
    covers!([SpecItem::McpToolsPrettyDeterminism]);
    let v = parse_value(r#"(tool :a 1 :b 2 :c (nested 1 2 3) :d "long")"#).unwrap();
    let opts = opts(30);
    let s1 = pretty_print(&v, &opts);
    for _ in 0..50 {
        assert_eq!(pretty_print(&v, &opts), s1);
    }
}

#[test]
fn round_trip_atoms_and_lists() {
    covers!([SpecItem::McpToolsPrettyRoundTrip]);
    let inputs = [
        "42",
        "#t",
        "()",
        r#""hello""#,
        "(a b c)",
        "(define foo (lambda (x) (+ x 1)))",
        r#"(tool :a 1 :b (nested :c 2 :d 3))"#,
        "((a b) (c d) (e f))",
        "(a . b)",
    ];
    for input in &inputs {
        let v = parse_value(input).unwrap();
        for w in [10usize, 20, 40, 80, 200] {
            let s = pretty_print(&v, &opts(w));
            let r = parse_value(&s).unwrap_or_else(|e| {
                panic!("re-parse failed: {} for {} at width {}\noutput:\n{}", e, input, w, s)
            });
            assert_eq!(r, v, "round-trip failed for {} at width {}", input, w);
        }
    }
}

#[test]
fn round_trip_strings_with_escapes() {
    covers!([SpecItem::McpToolsPrettyRoundTrip]);
    let v = Value::String(r#"a "b" \c"#.to_string());
    let s = pretty_print_default(&v);
    let r = parse_value(&s).unwrap();
    assert_eq!(r, v);
}

#[test]
fn feature_gate_exposes_public_api() {
    covers!([SpecItem::McpToolsPrettyFeatureGate]);
    // Compiles only when `format-pretty` is enabled.
    let _: fn(&Value, &PrettyOpts) -> String = pretty_print;
    let _: fn(&Value) -> String = pretty_print_default;
    let _: fn(&[Value], &PrettyOpts) -> String = pretty_print_top_forms;
}
