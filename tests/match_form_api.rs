//! Integration tests for `mcp_tools::match_form`.
//!
//! Spec: `specs/match-form/api.md`.

mod common;

use common::{covers, SpecItem};
use mcp_tools::{match_form, parse_value};

#[test]
fn match_form_basic_shape() {
    covers!([SpecItem::McpToolsMatchFormMatchForm]);
    let v = parse_value("(define x 42)").unwrap();
    let m = match_form(&v, "define").unwrap();
    assert_eq!(m.head(), "define");
    assert_eq!(m.positional().len(), 2);
}

#[test]
fn match_form_no_args() {
    covers!([SpecItem::McpToolsMatchFormMatchForm]);
    let v = parse_value("(reset)").unwrap();
    let m = match_form(&v, "reset").unwrap();
    assert_eq!(m.positional().len(), 0);
}

#[test]
fn form_match_is_borrowed() {
    covers!([
        SpecItem::McpToolsMatchFormFormMatchType,
        SpecItem::McpToolsMatchFormLifetime,
    ]);
    let v = parse_value("(f 1 :k 2)").unwrap();
    let m = match_form(&v, "f").unwrap();
    let pos = m.positional_at(0).unwrap();
    let kw = m.require_keyword("k").unwrap();
    drop(m);
    // References still valid — bound by `v`.
    assert_eq!(pos.as_i64(), Some(1));
    assert_eq!(kw.as_i64(), Some(2));
}

#[test]
fn head_accessor() {
    covers!([SpecItem::McpToolsMatchFormHead]);
    let v = parse_value("(my-tool)").unwrap();
    let m = match_form(&v, "my-tool").unwrap();
    assert_eq!(m.head(), "my-tool");
}

#[test]
fn positional_accessor_and_index_error() {
    covers!([SpecItem::McpToolsMatchFormPositional]);
    let v = parse_value("(f 1 2)").unwrap();
    let m = match_form(&v, "f").unwrap();
    assert_eq!(m.positional().len(), 2);
    assert_eq!(m.positional_at(0).unwrap().as_i64(), Some(1));
    assert_eq!(m.positional_at(1).unwrap().as_i64(), Some(2));
    let err = m.positional_at(2).unwrap_err().to_string();
    assert!(err.contains("f:"));
    assert!(err.contains("index 2"));
}

#[test]
fn keyword_accessors_present_and_missing() {
    covers!([SpecItem::McpToolsMatchFormKeyword]);
    let v = parse_value(r#"(f :name "x" :count 3)"#).unwrap();
    let m = match_form(&v, "f").unwrap();
    assert_eq!(m.keyword("name").unwrap().as_str(), Some("x"));
    assert_eq!(m.keyword("count").unwrap().as_i64(), Some(3));
    assert!(m.keyword("missing").is_none());
    assert!(m.require_keyword("name").is_ok());
    let err = m.require_keyword("missing").unwrap_err().to_string();
    assert!(err.contains("f:"));
    assert!(err.contains(":missing"));
}

#[test]
fn keyword_first_match_wins_for_duplicates() {
    covers!([SpecItem::McpToolsMatchFormKeyword]);
    let v = parse_value(r#"(f :k "first" :k "second")"#).unwrap();
    let m = match_form(&v, "f").unwrap();
    assert_eq!(m.keyword("k").unwrap().as_str(), Some("first"));
}

#[test]
fn keyword_lookup_strips_no_colon() {
    covers!([SpecItem::McpToolsMatchFormKeyword]);
    let v = parse_value("(f :k 1)").unwrap();
    let m = match_form(&v, "f").unwrap();
    // Keyword is stored as Keyword("k"); lookup with leading colon must miss.
    assert!(m.keyword(":k").is_none());
    assert!(m.keyword("k").is_some());
}

#[test]
fn error_input_not_list() {
    covers!([SpecItem::McpToolsMatchFormErrorCases]);
    let v = parse_value("42").unwrap();
    let err = match_form(&v, "f").unwrap_err().to_string();
    assert!(err.contains("expected list"));
    assert!(err.contains("integer"));
}

#[test]
fn error_input_pair_not_list() {
    covers!([SpecItem::McpToolsMatchFormErrorCases]);
    let v = parse_value("(a . b)").unwrap();
    let err = match_form(&v, "f").unwrap_err().to_string();
    assert!(err.contains("pair"));
}

#[test]
fn error_input_empty_list() {
    covers!([SpecItem::McpToolsMatchFormErrorCases]);
    let v = parse_value("()").unwrap();
    let err = match_form(&v, "f").unwrap_err().to_string();
    assert!(err.contains("(f"), "got: {}", err);
}

#[test]
fn error_head_not_symbol() {
    covers!([SpecItem::McpToolsMatchFormErrorCases]);
    let v = parse_value("(42 1)").unwrap();
    let err = match_form(&v, "f").unwrap_err().to_string();
    assert!(err.contains("symbol head"));
}

#[test]
fn error_head_mismatch() {
    covers!([SpecItem::McpToolsMatchFormErrorCases]);
    let v = parse_value("(actual 1)").unwrap();
    let err = match_form(&v, "expected").unwrap_err().to_string();
    assert!(err.contains("'expected'"));
    assert!(err.contains("'actual'"));
}

#[test]
fn error_dangling_keyword() {
    covers!([SpecItem::McpToolsMatchFormErrorCases]);
    let v = parse_value("(f :k)").unwrap();
    let err = match_form(&v, "f").unwrap_err().to_string();
    assert!(err.contains(":k"));
    assert!(err.contains("no value"));
}

#[test]
fn error_positional_after_keyword() {
    covers!([SpecItem::McpToolsMatchFormErrorCases]);
    let v = parse_value("(f :a 1 oops)").unwrap();
    let err = match_form(&v, "f").unwrap_err().to_string();
    assert!(err.contains("positional"));
    assert!(err.contains(":a"));
}
