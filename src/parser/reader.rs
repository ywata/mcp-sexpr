//! Recursive-descent reader producing [`Spanned`] values from the lexer's token stream.
//!
//! See `specs/parser/grammar.md` and `specs/parser/source-positions.md`.

use std::fmt;

use super::lexer::{tokenize, LexError, SpannedToken, Token};
use super::types::{Comment, Position, Span, Spanned, SpannedNode, Value};

/// Top-level parse: returns a [`Value`] with comments and spans discarded.
pub fn parse_value(input: &str) -> Result<Value, ParseError> {
    let spanned = parse_value_with_positions(input)?;
    Ok(spanned.into_value())
}

/// Top-level parse retaining source positions and adjacent comments.
pub fn parse_value_with_positions(input: &str) -> Result<Spanned, ParseError> {
    let tokens = tokenize(input).map_err(ParseError::from)?;
    let mut reader = Reader::new(tokens);
    let value = reader.read_value()?;
    let leading = std::mem::take(&mut reader.pending_leading);
    let value = reader.attach_leading(value, leading);
    reader.expect_eof()?;
    Ok(value)
}

/// Parse failure surfaced from the reader (or wrapped from the lexer).
#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    /// Source larger than 4 GiB.
    SourceTooLarge,
    /// Unexpected character at top level (lexer-originated).
    UnexpectedChar {
        /// The offending character.
        ch: char,
        /// Position at which it appeared.
        position: Position,
    },
    /// String literal opened but never closed.
    UnterminatedString {
        /// Position of the opening quote.
        position: Position,
    },
    /// Block comment opened but never closed.
    UnterminatedBlockComment {
        /// Position of the opening `#|`.
        position: Position,
    },
    /// Backslash escape inside a string was not recognized.
    InvalidEscape {
        /// The literal escape sequence.
        sequence: String,
        /// Position of the leading backslash.
        position: Position,
    },
    /// Numeric literal outside `i64` range.
    IntegerOutOfRange {
        /// Position of the literal.
        position: Position,
    },
    /// Numeric literal failed to parse as either integer or float.
    InvalidNumber {
        /// Position of the literal.
        position: Position,
    },
    /// Mismatched closing paren — extra `)` with no matching `(`.
    UnmatchedRParen {
        /// Position of the offending `)`.
        position: Position,
    },
    /// Open paren without matching close before EOF.
    UnclosedList {
        /// Position of the open paren.
        position: Position,
    },
    /// Dot in a list with no left-hand value.
    DotWithoutHead {
        /// Position of the dot.
        position: Position,
    },
    /// Dot at the end of a list with no right-hand value.
    DotWithoutTail {
        /// Position of the dot.
        position: Position,
    },
    /// More than one value after a dot — `(a . b c)` is malformed.
    DotWithMultipleTail {
        /// Position of the offending value after the dot's tail.
        position: Position,
    },
    /// Quote prefix (`'`, `` ` ``, `,`, `,@`) with no value following.
    QuoteWithoutValue {
        /// Position of the quote character.
        position: Position,
    },
    /// EOF where a value was expected.
    UnexpectedEof,
    /// Trailing input after the top-level form.
    TrailingInput {
        /// Position of the unexpected token.
        position: Position,
    },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::SourceTooLarge => f.write_str("source larger than 4 GiB is not supported"),
            ParseError::UnexpectedChar { ch, position } => {
                write!(f, "unexpected character {:?} at {}:{}", ch, position.line, position.column)
            }
            ParseError::UnterminatedString { position } => write!(
                f,
                "unterminated string starting at {}:{}",
                position.line, position.column
            ),
            ParseError::UnterminatedBlockComment { position } => write!(
                f,
                "unterminated block comment starting at {}:{}",
                position.line, position.column
            ),
            ParseError::InvalidEscape { sequence, position } => write!(
                f,
                "invalid string escape {:?} at {}:{}",
                sequence, position.line, position.column
            ),
            ParseError::IntegerOutOfRange { position } => write!(
                f,
                "integer literal out of i64 range at {}:{}",
                position.line, position.column
            ),
            ParseError::InvalidNumber { position } => write!(
                f,
                "invalid numeric literal at {}:{}",
                position.line, position.column
            ),
            ParseError::UnmatchedRParen { position } => write!(
                f,
                "unmatched ')' at {}:{}",
                position.line, position.column
            ),
            ParseError::UnclosedList { position } => write!(
                f,
                "unclosed '(' from {}:{}",
                position.line, position.column
            ),
            ParseError::DotWithoutHead { position } => write!(
                f,
                "dot without preceding value at {}:{}",
                position.line, position.column
            ),
            ParseError::DotWithoutTail { position } => write!(
                f,
                "dot without following value at {}:{}",
                position.line, position.column
            ),
            ParseError::DotWithMultipleTail { position } => write!(
                f,
                "more than one value after '.' at {}:{}",
                position.line, position.column
            ),
            ParseError::QuoteWithoutValue { position } => write!(
                f,
                "quote prefix without following value at {}:{}",
                position.line, position.column
            ),
            ParseError::UnexpectedEof => f.write_str("unexpected end of input"),
            ParseError::TrailingInput { position } => write!(
                f,
                "trailing input at {}:{}",
                position.line, position.column
            ),
        }
    }
}

