//! Integration tests for source positions and comment retention.

mod common;

use common::{covers, SpecItem};
use mcp_tools::{parse_value_with_positions, SpannedNode};

#[test]
fn span_covers_full_form_including_parens() {
    covers!([SpecItem::McpToolsParserSpans]);

    let s = parse_value_with_positions("(a b)").unwrap();
    assert_eq!(s.span.start.line, 1);
    assert_eq!(s.span.start.column, 1);
    assert_eq!(s.span.start.byte_offset, 0);
    assert_eq!(s.span.end.column, 6); // exclusive end past `)`
    assert_eq!(s.span.end.byte_offset, 5);
}

#[test]
fn nested_spans_track_inner_positions() {
    covers!([SpecItem::McpToolsParserSpans]);

    // (a (b c) d)
    //  ^  ^^^  ^
    let s = parse_value_with_positions("(a (b c) d)").unwrap();
    if let SpannedNode::List(items) = &s.value {
        assert_eq!(items[0].span.start.column, 2);
        assert_eq!(items[1].span.start.column, 4);
        assert_eq!(items[1].span.end.column, 9);
        assert_eq!(items[2].span.start.column, 10);
    } else {
        panic!("expected outer List");
    }
}

#[test]
fn multibyte_characters_advance_column_by_one() {
    covers!([SpecItem::McpToolsParserSpans]);

    // "héllo" is 5 chars but 6 bytes (é is 2 bytes); column should count chars.
    let s = parse_value_with_positions(r#"("héllo" x)"#).unwrap();
    if let SpannedNode::List(items) = &s.value {
        // The string starts at column 2 (after '(') and ends after the closing quote.
        assert_eq!(items[0].span.start.column, 2);
        // 7 characters (open quote, h, é, l, l, o, close quote) → end column = 2 + 7 = 9.
        assert_eq!(items[0].span.end.column, 9);
        // byte offsets do count bytes — string contents are 6 bytes (h é=2 l l o)
        // plus 2 quotes = 8 bytes for the literal; opening at byte 1, closing past byte 8.
        assert_eq!(items[0].span.end.byte_offset, 9);
        // x starts at column 10
        assert_eq!(items[1].span.start.column, 10);
    } else {
        panic!("expected outer List");
    }
}

#[test]
fn crlf_treated_as_single_line_break() {
    covers!([SpecItem::McpToolsParserSpans]);

    let s = parse_value_with_positions("(a\r\n b)").unwrap();
    if let SpannedNode::List(items) = &s.value {
        assert_eq!(items[0].span.start.line, 1);
        assert_eq!(items[1].span.start.line, 2);
        assert_eq!(items[1].span.start.column, 2); // after the leading space
    } else {
        panic!("expected List");
    }
}

#[test]
fn position_is_one_indexed_for_humans_zero_indexed_for_lsp() {
    covers!([SpecItem::McpToolsParserSpans]);

    let s = parse_value_with_positions("foo").unwrap();
    let pos = s.span.start;
    assert_eq!(pos.line, 1);
    assert_eq!(pos.column, 1);
    assert_eq!(pos.byte_offset, 0);

    let lsp = pos.lsp();
    assert_eq!(lsp, (0, 0));
}

#[test]
fn leading_comment_attaches_to_following_value() {
    covers!([SpecItem::McpToolsParserCommentRetention]);

    let s = parse_value_with_positions("; intro\n42").unwrap();
    assert_eq!(s.leading_comments.len(), 1);
    assert_eq!(s.leading_comments[0].text, " intro");
    assert!(s.trailing_comments.is_empty());
}

#[test]
fn trailing_comment_on_same_line_attaches_to_preceding_value() {
    covers!([SpecItem::McpToolsParserCommentRetention]);

    let s = parse_value_with_positions("42 ; tail").unwrap();
    assert_eq!(s.trailing_comments.len(), 1);
    assert_eq!(s.trailing_comments[0].text, " tail");
}

#[test]
fn comments_inside_lists_attach_to_neighbors() {
    covers!([SpecItem::McpToolsParserCommentRetention]);

    let src = "(a ; first comment\n  b ; second\n  c)";
    let s = parse_value_with_positions(src).unwrap();
    if let SpannedNode::List(items) = &s.value {
        assert_eq!(items[0].trailing_comments.len(), 1);
        assert_eq!(items[0].trailing_comments[0].text, " first comment");
        assert_eq!(items[1].trailing_comments.len(), 1);
        assert_eq!(items[1].trailing_comments[0].text, " second");
    } else {
        panic!("expected List");
    }
}

#[test]
fn block_comment_is_retained_with_positions() {
    covers!([SpecItem::McpToolsParserCommentRetention]);

    let src = "#| header |# 7";
    let s = parse_value_with_positions(src).unwrap();
    assert_eq!(s.leading_comments.len(), 1);
    assert_eq!(s.leading_comments[0].text, " header ");
}
