//! Integration tests for the parser grammar — atoms, lists, dotted pairs, quotes,
//! string escapes, and comments.

mod common;

use common::{covers, SpecItem};
use mcp_tools::{parse_value, ParseError, Value};

#[test]
fn parses_all_atom_kinds() {
    covers!([SpecItem::McpToolsParserGrammar]);

    assert_eq!(parse_value("nil").unwrap(), Value::Nil);
    assert_eq!(parse_value("()").unwrap(), Value::Nil);
    assert_eq!(parse_value("#t").unwrap(), Value::Bool(true));
    assert_eq!(parse_value("#f").unwrap(), Value::Bool(false));
    assert_eq!(parse_value("0").unwrap(), Value::Integer(0));
    assert_eq!(parse_value("-42").unwrap(), Value::Integer(-42));
    assert_eq!(parse_value("3.14").unwrap(), Value::Float(3.14));
    assert_eq!(
        parse_value("\"hello\"").unwrap(),
        Value::String("hello".into())
    );
    assert_eq!(parse_value("foo").unwrap(), Value::Symbol("foo".into()));
    assert_eq!(parse_value(":bar").unwrap(), Value::Keyword("bar".into()));
}

#[test]
fn parses_proper_lists() {
    covers!([SpecItem::McpToolsParserGrammar]);

    let v = parse_value("(1 2 3)").unwrap();
    assert_eq!(
        v,
        Value::List(vec![
            Value::Integer(1),
            Value::Integer(2),
            Value::Integer(3),
        ])
    );

    let nested = parse_value("(a (b c) d)").unwrap();
    let items = nested.as_list().unwrap();
    assert_eq!(items.len(), 3);
    assert!(items[1].is_list());
}

#[test]
fn parses_dotted_pairs() {
    covers!([SpecItem::McpToolsParserGrammar]);

    let v = parse_value("(1 . 2)").unwrap();
    assert!(v.is_pair());
    let (car, cdr) = v.as_pair().unwrap();
    assert_eq!(car.as_i64(), Some(1));
    assert_eq!(cdr.as_i64(), Some(2));
}

#[test]
fn parses_all_quote_forms() {
    covers!([SpecItem::McpToolsParserGrammar]);

    assert_eq!(
        parse_value("'x").unwrap(),
        Value::List(vec![
            Value::Symbol("quote".into()),
            Value::Symbol("x".into()),
        ])
    );
    assert_eq!(
        parse_value("`x").unwrap(),
        Value::List(vec![
            Value::Symbol("quasiquote".into()),
            Value::Symbol("x".into()),
        ])
    );
    assert_eq!(
        parse_value(",x").unwrap(),
        Value::List(vec![
            Value::Symbol("unquote".into()),
            Value::Symbol("x".into()),
        ])
    );
    assert_eq!(
        parse_value(",@x").unwrap(),
        Value::List(vec![
            Value::Symbol("unquote-splicing".into()),
            Value::Symbol("x".into()),
        ])
    );
}

#[test]
fn accepts_all_five_string_escapes() {
    covers!([SpecItem::McpToolsParserStringEscapes]);

    assert_eq!(
        parse_value(r#""back\\slash""#).unwrap(),
        Value::String("back\\slash".into())
    );
    assert_eq!(
        parse_value(r#""quote\"here""#).unwrap(),
        Value::String("quote\"here".into())
    );
    assert_eq!(
        parse_value(r#""line\nbreak""#).unwrap(),
        Value::String("line\nbreak".into())
    );
    assert_eq!(
        parse_value(r#""carr\rret""#).unwrap(),
        Value::String("carr\rret".into())
    );
    assert_eq!(
        parse_value(r#""tab\tinside""#).unwrap(),
        Value::String("tab\tinside".into())
    );
}

#[test]
fn rejects_unrecognized_string_escapes() {
    covers!([SpecItem::McpToolsParserStringEscapes]);

    let cases = [r#""\x""#, r#""\0""#, r#""\a""#, r#""\b""#];
    for input in cases {
        let err = parse_value(input).unwrap_err();
        let downcast: &ParseError = err
            .downcast_ref::<ParseError>()
            .expect("error chain should include ParseError");
        match downcast {
            ParseError::InvalidEscape { .. } => {}
            other => panic!("expected InvalidEscape for {:?}, got {:?}", input, other),
        }
    }
}

#[test]
fn line_comments_are_discarded_by_parse_value() {
    covers!([SpecItem::McpToolsParserComments]);

    let v = parse_value("; intro\n42").unwrap();
    assert_eq!(v, Value::Integer(42));

    let v = parse_value("(a ; trailing\n  b)").unwrap();
    assert_eq!(v.as_list().unwrap().len(), 2);
}

#[test]
fn block_comments_are_discarded_by_parse_value() {
    covers!([SpecItem::McpToolsParserComments]);

    let v = parse_value("#| header |# 7").unwrap();
    assert_eq!(v, Value::Integer(7));

    let v = parse_value("#| outer #| inner |# tail |# 9").unwrap();
    assert_eq!(v, Value::Integer(9));
}

#[test]
fn nested_block_comments_track_depth() {
    covers!([SpecItem::McpToolsParserComments]);

    // The outer comment must close at the SECOND |#, not the first.
    let v = parse_value("#| #| inner |# still outer |# x").unwrap();
    assert_eq!(v, Value::Symbol("x".into()));
}