impl std::error::Error for ParseError {}

impl From<LexError> for ParseError {
    fn from(err: LexError) -> Self {
        match err {
            LexError::SourceTooLarge => ParseError::SourceTooLarge,
            LexError::UnexpectedChar { ch, position } => {
                ParseError::UnexpectedChar { ch, position }
            }
            LexError::UnterminatedString { position } => {
                ParseError::UnterminatedString { position }
            }
            LexError::UnterminatedBlockComment { position } => {
                ParseError::UnterminatedBlockComment { position }
            }
            LexError::InvalidEscape { sequence, position } => {
                ParseError::InvalidEscape { sequence, position }
            }
            LexError::IntegerOutOfRange { position } => {
                ParseError::IntegerOutOfRange { position }
            }
            LexError::InvalidNumber { position } => ParseError::InvalidNumber { position },
        }
    }
}

struct Reader {
    tokens: Vec<SpannedToken>,
    cursor: usize,
    pending_leading: Vec<Comment>,
}

impl Reader {
    fn new(tokens: Vec<SpannedToken>) -> Self {
        Reader {
            tokens,
            cursor: 0,
            pending_leading: Vec::new(),
        }
    }

    fn peek_kind(&self) -> Option<&Token> {
        self.tokens.get(self.cursor).map(|t| &t.token)
    }

    fn peek_span(&self) -> Option<Span> {
        self.tokens.get(self.cursor).map(|t| t.span)
    }

    fn advance(&mut self) -> Option<SpannedToken> {
        let tok = self.tokens.get(self.cursor).cloned();
        if tok.is_some() {
            self.cursor += 1;
        }
        tok
    }

    /// Consume comment tokens at the current position into `pending_leading`. After this,
    /// `peek_kind` is either a value-bearing token or `None`.
    fn drain_leading_comments(&mut self) {
        while let Some(Token::Comment(c)) = self.peek_kind() {
            let c = c.clone();
            self.advance();
            self.pending_leading.push(c);
        }
    }

    /// Drain trailing same-line comments that follow a value's `end_position`.
    fn drain_trailing_comments(&mut self, value_end: Position) -> Vec<Comment> {
        let mut out = Vec::new();
        while let Some(Token::Comment(c)) = self.peek_kind() {
            if c.span.start.line == value_end.line {
                let c = c.clone();
                self.advance();
                out.push(c);
            } else {
                break;
            }
        }
        out
    }

    fn attach_leading(&self, mut value: Spanned, mut leading: Vec<Comment>) -> Spanned {
        if leading.is_empty() {
            return value;
        }
        leading.append(&mut value.leading_comments);
        value.leading_comments = leading;
        value
    }

    fn expect_eof(&mut self) -> Result<(), ParseError> {
        self.drain_leading_comments();
        match self.peek_kind() {
            None => Ok(()),
            Some(_) => {
                let pos = self.peek_span().unwrap().start;
                Err(ParseError::TrailingInput { position: pos })
            }
        }
    }

