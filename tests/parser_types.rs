//! Integration tests for the parser value types — Value construction, Spanned →
//! Value conversion, numeric tower limits, keyword canonicalization, and list
//! representation.

mod common;

use common::{covers, SpecItem};
use mcp_tools::{
    parse_value, parse_value_with_positions, ParseError, Spanned, SpannedNode, Value,
};

#[test]
fn value_constructors_and_predicates() {
    covers!([SpecItem::McpToolsParserValueType]);

    assert!(Value::Nil.is_nil());
    assert!(Value::Bool(true).is_bool());
    assert!(Value::Integer(0).is_integer());
    assert!(Value::Float(0.0).is_float());
    assert!(Value::String(String::new()).is_string());
    assert!(Value::Symbol(String::new()).is_symbol());
    assert!(Value::Keyword(String::new()).is_keyword());
    assert!(Value::List(Vec::new()).is_list());
    assert!(Value::Pair(Box::new((Value::Nil, Value::Nil))).is_pair());

    assert_eq!(Value::Bool(true).as_bool(), Some(true));
    assert_eq!(Value::Integer(7).as_i64(), Some(7));
    assert_eq!(Value::Float(2.5).as_f64(), Some(2.5));
    assert_eq!(Value::String("x".into()).as_str(), Some("x"));
    assert_eq!(Value::Symbol("y".into()).as_symbol(), Some("y"));
    assert_eq!(Value::Keyword("z".into()).as_keyword(), Some("z"));
    assert_eq!(Value::List(vec![Value::Integer(1)]).as_list().unwrap().len(), 1);
}

#[test]
fn spanned_into_value_strips_spans_and_comments() {
    covers!([SpecItem::McpToolsParserSpannedType]);

    let spanned: Spanned = parse_value_with_positions("; comment\n42").unwrap();
    assert_eq!(spanned.leading_comments.len(), 1);
    let v = spanned.into_value();
    assert_eq!(v, Value::Integer(42));
}

#[test]
fn spanned_node_recurses_into_spanned_for_lists() {
    covers!([SpecItem::McpToolsParserSpannedType]);

    let s = parse_value_with_positions("(1 2)").unwrap();
    if let SpannedNode::List(items) = &s.value {
        // Each item is a full Spanned with its own span.
        assert!(items.iter().all(|i| i.span.start.byte_offset <= i.span.end.byte_offset));
    } else {
        panic!("expected List");
    }
}

#[test]
fn numeric_tower_accepts_i64_boundaries() {
    covers!([SpecItem::McpToolsParserNumericTower]);

    let max_str = i64::MAX.to_string();
    let min_str = i64::MIN.to_string();

    assert_eq!(parse_value(&max_str).unwrap(), Value::Integer(i64::MAX));
    assert_eq!(parse_value(&min_str).unwrap(), Value::Integer(i64::MIN));
}

#[test]
fn numeric_tower_rejects_out_of_range_integers() {
    covers!([SpecItem::McpToolsParserNumericTower]);

    // Just past i64::MAX
    let too_big = (i64::MAX as u128 + 1).to_string();
    let err = parse_value(&too_big).unwrap_err();
    let downcast = err.downcast_ref::<ParseError>().expect("ParseError");
    match downcast {
        ParseError::IntegerOutOfRange { .. } => {}
        other => panic!("expected IntegerOutOfRange, got {:?}", other),
    }
}

#[test]
fn numeric_tower_has_no_rational_or_complex_syntax() {
    covers!([SpecItem::McpToolsParserNumericTower]);

    // "1/2" is not a single number — the grammar has no rational syntax. The
    // lexer produces Integer(1) followed by Symbol("/2"); inside a list both
    // tokens become separate values.
    let v = parse_value("(1/2)").unwrap();
    let items = v.as_list().unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0], Value::Integer(1));
    assert_eq!(items[1], Value::Symbol("/2".into()));
}

#[test]
fn keyword_stored_without_leading_colon() {
    covers!([SpecItem::McpToolsParserKeywordCanonicalization]);

    let v = parse_value(":name").unwrap();
    assert_eq!(v, Value::Keyword("name".into()));
    assert_eq!(v.as_keyword(), Some("name"));

    let v = parse_value("(:foo 1 :bar 2)").unwrap();
    let items = v.as_list().unwrap();
    assert_eq!(items[0], Value::Keyword("foo".into()));
    assert_eq!(items[2], Value::Keyword("bar".into()));
}

#[test]
fn list_representation_uses_vec_for_proper_lists() {
    covers!([SpecItem::McpToolsParserListRepresentation]);

    let v = parse_value("(0 1 2 3 4 5 6 7 8 9)").unwrap();
    let items = v.as_list().expect("proper list");
    for (i, item) in items.iter().enumerate() {
        assert_eq!(item.as_i64(), Some(i as i64));
    }
}

#[test]
fn pair_only_for_dotted_form() {
    covers!([SpecItem::McpToolsParserListRepresentation]);

    // Proper list parses to List, not Pair-chain.
    let v = parse_value("(1 2 3)").unwrap();
    assert!(v.is_list());
    assert!(!v.is_pair());

    // Dotted form parses to Pair.
    let v = parse_value("(1 . 2)").unwrap();
    assert!(v.is_pair());
    assert!(!v.is_list());
}
