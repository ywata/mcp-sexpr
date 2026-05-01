//! Hand-rolled tokenizer for the position-tracking parser.
//!
//! See `specs/parser/grammar.md` for the recognized surface and
//! `specs/parser/source-positions.md` for the position-tracking rules.

use super::types::{Comment, CommentKind, Position, Span};

/// Token produced by the lexer.
///
/// Comments are kept in the token stream; the reader is responsible for attaching
/// them to nearby values.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// `(`
    LParen,
    /// `)`
    RParen,
    /// `.` (in token position — between two atoms inside parens; bare dots in symbols are part of the symbol).
    Dot,
    /// `'` (quote prefix).
    Quote,
    /// `` ` `` (quasiquote prefix).
    Quasiquote,
    /// `,` (unquote prefix).
    Unquote,
    /// `,@` (unquote-splicing prefix).
    UnquoteSplicing,
    /// Boolean literal `#t` / `#f`.
    Bool(bool),
    /// Integer literal.
    Integer(i64),
    /// Floating literal.
    Float(f64),
    /// Quoted string literal (escape-decoded).
    String(String),
    /// Identifier-like atom (includes `nil`).
    Symbol(String),
    /// `:foo` keyword (without leading colon).
    Keyword(String),
    /// Line or block comment.
    Comment(Comment),
}

/// One token plus the span it covers.
#[derive(Debug, Clone, PartialEq)]
pub struct SpannedToken {
    /// Lexer-recognized payload.
    pub token: Token,
    /// Source range covered by the token.
    pub span: Span,
}

/// Failure raised by the lexer.
#[derive(Debug, Clone, PartialEq)]
pub enum LexError {
    /// Source larger than 4 GiB; offsets would overflow `u32`.
    SourceTooLarge,
    /// Unrecognized character at top level (e.g., `[`, `]`, etc.).
    UnexpectedChar {
        /// The offending character.
        ch: char,
        /// Position at which it appeared.
        position: Position,
    },
    /// String opened but never closed before EOF.
    UnterminatedString {
        /// Position of the opening quote.
        position: Position,
    },
    /// Block comment opened but never closed before EOF.
    UnterminatedBlockComment {
        /// Position of the opening `#|`.
        position: Position,
    },
    /// Backslash escape inside a string was not recognized.
    InvalidEscape {
        /// The literal escape sequence (e.g., `\\x`).
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
}

/// Tokenize the input. Returns the full token stream (including comments) on success,
/// or the first lex error encountered.
pub fn tokenize(input: &str) -> Result<Vec<SpannedToken>, LexError> {
    let mut lexer = Lexer::new(input)?;
    lexer.consume_bom();
    let mut out = Vec::new();
    while let Some(tok) = lexer.next_token()? {
        out.push(tok);
    }
    Ok(out)
}

struct Lexer<'a> {
    input: &'a str,
    bytes: &'a [u8],
    /// Current byte offset into `input`.
    pos: usize,
    /// Current line (1-indexed).
    line: u32,
    /// Current column in Unicode scalar values (1-indexed).
    column: u32,
}

impl<'a> Lexer<'a> {
    fn new(input: &'a str) -> Result<Self, LexError> {
        if input.len() > u32::MAX as usize {
            return Err(LexError::SourceTooLarge);
        }
        Ok(Lexer {
            input,
            bytes: input.as_bytes(),
            pos: 0,
            line: 1,
            column: 1,
        })
    }