    fn read_value(&mut self) -> Result<Spanned, ParseError> {
        self.drain_leading_comments();
        let leading = std::mem::take(&mut self.pending_leading);
        let value = self.read_one()?;
        let value = self.attach_leading(value, leading);
        let trailing = self.drain_trailing_comments(value.span.end);
        Ok(Spanned {
            trailing_comments: trailing,
            ..value
        })
    }

    fn read_one(&mut self) -> Result<Spanned, ParseError> {
        let Some(spanned_tok) = self.tokens.get(self.cursor).cloned() else {
            return Err(ParseError::UnexpectedEof);
        };
        match spanned_tok.token {
            Token::Comment(_) => {
                // shouldn't happen; comments drained at higher level
                self.advance();
                self.read_one()
            }
            Token::LParen => self.read_list(spanned_tok.span),
            Token::RParen => Err(ParseError::UnmatchedRParen {
                position: spanned_tok.span.start,
            }),
            Token::Dot => Err(ParseError::DotWithoutHead {
                position: spanned_tok.span.start,
            }),
            Token::Quote => self.read_quoted("quote", spanned_tok.span),
            Token::Quasiquote => self.read_quoted("quasiquote", spanned_tok.span),
            Token::Unquote => self.read_quoted("unquote", spanned_tok.span),
            Token::UnquoteSplicing => self.read_quoted("unquote-splicing", spanned_tok.span),
            Token::Bool(b) => {
                self.advance();
                Ok(self.atom(SpannedNode::Bool(b), spanned_tok.span))
            }
            Token::Integer(n) => {
                self.advance();
                Ok(self.atom(SpannedNode::Integer(n), spanned_tok.span))
            }
            Token::Float(x) => {
                self.advance();
                Ok(self.atom(SpannedNode::Float(x), spanned_tok.span))
            }
            Token::String(s) => {
                self.advance();
                Ok(self.atom(SpannedNode::String(s), spanned_tok.span))
            }
            Token::Symbol(s) => {
                self.advance();
                let node = if s == "nil" {
                    SpannedNode::Nil
                } else {
                    SpannedNode::Symbol(s)
                };
                Ok(self.atom(node, spanned_tok.span))
            }
            Token::Keyword(s) => {
                self.advance();
                Ok(self.atom(SpannedNode::Keyword(s), spanned_tok.span))
            }
        }
    }

    fn atom(&self, node: SpannedNode, span: Span) -> Spanned {
        Spanned {
            value: node,
            span,
            leading_comments: Vec::new(),
            trailing_comments: Vec::new(),
        }
    }

