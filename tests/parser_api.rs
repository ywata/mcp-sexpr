//! Integration tests for the public parser API surface.

mod common;

use common::{covers, SpecItem};
#[allow(deprecated)]
use mcp_tools::{
    parse_value, parse_value_lexpr, parse_value_with_positions, ParseError, Spanned, Value,
};

#[test]
fn parse_value_returns_value() {
    covers!([SpecItem::McpToolsParserParseValue]);

    let v = parse_value("(tool :name \"x\")").unwrap();
    let items = v.as_list().expect("list");
    assert_eq!(items[0].as_symbol(), Some("tool"));
    assert_eq!(items[1], Value::Keyword("name".into()));
    assert_eq!(items[2], Value::String("x".into()));
}

#[test]
fn parse_value_with_positions_returns_spanned() {
    covers!([SpecItem::McpToolsParserParseValueWithPositions]);

    let s: Spanned = parse_value_with_positions("(tool :name \"x\")").unwrap();
    assert_eq!(s.span.start.line, 1);
    assert_eq!(s.span.start.column, 1);
    // into_value yields the same shape parse_value would produce.
    let v = s.into_value();
    assert!(v.is_list());
}

#[test]
fn parse_value_with_positions_into_value_matches_parse_value() {
    covers!([
        SpecItem::McpToolsParserParseValue,
        SpecItem::McpToolsParserParseValueWithPositions,
    ]);

    let inputs = [
        "()",
        "nil",
        "42",
        "(a b c)",
        "(1 . 2)",
        "(:k \"v\")",
        "'expr",
    ];
    for input in inputs {
        let plain = parse_value(input).unwrap();
        let spanned = parse_value_with_positions(input).unwrap().into_value();
        assert_eq!(plain, spanned, "mismatch for {:?}", input);
    }
}

#[test]
#[allow(deprecated)]
fn parse_value_lexpr_keeps_old_behavior() {
    covers!([SpecItem::McpToolsParserApiDeprecation]);

    // The deprecated function still exists and returns lexpr::Value.
    let v: lexpr::Value = parse_value_lexpr("(tool :name \"x\")").unwrap();
    assert!(v.is_cons());
}

#[test]
fn parse_errors_round_trip_through_anyhow_chain() {
    covers!([SpecItem::McpToolsParserParseValue]);

    let cases: &[(&str, fn(&ParseError) -> bool)] = &[
        ("(", |e| matches!(e, ParseError::UnclosedList { .. })),
        (")", |e| matches!(e, ParseError::UnmatchedRParen { .. })),
        ("\"unterminated", |e| matches!(e, ParseError::UnterminatedString { .. })),
        ("#| unclosed", |e| matches!(e, ParseError::UnterminatedBlockComment { .. })),
        (r#""\x""#, |e| matches!(e, ParseError::InvalidEscape { .. })),
        ("1 2", |e| matches!(e, ParseError::TrailingInput { .. })),
        ("(. x)", |e| matches!(e, ParseError::DotWithoutHead { .. })),
        ("(x .)", |e| matches!(e, ParseError::DotWithoutTail { .. })),
        ("(x . y z)", |e| matches!(e, ParseError::DotWithMultipleTail { .. })),
        ("'", |e| matches!(e, ParseError::QuoteWithoutValue { .. })),
        ("", |e| matches!(e, ParseError::UnexpectedEof)),
    ];

    for (input, predicate) in cases {
        let err = parse_value(input).unwrap_err();
        let downcast = err
            .downcast_ref::<ParseError>()
            .unwrap_or_else(|| panic!("error chain missing ParseError for {:?}", input));
        assert!(
            predicate(downcast),
            "wrong ParseError variant for {:?}: {:?}",
            input,
            downcast
        );
    }
}

#[test]
#[allow(deprecated)]
fn deprecated_lexpr_helpers_still_callable() {
    covers!([SpecItem::McpToolsParserApiDeprecation]);

    use mcp_tools::{
        get_kw_str_lexpr, get_kw_value_lexpr, iter_list_lexpr, parse_str_list_lexpr,
        parse_text_ref_lexpr, require_kw_str_lexpr, TextRef,
    };

    let v = parse_value_lexpr("(tool :name \"abc\" :items (\"a\" \"b\"))").unwrap();

    let kv = get_kw_value_lexpr(&v, "name").unwrap();
    assert!(kv.is_some());

    assert_eq!(
        get_kw_str_lexpr(&v, "name").unwrap(),
        Some("abc".to_string())
    );
    assert_eq!(require_kw_str_lexpr(&v, "name").unwrap(), "abc");

    let xs: Vec<lexpr::Value> = iter_list_lexpr(&v).unwrap().collect();
    assert!(!xs.is_empty());

    let items = get_kw_value_lexpr(&v, "items").unwrap().unwrap();
    assert_eq!(parse_str_list_lexpr(&items).unwrap(), vec!["a", "b"]);

    let lit = parse_value_lexpr("\"hi\"").unwrap();
    assert_eq!(
        parse_text_ref_lexpr(&lit).unwrap(),
        TextRef::Literal("hi".into())
    );
}