    fn consume_bom(&mut self) {
        if self.bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
            self.pos += 3;
        }
    }

    fn current_position(&self) -> Position {
        Position {
            line: self.line,
            column: self.column,
            byte_offset: self.pos as u32,
        }
    }

    fn peek_char(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn peek_char_at(&self, offset: usize) -> Option<char> {
        self.input[self.pos + offset..].chars().next()
    }

    fn advance_char(&mut self) -> Option<char> {
        let ch = self.peek_char()?;
        let len = ch.len_utf8();
        self.pos += len;
        if ch == '\n' {
            self.line += 1;
            self.column = 1;
        } else if ch == '\r' {
            self.line += 1;
            self.column = 1;
            if self.peek_char() == Some('\n') {
                self.pos += 1;
            }
        } else {
            self.column += 1;
        }
        Some(ch)
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek_char() {
            if ch.is_whitespace() {
                self.advance_char();
            } else {
                break;
            }
        }
    }

    fn next_token(&mut self) -> Result<Option<SpannedToken>, LexError> {
        self.skip_whitespace();
        let start = self.current_position();

        let Some(ch) = self.peek_char() else {
            return Ok(None);
        };

        let token = match ch {
            '(' => {
                self.advance_char();
                Token::LParen
            }
            ')' => {
                self.advance_char();
                Token::RParen
            }
            '\'' => {
                self.advance_char();
                Token::Quote
            }
            '`' => {
                self.advance_char();
                Token::Quasiquote
            }
            ',' => {
                self.advance_char();
                if self.peek_char() == Some('@') {
                    self.advance_char();
                    Token::UnquoteSplicing
                } else {
                    Token::Unquote
                }
            }
            ';' => return self.lex_line_comment(start).map(Some),
            '#' => match self.peek_char_at(1) {
                Some('t') => {
                    self.advance_char();
                    self.advance_char();
                    Token::Bool(true)
                }
                Some('f') => {
                    self.advance_char();
                    self.advance_char();
                    Token::Bool(false)
                }
                Some('|') => return self.lex_block_comment(start).map(Some),
                _ => {
                    return Err(LexError::UnexpectedChar {
                        ch,
                        position: start,
                    })
                }
            },
            '"' => return self.lex_string(start).map(Some),
            ':' => return self.lex_keyword(start).map(Some),
            '.' if self.next_atom_break(1) => {
                self.advance_char();
                Token::Dot
            }
            '-' | '+' if matches!(self.peek_char_at(1), Some(c) if c.is_ascii_digit() || c == '.') => {
                return self.lex_number(start).map(Some)
            }
            c if c.is_ascii_digit() => return self.lex_number(start).map(Some),
            c if is_symbol_start(c) => return self.lex_symbol(start).map(Some),
            _ => {
                return Err(LexError::UnexpectedChar {
                    ch,
                    position: start,
                })
            }
        };
        let end = self.current_position();
        Ok(Some(SpannedToken {
            token,
            span: Span::new(start, end),
        }))
    }

    /// Returns true if the character `offset` bytes ahead is a token break (whitespace,
    /// paren, EOF). Used to disambiguate a bare `.` token from a dot inside a symbol like
    /// `foo.bar`.
    fn next_atom_break(&self, offset: usize) -> bool {
        match self.peek_char_at(offset) {
            None => true,
            Some(c) => c.is_whitespace() || c == '(' || c == ')' || c == ';',
        }
    }

    fn lex_line_comment(&mut self, start: Position) -> Result<SpannedToken, LexError> {
        // current char is ';'
        let body_start = self.pos;
        self.advance_char();
        while let Some(ch) = self.peek_char() {
            if ch == '\n' || ch == '\r' {
                break;
            }
            self.advance_char();
        }
        let body_end = self.pos;
        let end = self.current_position();
        let text = self.input[body_start + 1..body_end].to_string();
        Ok(SpannedToken {
            token: Token::Comment(Comment {
                kind: CommentKind::Line,
                text,
                span: Span::new(start, end),
            }),
            span: Span::new(start, end),
        })
    }

    fn lex_block_comment(&mut self, start: Position) -> Result<SpannedToken, LexError> {
        // current chars are '#|'
        let body_start = self.pos + 2;
        self.advance_char();
        self.advance_char();
        let mut depth: usize = 1;
        while depth > 0 {
            let Some(ch) = self.peek_char() else {
                return Err(LexError::UnterminatedBlockComment { position: start });
            };
            if ch == '#' && self.peek_char_at(1) == Some('|') {
                self.advance_char();
                self.advance_char();
                depth += 1;
                continue;
            }
            if ch == '|' && self.peek_char_at(1) == Some('#') {
                self.advance_char();
                self.advance_char();
                depth -= 1;
                continue;
            }
            self.advance_char();
        }
        let body_end = self.pos - 2;
        let end = self.current_position();
        let text = self.input[body_start..body_end].to_string();
        Ok(SpannedToken {
            token: Token::Comment(Comment {
                kind: CommentKind::Block,
                text,
                span: Span::new(start, end),
            }),
            span: Span::new(start, end),
        })
    }

    fn lex_string(&mut self, start: Position) -> Result<SpannedToken, LexError> {
        // current char is '"'
        self.advance_char();
        let mut buf = String::new();
        loop {
            let Some(ch) = self.peek_char() else {
                return Err(LexError::UnterminatedString { position: start });
            };
            if ch == '"' {
                self.advance_char();
                let end = self.current_position();
                return Ok(SpannedToken {
                    token: Token::String(buf),
                    span: Span::new(start, end),
                });
            }
            if ch == '\\' {
                let escape_start = self.current_position();
                self.advance_char();
                let Some(esc) = self.peek_char() else {
                    return Err(LexError::UnterminatedString { position: start });
                };
                match esc {
                    '\\' => buf.push('\\'),
                    '"' => buf.push('"'),
                    'n' => buf.push('\n'),
                    'r' => buf.push('\r'),
                    't' => buf.push('\t'),
                    other => {
                        return Err(LexError::InvalidEscape {
                            sequence: format!("\\{}", other),
                            position: escape_start,
                        })
                    }
                }
                self.advance_char();
                continue;
            }
            buf.push(ch);
            self.advance_char();
        }
    }

    fn lex_keyword(&mut self, start: Position) -> Result<SpannedToken, LexError> {
        // current char is ':'
        self.advance_char();
        let name_start = self.pos;
        while let Some(ch) = self.peek_char() {
            if is_symbol_continue(ch) {
                self.advance_char();
            } else {
                break;
            }
        }
        let name_end = self.pos;
        let end = self.current_position();
        let name = self.input[name_start..name_end].to_string();
        Ok(SpannedToken {
            token: Token::Keyword(name),
            span: Span::new(start, end),
        })
    }

    fn lex_symbol(&mut self, start: Position) -> Result<SpannedToken, LexError> {
        let name_start = self.pos;
        while let Some(ch) = self.peek_char() {
            if is_symbol_continue(ch) {
                self.advance_char();
            } else {
                break;
            }
        }
        let name_end = self.pos;
        let end = self.current_position();
        let name = &self.input[name_start..name_end];
        Ok(SpannedToken {
            token: Token::Symbol(name.to_string()),
            span: Span::new(start, end),
        })
    }

    fn lex_number(&mut self, start: Position) -> Result<SpannedToken, LexError> {
        let lit_start = self.pos;
        // optional sign
        if matches!(self.peek_char(), Some('+') | Some('-')) {
            self.advance_char();
        }
        let mut saw_digit = false;
        let mut saw_dot = false;
        let mut saw_exp = false;

        while let Some(ch) = self.peek_char() {
            if ch.is_ascii_digit() {
                saw_digit = true;
                self.advance_char();
            } else if ch == '.' && !saw_dot && !saw_exp {
                saw_dot = true;
                self.advance_char();
            } else if (ch == 'e' || ch == 'E') && !saw_exp && saw_digit {
                saw_exp = true;
                self.advance_char();
                if matches!(self.peek_char(), Some('+') | Some('-')) {
                    self.advance_char();
                }
            } else {
                break;
            }
        }
        let lit_end = self.pos;
        let end = self.current_position();
        let lit = &self.input[lit_start..lit_end];

        if !saw_digit {
            return Err(LexError::InvalidNumber { position: start });
        }

        let token = if saw_dot || saw_exp {
            match lit.parse::<f64>() {
                Ok(x) => Token::Float(x),
                Err(_) => return Err(LexError::InvalidNumber { position: start }),
            }
        } else {
            match lit.parse::<i64>() {
                Ok(n) => Token::Integer(n),
                Err(_) => return Err(LexError::IntegerOutOfRange { position: start }),
            }
        };

        Ok(SpannedToken {
            token,
            span: Span::new(start, end),
        })
    }
}