    fn read_list(&mut self, lparen_span: Span) -> Result<Spanned, ParseError> {
        // current token is LParen
        self.advance();

        let mut items: Vec<Spanned> = Vec::new();
        let mut dotted_tail: Option<Spanned> = None;

        loop {
            self.drain_leading_comments();

            match self.peek_kind() {
                None => {
                    return Err(ParseError::UnclosedList {
                        position: lparen_span.start,
                    })
                }
                Some(Token::RParen) => {
                    let close = self.advance().unwrap();
                    let span = Span::new(lparen_span.start, close.span.end);
                    if !self.pending_leading.is_empty() {
                        if let Some(last) = items.last_mut() {
                            last.trailing_comments
                                .extend(self.pending_leading.drain(..));
                        } else {
                            self.pending_leading.clear();
                        }
                    }
                    let node = match (items.is_empty(), dotted_tail) {
                        (true, None) => SpannedNode::Nil,
                        (true, Some(_)) => unreachable!("dot with empty list rejected earlier"),
                        (false, None) => SpannedNode::List(items),
                        (false, Some(tail)) => {
                            // Build (a b . c) as either List([a, b, c]) when c is a List, or
                            // a right-folded Pair chain otherwise.
                            build_dotted(items, tail)
                        }
                    };
                    return Ok(Spanned {
                        value: node,
                        span,
                        leading_comments: Vec::new(),
                        trailing_comments: Vec::new(),
                    });
                }
                Some(Token::Dot) => {
                    let dot_tok = self.advance().unwrap();
                    if items.is_empty() {
                        return Err(ParseError::DotWithoutHead {
                            position: dot_tok.span.start,
                        });
                    }
                    self.drain_leading_comments();
                    if matches!(self.peek_kind(), Some(Token::RParen) | None) {
                        return Err(ParseError::DotWithoutTail {
                            position: dot_tok.span.start,
                        });
                    }
                    let leading = std::mem::take(&mut self.pending_leading);
                    let tail = self.read_one()?;
                    let tail = self.attach_leading(tail, leading);
                    let trailing = self.drain_trailing_comments(tail.span.end);
                    let tail = Spanned {
                        trailing_comments: trailing,
                        ..tail
                    };
                    self.drain_leading_comments();
                    match self.peek_kind() {
                        Some(Token::RParen) | None => {}
                        Some(_) => {
                            let pos = self.peek_span().unwrap().start;
                            return Err(ParseError::DotWithMultipleTail { position: pos });
                        }
                    }
                    dotted_tail = Some(tail);
                }
                Some(_) => {
                    let leading = std::mem::take(&mut self.pending_leading);
                    let item = self.read_one()?;
                    let item = self.attach_leading(item, leading);
                    let trailing = self.drain_trailing_comments(item.span.end);
                    items.push(Spanned {
                        trailing_comments: trailing,
                        ..item
                    });
                }
            }
        }
    }

    fn read_quoted(&mut self, head: &str, prefix_span: Span) -> Result<Spanned, ParseError> {
        self.advance();
        self.drain_leading_comments();
        if self.peek_kind().is_none() {
            return Err(ParseError::QuoteWithoutValue {
                position: prefix_span.start,
            });
        }
        let leading = std::mem::take(&mut self.pending_leading);
        let inner = self.read_one()?;
        let inner = self.attach_leading(inner, leading);
        let trailing = self.drain_trailing_comments(inner.span.end);
        let inner = Spanned {
            trailing_comments: trailing,
            ..inner
        };
        let span = Span::new(prefix_span.start, inner.span.end);
        let head_node = Spanned {
            value: SpannedNode::Symbol(head.to_string()),
            span: Span::new(prefix_span.start, prefix_span.start),
            leading_comments: Vec::new(),
            trailing_comments: Vec::new(),
        };
        Ok(Spanned {
            value: SpannedNode::List(vec![head_node, inner]),
            span,
            leading_comments: Vec::new(),
            trailing_comments: Vec::new(),
        })
    }
}

