//! Value types for the position-tracking parser.
//!
//! See `specs/parser/value-types.md` for the canonical specification.

use std::fmt;

/// Lightweight S-expression value used for construction, formatting, and equality.
///
/// `Value` carries no source-position metadata and no comment retention. The 95%
/// case (programmatic construction, walking, formatting) does not pay for span
/// machinery. Code that needs spans uses [`Spanned`].
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// `()` and `nil` both parse to this variant.
    Nil,
    /// `#t` and `#f`.
    Bool(bool),
    /// Decimal integer literal that fits in `i64`.
    Integer(i64),
    /// Floating literal.
    Float(f64),
    /// Quoted string literal.
    String(String),
    /// Identifier-like atom.
    Symbol(String),
    /// Keyword `:foo` — stored without the leading colon.
    Keyword(String),
    /// Proper list `(a b c)`.
    List(Vec<Value>),
    /// Genuine dotted pair `(a . b)`.
    Pair(Box<(Value, Value)>),
}

impl Eq for Value {}

impl Value {
    /// Returns `true` iff this is `Value::Nil`.
    pub fn is_nil(&self) -> bool {
        matches!(self, Value::Nil)
    }

    /// Returns `true` iff this is `Value::Bool(_)`.
    pub fn is_bool(&self) -> bool {
        matches!(self, Value::Bool(_))
    }

    /// Returns `true` iff this is `Value::Integer(_)`.
    pub fn is_integer(&self) -> bool {
        matches!(self, Value::Integer(_))
    }

    /// Returns `true` iff this is `Value::Float(_)`.
    pub fn is_float(&self) -> bool {
        matches!(self, Value::Float(_))
    }

    /// Returns `true` iff this is `Value::String(_)`.
    pub fn is_string(&self) -> bool {
        matches!(self, Value::String(_))
    }

    /// Returns `true` iff this is `Value::Symbol(_)`.
    pub fn is_symbol(&self) -> bool {
        matches!(self, Value::Symbol(_))
    }

    /// Returns `true` iff this is `Value::Keyword(_)`.
    pub fn is_keyword(&self) -> bool {
        matches!(self, Value::Keyword(_))
    }

    /// Returns `true` iff this is `Value::List(_)`.
    pub fn is_list(&self) -> bool {
        matches!(self, Value::List(_))
    }

    /// Returns `true` iff this is `Value::Pair(_)`.
    pub fn is_pair(&self) -> bool {
        matches!(self, Value::Pair(_))
    }

    /// Returns the inner `bool` for `Value::Bool`, else `None`.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Returns the inner `i64` for `Value::Integer`, else `None`.
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Integer(n) => Some(*n),
            _ => None,
        }
    }

    /// Returns the inner `f64` for `Value::Float`, else `None`. Does not coerce integers.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Float(x) => Some(*x),
            _ => None,
        }
    }

    /// Returns the inner string for `Value::String`, else `None`.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    /// Returns the inner symbol name for `Value::Symbol`, else `None`.
    pub fn as_symbol(&self) -> Option<&str> {
        match self {
            Value::Symbol(s) => Some(s),
            _ => None,
        }
    }

    /// Returns the inner keyword name for `Value::Keyword` (without leading colon), else `None`.
    pub fn as_keyword(&self) -> Option<&str> {
        match self {
            Value::Keyword(s) => Some(s),
            _ => None,
        }
    }

    /// Returns the slice for `Value::List`, else `None`. Does not match `Value::Pair`.
    pub fn as_list(&self) -> Option<&[Value]> {
        match self {
            Value::List(items) => Some(items),
            _ => None,
        }
    }

    /// Returns the `(car, cdr)` pair for `Value::Pair`, else `None`.
    pub fn as_pair(&self) -> Option<(&Value, &Value)> {
        match self {
            Value::Pair(pair) => Some((&pair.0, &pair.1)),
            _ => None,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Nil => f.write_str("()"),
            Value::Bool(true) => f.write_str("#t"),
            Value::Bool(false) => f.write_str("#f"),
            Value::Integer(n) => write!(f, "{}", n),
            Value::Float(x) => {
                if x.is_nan() {
                    f.write_str("+nan.0")
                } else if x.is_infinite() {
                    f.write_str(if *x > 0.0 { "+inf.0" } else { "-inf.0" })
                } else if *x == x.trunc() && x.is_finite() {
                    write!(f, "{:.1}", x)
                } else {
                    write!(f, "{}", x)
                }
            }
            Value::String(s) => write!(f, "{}", crate::quote_str(s)),
            Value::Symbol(s) => f.write_str(s),
            Value::Keyword(s) => write!(f, ":{}", s),
            Value::List(items) => {
                f.write_str("(")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        f.write_str(" ")?;
                    }
                    write!(f, "{}", item)?;
                }
                f.write_str(")")
            }
            Value::Pair(pair) => write!(f, "({} . {})", pair.0, pair.1),
        }
    }
}

