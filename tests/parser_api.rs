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
        "(:k :v)",
        "(:k v)",
        "(head :k :v)",
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

#[test]
fn get_kw_value_consumes_the_value_slot_positionally() {
    covers!([SpecItem::McpToolsParserKwValueExtraction]);

    use mcp_tools::get_kw_value;

    // A keyword in value position is the value — not a dangling next key. If the
    // scan re-read `:pass` as a key, `:note` would land in a value slot and stop
    // resolving, so asserting the later key still resolves pins the alignment.
    let v = parse_value("(record :verdict :pass :note \"ok\")").unwrap();
    assert_eq!(
        get_kw_value(&v, "verdict").unwrap(),
        Some(Value::Keyword("pass".into()))
    );
    assert_eq!(
        get_kw_value(&v, "note").unwrap(),
        Some(Value::String("ok".into()))
    );

    // The rule is type-blind: every variant is accepted in the value slot.
    let v = parse_value("(t :a 1 :b () :c (sub :k :w) :d nil :e :z)").unwrap();
    assert_eq!(get_kw_value(&v, "a").unwrap(), Some(Value::Integer(1)));
    // `()` parses to Nil (see mcp-tools/parser/list-representation); the point
    // here is that the slot is taken verbatim, whatever the variant.
    assert_eq!(get_kw_value(&v, "b").unwrap(), Some(Value::Nil));
    assert_eq!(
        get_kw_value(&v, "c").unwrap(),
        Some(Value::List(vec![
            Value::Symbol("sub".into()),
            Value::Keyword("k".into()),
            Value::Keyword("w".into()),
        ]))
    );
    assert_eq!(get_kw_value(&v, "e").unwrap(), Some(Value::Keyword("z".into())));
}

#[test]
fn get_kw_value_skips_the_head_and_stops_at_a_non_keyword_key_slot() {
    covers!([SpecItem::McpToolsParserKwValueExtraction]);

    use mcp_tools::get_kw_value;

    // Index 0 is the head even when it is itself a keyword.
    let v = parse_value("(:head :head 1)").unwrap();
    assert_eq!(get_kw_value(&v, "head").unwrap(), Some(Value::Integer(1)));

    // The first key slot holding a non-keyword ends the scan; keys after it are
    // not reachable.
    let v = parse_value("(t :a 1 positional :b 2)").unwrap();
    assert_eq!(get_kw_value(&v, "a").unwrap(), Some(Value::Integer(1)));
    assert_eq!(get_kw_value(&v, "b").unwrap(), None);

    // A well-formed scan that never matches yields None, not an error.
    let v = parse_value("(t :a 1)").unwrap();
    assert_eq!(get_kw_value(&v, "missing").unwrap(), None);
}

#[test]
fn get_kw_value_rejects_non_lists_and_dangling_keywords() {
    covers!([SpecItem::McpToolsParserKwValueExtraction]);

    use mcp_tools::get_kw_value;

    // Non-list roots, including a dotted pair, are an error.
    for src in ["\"str\"", ":kw", "42", "(a . b)"] {
        let v = parse_value(src).unwrap();
        let err = get_kw_value(&v, "a").unwrap_err().to_string();
        assert!(
            err.contains("expected list"),
            "{}: unexpected error {}",
            src,
            err
        );
    }

    // A key slot with no following item is a malformed form, not a missing key —
    // and it is an error even when the dangling keyword is not the one requested.
    let v = parse_value("(t :a 1 :b)").unwrap();
    let err = get_kw_value(&v, "b").unwrap_err().to_string();
    assert!(err.contains("expected value after keyword :b"), "{}", err);
    let err = get_kw_value(&v, "absent").unwrap_err().to_string();
    assert!(err.contains("expected value after keyword :b"), "{}", err);
}

#[test]
fn get_kw_str_inherits_the_scanning_rules() {
    covers!([SpecItem::McpToolsParserKwValueExtraction]);

    use mcp_tools::{get_kw_str, require_kw_str};

    let v = parse_value("(record :verdict :pass :note \"ok\")").unwrap();
    assert_eq!(get_kw_str(&v, "note").unwrap(), Some("ok".to_string()));
    assert_eq!(require_kw_str(&v, "note").unwrap(), "ok");

    // The wrapper adds only the string type-check; the keyword-valued slot is
    // found first, then rejected as a non-string.
    let err = get_kw_str(&v, "verdict").unwrap_err().to_string();
    assert!(err.contains("must be a string"), "{}", err);
}

#[test]
fn get_kw_value_returns_bare_symbols_in_value_position_as_symbols() {
    covers!([SpecItem::McpToolsParserKwValueExtraction]);

    use mcp_tools::get_kw_value;

    // A bare word in value position is the value, and stays a Symbol -- it is not
    // coerced to a string and not mistaken for a key slot. As with the keyword
    // case, asserting the following key still resolves is what catches an
    // alignment shift.
    let v = parse_value("(record :verdict pass :note \"ok\")").unwrap();
    assert_eq!(
        get_kw_value(&v, "verdict").unwrap(),
        Some(Value::Symbol("pass".into()))
    );
    assert_eq!(
        get_kw_value(&v, "note").unwrap(),
        Some(Value::String("ok".into()))
    );

    // Keyword and symbol spellings of the same enumeration coexist in one form and
    // neither is normalized into the other.
    let v = parse_value("(report :status pass :verdict :pass)").unwrap();
    assert_eq!(
        get_kw_value(&v, "status").unwrap(),
        Some(Value::Symbol("pass".into()))
    );
    assert_eq!(
        get_kw_value(&v, "verdict").unwrap(),
        Some(Value::Keyword("pass".into()))
    );

    // get_kw_str does not accept a symbol as a string.
    let err = mcp_tools::get_kw_str(&v, "status").unwrap_err().to_string();
    assert!(err.contains("must be a string"), "{}", err);
}