fn build_dotted(items: Vec<Spanned>, tail: Spanned) -> SpannedNode {
    if let SpannedNode::List(tail_items) = tail.value {
        let mut combined = items;
        combined.extend(tail_items);
        SpannedNode::List(combined)
    } else if let SpannedNode::Nil = tail.value {
        SpannedNode::List(items)
    } else {
        // right-fold (a b . c) -> Pair(a, Pair(b, c))
        let mut tail = tail;
        for item in items.into_iter().rev() {
            let span = Span::new(item.span.start, tail.span.end);
            tail = Spanned {
                value: SpannedNode::Pair(Box::new((item, tail))),
                span,
                leading_comments: Vec::new(),
                trailing_comments: Vec::new(),
            };
        }
        tail.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SpecItem;

    macro_rules! covers {
        ([$($item:expr),* $(,)?]) => {
            {
                $(
                    let _ = $item;
                )*
            }
        };
    }

    #[test]
    fn parses_atoms_and_lists() {
        covers!([SpecItem::McpToolsParserGrammar, SpecItem::McpToolsParserParseValue]);

        assert_eq!(parse_value("()").unwrap(), Value::Nil);
        assert_eq!(parse_value("nil").unwrap(), Value::Nil);
        assert_eq!(parse_value("#t").unwrap(), Value::Bool(true));
        assert_eq!(parse_value("42").unwrap(), Value::Integer(42));
        assert_eq!(parse_value("1.5").unwrap(), Value::Float(1.5));
        assert_eq!(
            parse_value("\"hi\"").unwrap(),
            Value::String("hi".into())
        );
        assert_eq!(
            parse_value("(1 2 3)").unwrap(),
            Value::List(vec![Value::Integer(1), Value::Integer(2), Value::Integer(3)])
        );
    }

    #[test]
    fn parses_dotted_pairs() {
        covers!([SpecItem::McpToolsParserGrammar, SpecItem::McpToolsParserListRepresentation]);

        let v = parse_value("(a . b)").unwrap();
        assert!(v.is_pair());
        let (car, cdr) = v.as_pair().unwrap();
        assert_eq!(car.as_symbol(), Some("a"));
        assert_eq!(cdr.as_symbol(), Some("b"));
    }

    #[test]
    fn parses_quote_forms() {
        covers!([SpecItem::McpToolsParserGrammar]);

        assert_eq!(
            parse_value("'x").unwrap(),
            Value::List(vec![
                Value::Symbol("quote".into()),
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
    fn keyword_canonicalized_without_colon() {
        covers!([SpecItem::McpToolsParserKeywordCanonicalization]);

        let v = parse_value(":foo").unwrap();
        assert_eq!(v, Value::Keyword("foo".into()));
    }

    #[test]
    fn rejects_invalid_string_escape() {
        covers!([SpecItem::McpToolsParserStringEscapes]);

        let err = parse_value(r#""\x""#).unwrap_err();
        match err {
            ParseError::InvalidEscape { sequence, .. } => assert_eq!(sequence, "\\x"),
            other => panic!("expected InvalidEscape, got {:?}", other),
        }
    }

    #[test]
    fn unclosed_list_reports_open_paren_position() {
        covers!([SpecItem::McpToolsParserGrammar]);

        let err = parse_value("(a b").unwrap_err();
        match err {
            ParseError::UnclosedList { position } => {
                assert_eq!(position.line, 1);
                assert_eq!(position.column, 1);
            }
            other => panic!("expected UnclosedList, got {:?}", other),
        }
    }

    #[test]
    fn span_covers_full_form() {
        covers!([SpecItem::McpToolsParserSpans]);

        let s = parse_value_with_positions("(a b)").unwrap();
        assert_eq!(s.span.start.column, 1);
        assert_eq!(s.span.end.column, 6); // exclusive end past the ')'
        if let SpannedNode::List(items) = &s.value {
            assert_eq!(items[0].span.start.column, 2);
            assert_eq!(items[1].span.start.column, 4);
        } else {
            panic!("expected List");
        }
    }

    #[test]
    fn leading_comment_attaches_to_next_value() {
        covers!([SpecItem::McpToolsParserCommentRetention]);

        let s = parse_value_with_positions("; intro\n42").unwrap();
        assert_eq!(s.leading_comments.len(), 1);
        assert_eq!(s.leading_comments[0].text, " intro");
    }

    #[test]
    fn trailing_same_line_comment_attaches_to_value() {
        covers!([SpecItem::McpToolsParserCommentRetention]);

        let s = parse_value_with_positions("42 ; tail").unwrap();
        assert_eq!(s.trailing_comments.len(), 1);
        assert_eq!(s.trailing_comments[0].text, " tail");
    }

    #[test]
    fn trailing_input_is_an_error() {
        covers!([SpecItem::McpToolsParserParseValue]);

        let err = parse_value("1 2").unwrap_err();
        match err {
            ParseError::TrailingInput { .. } => {}
            other => panic!("expected TrailingInput, got {:?}", other),
        }
    }

    #[test]
    fn parses_keyword_value_pairs() {
        covers!([SpecItem::McpToolsParserParseValue]);

        let v = parse_value(r#"(tool :name "x")"#).unwrap();
        let xs = v.as_list().unwrap();
        assert_eq!(xs[0].as_symbol(), Some("tool"));
        assert_eq!(xs[1], Value::Keyword("name".into()));
        assert_eq!(xs[2], Value::String("x".into()));
    }
}
