//! Integration tests for the bidirectional `Value` ↔ `lexpr::Value` conversion.

mod common;

use common::{covers, SpecItem};
use mcp_tools::{LexprConversionError, Value};

#[test]
fn value_to_lexpr_then_back_round_trips_for_atoms() {
    covers!([SpecItem::McpToolsParserLexprConversion]);

    let cases: Vec<Value> = vec![
        Value::Nil,
        Value::Bool(true),
        Value::Bool(false),
        Value::Integer(0),
        Value::Integer(i64::MAX),
        Value::Integer(i64::MIN),
        Value::Float(0.0),
        Value::Float(-1.5),
        Value::String("with \"quote\" and \\".into()),
        Value::Symbol("foo-bar".into()),
        Value::Keyword("name".into()),
    ];
    for v in cases {
        let lexpr_v: lexpr::Value = lexpr::Value::from(v.clone());
        let back = Value::try_from(lexpr_v).unwrap();
        assert_eq!(back, v);
    }
}

#[test]
fn value_to_lexpr_then_back_round_trips_for_lists() {
    covers!([SpecItem::McpToolsParserLexprConversion]);

    let nested = Value::List(vec![
        Value::Symbol("tool".into()),
        Value::Keyword("name".into()),
        Value::String("hi".into()),
        Value::List(vec![Value::Integer(1), Value::Integer(2)]),
    ]);
    let lexpr_v = lexpr::Value::from(nested.clone());
    let back = Value::try_from(lexpr_v).unwrap();
    assert_eq!(back, nested);

    let pair = Value::Pair(Box::new((Value::Integer(1), Value::Integer(2))));
    let lexpr_v = lexpr::Value::from(pair.clone());
    let back = Value::try_from(lexpr_v).unwrap();
    assert_eq!(back, pair);
}

#[test]
fn lexpr_bignum_to_value_errors_with_bignum_out_of_range() {
    covers!([SpecItem::McpToolsMigrationNumericTowerLoss]);

    // PosInt strictly greater than i64::MAX in lexpr's Number representation.
    let n = lexpr::Number::from(i64::MAX as u64 + 1);
    let lexpr_v = lexpr::Value::Number(n);
    match Value::try_from(lexpr_v) {
        Err(LexprConversionError::BignumOutOfRange) => {}
        other => panic!("expected BignumOutOfRange, got {:?}", other),
    }
}

#[test]
fn lexpr_char_to_value_errors() {
    covers!([SpecItem::McpToolsMigrationLexprConversionLossy]);

    let lexpr_v = lexpr::Value::Char('!');
    match Value::try_from(lexpr_v) {
        Err(LexprConversionError::UnsupportedChar('!')) => {}
        other => panic!("expected UnsupportedChar('!'), got {:?}", other),
    }
}

#[test]
fn lexpr_bytes_to_value_errors() {
    covers!([SpecItem::McpToolsMigrationLexprConversionLossy]);

    let lexpr_v = lexpr::Value::bytes(vec![0xDE, 0xAD]);
    match Value::try_from(lexpr_v) {
        Err(LexprConversionError::UnsupportedBytes) => {}
        other => panic!("expected UnsupportedBytes, got {:?}", other),
    }
}

#[test]
fn lexpr_vector_to_value_errors() {
    covers!([SpecItem::McpToolsMigrationLexprConversionLossy]);

    let lexpr_v = lexpr::Value::Vector(
        vec![lexpr::Value::Number(1i64.into()), lexpr::Value::Number(2i64.into())]
            .into_boxed_slice(),
    );
    match Value::try_from(lexpr_v) {
        Err(LexprConversionError::UnsupportedVector) => {}
        other => panic!("expected UnsupportedVector, got {:?}", other),
    }
}

#[test]
fn keyword_and_symbol_disambiguated_through_round_trip() {
    covers!([
        SpecItem::McpToolsParserLexprConversion,
        SpecItem::McpToolsParserKeywordCanonicalization,
    ]);

    // Value -> lexpr::Value emits a keyword; round-trip back recovers it.
    let kw = Value::Keyword("foo".into());
    let lexpr_v = lexpr::Value::from(kw.clone());
    assert!(lexpr_v.is_keyword());
    let back = Value::try_from(lexpr_v).unwrap();
    assert_eq!(back, kw);

    // Symbol-with-leading-colon-by-string-construction is a Symbol, not a Keyword.
    let lexpr_sym = lexpr::Value::Symbol(":bar".into());
    let back = Value::try_from(lexpr_sym).unwrap();
    assert_eq!(back, Value::Symbol(":bar".into()));
}

#[test]
fn lexpr_proper_list_via_cons_cells_becomes_list() {
    covers!([SpecItem::McpToolsParserLexprConversion]);

    let lexpr_v = lexpr::from_str("(1 2 3)").unwrap();
    let v = Value::try_from(lexpr_v).unwrap();
    assert_eq!(
        v,
        Value::List(vec![
            Value::Integer(1),
            Value::Integer(2),
            Value::Integer(3),
        ])
    );
}

#[test]
fn lexpr_dotted_pair_becomes_pair() {
    covers!([SpecItem::McpToolsParserLexprConversion]);

    let lexpr_v = lexpr::from_str("(1 . 2)").unwrap();
    let v = Value::try_from(lexpr_v).unwrap();
    assert_eq!(
        v,
        Value::Pair(Box::new((Value::Integer(1), Value::Integer(2))))
    );
}

#[test]
fn lexpr_improper_list_becomes_pair_chain() {
    covers!([SpecItem::McpToolsParserLexprConversion]);

    let lexpr_v = lexpr::from_str("(1 2 . 3)").unwrap();
    let v = Value::try_from(lexpr_v).unwrap();
    assert_eq!(
        v,
        Value::Pair(Box::new((
            Value::Integer(1),
            Value::Pair(Box::new((Value::Integer(2), Value::Integer(3))))
        )))
    );
}