fn is_symbol_start(ch: char) -> bool {
    match ch {
        '(' | ')' | '"' | '\'' | '`' | ',' | ';' | '#' | ':' => false,
        c if c.is_whitespace() => false,
        c if c.is_ascii_digit() => false,
        _ => true,
    }
}

fn is_symbol_continue(ch: char) -> bool {
    match ch {
        '(' | ')' | '"' | '\'' | '`' | ',' | ';' | '#' => false,
        c if c.is_whitespace() => false,
        _ => true,
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

    fn tokens_of(input: &str) -> Vec<Token> {
        tokenize(input)
            .unwrap()
            .into_iter()
            .map(|t| t.token)
            .collect()
    }

    #[test]
    fn tokenize_atoms() {
        covers!([SpecItem::McpToolsParserGrammar]);

        assert_eq!(tokens_of("nil"), vec![Token::Symbol("nil".into())]);
        assert_eq!(tokens_of("#t"), vec![Token::Bool(true)]);
        assert_eq!(tokens_of("#f"), vec![Token::Bool(false)]);
        assert_eq!(tokens_of("42"), vec![Token::Integer(42)]);
        assert_eq!(tokens_of("-7"), vec![Token::Integer(-7)]);
        assert_eq!(tokens_of("1.5"), vec![Token::Float(1.5)]);
        assert_eq!(tokens_of(":foo"), vec![Token::Keyword("foo".into())]);
    }

    #[test]
    fn tokenize_lists_and_dots() {
        covers!([SpecItem::McpToolsParserGrammar]);

        assert_eq!(
            tokens_of("(a . b)"),
            vec![
                Token::LParen,
                Token::Symbol("a".into()),
                Token::Dot,
                Token::Symbol("b".into()),
                Token::RParen,
            ]
        );
    }

    #[test]
    fn tokenize_quote_forms() {
        covers!([SpecItem::McpToolsParserGrammar]);

        assert_eq!(tokens_of("'x"), vec![Token::Quote, Token::Symbol("x".into())]);
        assert_eq!(
            tokens_of(",@x"),
            vec![Token::UnquoteSplicing, Token::Symbol("x".into())]
        );
        assert_eq!(
            tokens_of("`(,a)"),
            vec![
                Token::Quasiquote,
                Token::LParen,
                Token::Unquote,
                Token::Symbol("a".into()),
                Token::RParen,
            ]
        );
    }

    #[test]
    fn tokenize_string_escapes() {
        covers!([SpecItem::McpToolsParserStringEscapes]);

        assert_eq!(
            tokens_of(r#""a\\b\"c\nd\re\tf""#),
            vec![Token::String("a\\b\"c\nd\re\tf".into())]
        );
    }

    #[test]
    fn tokenize_invalid_escape_errors() {
        covers!([SpecItem::McpToolsParserStringEscapes]);

        let err = tokenize(r#""\x41""#).unwrap_err();
        match err {
            LexError::InvalidEscape { sequence, .. } => assert_eq!(sequence, "\\x"),
            other => panic!("expected InvalidEscape, got {:?}", other),
        }
    }

    #[test]
    fn tokenize_line_comment() {
        covers!([SpecItem::McpToolsParserComments]);

        let toks = tokens_of("; hello\n42");
        match &toks[0] {
            Token::Comment(c) => {
                assert_eq!(c.kind, CommentKind::Line);
                assert_eq!(c.text, " hello");
            }
            other => panic!("expected line comment, got {:?}", other),
        }
        assert_eq!(toks[1], Token::Integer(42));
    }

    #[test]
    fn tokenize_block_comment_nested() {
        covers!([SpecItem::McpToolsParserComments]);

        let toks = tokens_of("#| outer #| inner |# still |#");
        assert_eq!(toks.len(), 1);
        match &toks[0] {
            Token::Comment(c) => assert_eq!(c.kind, CommentKind::Block),
            other => panic!("expected block comment, got {:?}", other),
        }
    }

    #[test]
    fn integer_out_of_range_is_an_error() {
        covers!([SpecItem::McpToolsParserNumericTower]);

        // 2^65 doesn't fit in i64
        let err = tokenize("36893488147419103232").unwrap_err();
        match err {
            LexError::IntegerOutOfRange { .. } => {}
            other => panic!("expected IntegerOutOfRange, got {:?}", other),
        }
    }

    #[test]
    fn position_advances_through_multibyte() {
        covers!([SpecItem::McpToolsParserSpans]);

        let toks = tokenize("\"héllo\" x").unwrap();
        // first token spans the string
        assert_eq!(toks[0].span.start.column, 1);
        assert_eq!(toks[0].span.start.byte_offset, 0);
        // 'x' starts at column 9 (string is 7 chars: " h é l l o " -> 7 chars)
        // " h é l l o " = 7 chars (the two quotes plus 5 letters), then space, then x
        assert_eq!(toks[1].span.start.column, 9);
    }

    #[test]
    fn crlf_counts_as_one_line_break() {
        covers!([SpecItem::McpToolsParserSpans]);

        let toks = tokenize("a\r\nb").unwrap();
        assert_eq!(toks[0].span.start.line, 1);
        assert_eq!(toks[1].span.start.line, 2);
        assert_eq!(toks[1].span.start.column, 1);
    }
}
