//! Integration tests for `iter_list` and `parse_str_list` (specs/parser/api.md § List Iteration).

mod common;

use common::{covers, SpecItem};
use mcp_tools::{iter_list, parse_str_list, parse_value, Value};

fn v(src: &str) -> Value {
    parse_value(src).unwrap()
}

fn iter_err(src: &str) -> String {
    match iter_list(&v(src)) {
        Ok(items) => panic!("{src}: expected Err, got {} items", items.count()),
        Err(e) => format!("{e:#}"),
    }
}

#[test]
fn iter_list_yields_list_items_in_order() {
    covers!([SpecItem::McpToolsParserListIteration]);

    let items: Vec<Value> = iter_list(&v("(a b c)")).unwrap().collect();
    let names: Vec<_> = items.iter().map(|i| i.as_symbol().unwrap()).collect();
    assert_eq!(names, ["a", "b", "c"]);
}

#[test]
fn iter_list_on_empty_list_is_ok_and_empty() {
    covers!([SpecItem::McpToolsParserListIteration]);

    assert_eq!(iter_list(&v("()")).unwrap().count(), 0);
    assert_eq!(iter_list(&Value::Nil).unwrap().count(), 0);
}

#[test]
fn iter_list_redundant_dot_is_a_proper_list() {
    covers!([SpecItem::McpToolsParserListIteration]);

    assert_eq!(iter_list(&v("(a b . (c))")).unwrap().count(), 3);
}

#[test]
fn iter_list_rejects_every_scalar_variant_by_name() {
    covers!([SpecItem::McpToolsParserListIteration]);

    for (src, variant) in [
        ("\"a\"", "string"),
        ("1", "integer"),
        ("1.5", "float"),
        ("#t", "bool"),
        ("foo", "symbol"),
        (":k", "keyword"),
    ] {
        let msg = iter_err(src);
        assert!(msg.contains("expected list"), "{src}: {msg}");
        assert!(msg.contains(variant), "{src}: {msg}");
    }
}

#[test]
fn iter_list_rejects_dotted_pair() {
    covers!([SpecItem::McpToolsParserListIteration]);

    let msg = iter_err("(a . b)");
    assert!(msg.contains("expected list"), "{msg}");
    assert!(msg.contains("pair"), "{msg}");
}

#[test]
fn iter_list_rejects_improper_list_without_truncating() {
    covers!([SpecItem::McpToolsParserListIteration]);

    let msg = iter_err("(a b . c)");
    assert!(msg.contains("expected list"), "{msg}");
}

#[test]
fn parse_str_list_reads_string_list() {
    covers!([SpecItem::McpToolsParserListIteration]);

    assert_eq!(parse_str_list(&v("(\"a\" \"b\")")).unwrap(), ["a", "b"]);
    assert!(parse_str_list(&v("()")).unwrap().is_empty());
}

#[test]
fn parse_str_list_rejects_scalar() {
    covers!([SpecItem::McpToolsParserListIteration]);

    // mailboxer's report: a scalar used to read as an empty list.
    let err = parse_str_list(&v("\"a\"")).unwrap_err();
    assert!(format!("{err:#}").contains("expected list"), "{err:#}");
}

#[test]
fn parse_str_list_rejects_dotted_pair() {
    covers!([SpecItem::McpToolsParserListIteration]);

    let err = parse_str_list(&v("(\"a\" . \"b\")")).unwrap_err();
    assert!(format!("{err:#}").contains("expected list"), "{err:#}");
}

#[test]
fn parse_str_list_rejects_non_string_item() {
    covers!([SpecItem::McpToolsParserListIteration]);

    let err = parse_str_list(&v("(\"a\" 1)")).unwrap_err();
    assert!(
        format!("{err:#}").contains("expected string item in list"),
        "{err:#}"
    );
}