/// Source position with 1-indexed line and column for human display.
///
/// See `specs/parser/source-positions.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Position {
    /// 1-indexed line number. Always `>= 1` for parser-emitted positions.
    pub line: u32,
    /// 1-indexed column counted in Unicode scalar values. Always `>= 1`.
    pub column: u32,
    /// 0-indexed UTF-8 byte offset into the source.
    pub byte_offset: u32,
}

impl Position {
    /// Position at the very start of the source.
    pub const START: Position = Position {
        line: 1,
        column: 1,
        byte_offset: 0,
    };

    /// Returns `(line, column)` 0-indexed for LSP `Position { line, character }` consumers.
    pub fn lsp(&self) -> (u32, u32) {
        (self.line.saturating_sub(1), self.column.saturating_sub(1))
    }
}

/// Inclusive-start, exclusive-end source range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    /// Position of the first character covered.
    pub start: Position,
    /// Position immediately after the last character covered.
    pub end: Position,
}

impl Span {
    /// Construct a span from two positions. No ordering check is performed; callers are
    /// expected to pass `start <= end`.
    pub const fn new(start: Position, end: Position) -> Self {
        Span { start, end }
    }

    /// Zero-width span at `pos`.
    pub const fn empty_at(pos: Position) -> Self {
        Span { start: pos, end: pos }
    }
}

/// Parser-recognized comment form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommentKind {
    /// `; ...` to end of line.
    Line,
    /// `#| ... |#` (may nest).
    Block,
}

/// Comment retained alongside a parsed value.
///
/// `text` is the comment body without the delimiter (no leading `;` or `#|`, no
/// trailing `|#`). `span` covers the entire delimiter-inclusive range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comment {
    /// Line or block.
    pub kind: CommentKind,
    /// Comment body without delimiters.
    pub text: String,
    /// Source range covered by the comment, including delimiters.
    pub span: Span,
}

/// Spanned counterpart to [`Value`] used by the position-tracking entry point.
///
/// Each node carries its `span`, plus comments attached to its leading and trailing
/// positions per the rules in `specs/parser/source-positions.md`.
#[derive(Debug, Clone, PartialEq)]
pub struct Spanned {
    /// Inner value with `List`/`Pair` recursing into `Spanned`.
    pub value: SpannedNode,
    /// Source range covered by this node.
    pub span: Span,
    /// Comments preceding this node, in source order.
    pub leading_comments: Vec<Comment>,
    /// Same-line comments following this node, in source order.
    pub trailing_comments: Vec<Comment>,
}

/// Inner enum for [`Spanned`] mirroring [`Value`] except that recursive variants
/// hold `Spanned` rather than `Value`.
#[derive(Debug, Clone, PartialEq)]
pub enum SpannedNode {
    /// `()` and `nil`.
    Nil,
    /// `#t` and `#f`.
    Bool(bool),
    /// Decimal integer literal.
    Integer(i64),
    /// Floating literal.
    Float(f64),
    /// Quoted string literal.
    String(String),
    /// Identifier-like atom.
    Symbol(String),
    /// Keyword without leading colon.
    Keyword(String),
    /// Proper list with each element retaining its span.
    List(Vec<Spanned>),
    /// Dotted pair with each side retaining its span.
    Pair(Box<(Spanned, Spanned)>),
}

impl Spanned {
    /// Strip spans and comments, producing the lightweight [`Value`] representation.
    pub fn into_value(self) -> Value {
        match self.value {
            SpannedNode::Nil => Value::Nil,
            SpannedNode::Bool(b) => Value::Bool(b),
            SpannedNode::Integer(n) => Value::Integer(n),
            SpannedNode::Float(x) => Value::Float(x),
            SpannedNode::String(s) => Value::String(s),
            SpannedNode::Symbol(s) => Value::Symbol(s),
            SpannedNode::Keyword(s) => Value::Keyword(s),
            SpannedNode::List(items) => {
                Value::List(items.into_iter().map(Spanned::into_value).collect())
            }
            SpannedNode::Pair(pair) => {
                let (car, cdr) = *pair;
                Value::Pair(Box::new((car.into_value(), cdr.into_value())))
            }
        }
    }

