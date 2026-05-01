//! Integration tests for the AST builder functions (`mcp_tools::build`).
//!
//! Spec: `specs/build/api.md`. The src/ unit tests cover finer-grained behavior
//! using a compile-time `covers!` macro; this file uses the runtime macro so
//! coverage is recorded in `coverage.db`.

mod common;

use common::{covers, SpecItem};
use mcp_tools::build::{cons, integer, keyword, list, string, symbol};
use mcp_tools::{parse_value, Value};

#[test]
fn cons_returns_pair() {
    covers!([SpecItem::McpToolsBuildCons]);
    let v = cons(integer(1), integer(2));
    assert!(matches!(v, Value::Pair(_)));
    assert!(v.is_pair());
}

#[test]
fn cons_does_not_collapse_into_list() {
    covers!([SpecItem::McpToolsBuildCons]);
    let nested = cons(symbol("a"), cons(symbol("b"), Value::Nil));
    let proper = list(vec![symbol("a"), symbol("b")]);
    assert_ne!(nested, proper);
}

#[test]
fn list_empty_yields_nil() {
    covers!([SpecItem::McpToolsBuildList]);
    assert_eq!(list(vec![]), Value::Nil);
}

#[test]
fn list_round_trip_through_parse_value() {
    covers!([
        SpecItem::McpToolsBuildList,
        SpecItem::McpToolsBuildSymbol,
        SpecItem::McpToolsBuildInteger,
    ]);
    let built = list(vec![symbol("define"), symbol("x"), integer(42)]);
    let parsed = parse_value("(define x 42)").unwrap();
    assert_eq!(built, parsed);
}

#[test]
fn keyword_stores_name_without_validating() {
    covers!([SpecItem::McpToolsBuildKeyword]);
    assert_eq!(keyword("doc"), Value::Keyword("doc".to_string()));
    // Spec note: passing ":doc" is wrong (becomes Keyword(":doc")), but
    // not rejected — pin the documented behavior.
    assert_eq!(keyword(":doc"), Value::Keyword(":doc".to_string()));
}

#[test]
fn symbol_stores_verbatim() {
    covers!([SpecItem::McpToolsBuildSymbol]);
    assert_eq!(symbol("foo-bar"), Value::Symbol("foo-bar".to_string()));
}

#[test]
fn string_stores_unescaped() {
    covers!([SpecItem::McpToolsBuildString]);
    let v = string(r#"with "quote" and \backslash"#);
    assert_eq!(v.as_str(), Some(r#"with "quote" and \backslash"#));
}

#[test]
fn integer_handles_extremes() {
    covers!([SpecItem::McpToolsBuildInteger]);
    assert_eq!(integer(i64::MAX).as_i64(), Some(i64::MAX));
    assert_eq!(integer(i64::MIN).as_i64(), Some(i64::MIN));
    assert_eq!(integer(0).as_i64(), Some(0));
}