    /// Borrowing variant of [`Spanned::into_value`].
    pub fn to_value(&self) -> Value {
        match &self.value {
            SpannedNode::Nil => Value::Nil,
            SpannedNode::Bool(b) => Value::Bool(*b),
            SpannedNode::Integer(n) => Value::Integer(*n),
            SpannedNode::Float(x) => Value::Float(*x),
            SpannedNode::String(s) => Value::String(s.clone()),
            SpannedNode::Symbol(s) => Value::Symbol(s.clone()),
            SpannedNode::Keyword(s) => Value::Keyword(s.clone()),
            SpannedNode::List(items) => Value::List(items.iter().map(Spanned::to_value).collect()),
            SpannedNode::Pair(pair) => {
                Value::Pair(Box::new((pair.0.to_value(), pair.1.to_value())))
            }
        }
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
    fn value_construction_and_predicates() {
        covers!([SpecItem::McpToolsParserValueType]);

        assert!(Value::Nil.is_nil());
        assert!(Value::Bool(true).is_bool());
        assert_eq!(Value::Bool(true).as_bool(), Some(true));
        assert_eq!(Value::Integer(42).as_i64(), Some(42));
        assert_eq!(Value::Float(1.5).as_f64(), Some(1.5));
        assert_eq!(Value::String("hi".into()).as_str(), Some("hi"));
        assert_eq!(Value::Symbol("foo".into()).as_symbol(), Some("foo"));
        assert_eq!(Value::Keyword("k".into()).as_keyword(), Some("k"));

        let list = Value::List(vec![Value::Integer(1), Value::Integer(2)]);
        assert!(list.is_list());
        assert_eq!(list.as_list().map(|s| s.len()), Some(2));

        let pair = Value::Pair(Box::new((Value::Integer(1), Value::Integer(2))));
        assert!(pair.is_pair());
        assert!(pair.as_list().is_none());
    }

    #[test]
    fn spanned_into_value_strips_metadata() {
        covers!([SpecItem::McpToolsParserSpannedType]);

        let spanned = Spanned {
            value: SpannedNode::Integer(7),
            span: Span::empty_at(Position::START),
            leading_comments: vec![Comment {
                kind: CommentKind::Line,
                text: " hi".into(),
                span: Span::empty_at(Position::START),
            }],
            trailing_comments: vec![],
        };

        assert_eq!(spanned.into_value(), Value::Integer(7));
    }

    #[test]
    fn keyword_canonicalization_no_leading_colon() {
        covers!([SpecItem::McpToolsParserKeywordCanonicalization]);

        let kw = Value::Keyword("foo".into());
        assert_eq!(kw.as_keyword(), Some("foo"));
        assert_eq!(format!("{}", kw), ":foo");
    }

    #[test]
    fn list_representation_uses_vec() {
        covers!([SpecItem::McpToolsParserListRepresentation]);

        let xs = Value::List((0..1000).map(Value::Integer).collect());
        assert!(xs.is_list());
        assert_eq!(xs.as_list().unwrap().len(), 1000);
        assert_eq!(xs.as_list().unwrap()[500].as_i64(), Some(500));

        let pair = Value::Pair(Box::new((Value::Integer(1), Value::Integer(2))));
        assert!(pair.as_list().is_none());
        assert_eq!(pair.as_pair().map(|(a, _)| a.as_i64()), Some(Some(1)));
    }

    #[test]
    fn numeric_tower_stores_only_i64_and_f64() {
        covers!([SpecItem::McpToolsParserNumericTower]);

        let max = Value::Integer(i64::MAX);
        let min = Value::Integer(i64::MIN);
        assert_eq!(max.as_i64(), Some(i64::MAX));
        assert_eq!(min.as_i64(), Some(i64::MIN));

        let f = Value::Float(1.5);
        assert!(f.as_i64().is_none());
        assert_eq!(f.as_f64(), Some(1.5));
    }
}
