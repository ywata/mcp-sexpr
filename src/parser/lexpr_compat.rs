//! Bidirectional conversion between [`Value`] and [`lexpr::Value`].
//!
//! See `specs/parser/api.md` and `specs/migration/lexpr-deprecation.md`.

use std::fmt;

use super::types::Value;

/// Failure converting a [`lexpr::Value`] into [`Value`].
///
/// The reverse direction ([`Value`] → [`lexpr::Value`]) is total and never errors.
#[derive(Debug, Clone, PartialEq)]
pub enum LexprConversionError {
    /// `lexpr::Number` represented an integer outside `i64::MIN..=i64::MAX`.
    BignumOutOfRange,
    /// `lexpr::Value::Char` has no [`Value`] counterpart.
    UnsupportedChar(char),
    /// `lexpr::Value::Bytes` has no [`Value`] counterpart.
    UnsupportedBytes,
    /// `lexpr::Value::Vector` has no [`Value`] counterpart.
    UnsupportedVector,
    /// Reserved for future lexpr versions that introduce a rational variant.
    UnsupportedRational {
        /// Numerator of the rational.
        num: i64,
        /// Denominator of the rational.
        den: i64,
    },
    /// Reserved for future lexpr versions that introduce a complex variant.
    UnsupportedComplex,
}

impl fmt::Display for LexprConversionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LexprConversionError::BignumOutOfRange => {
                f.write_str("integer outside i64 range cannot be converted to mcp-tools Value")
            }
            LexprConversionError::UnsupportedChar(c) => {
                write!(f, "character literal {:?} has no Value counterpart", c)
            }
            LexprConversionError::UnsupportedBytes => {
                f.write_str("byte vector has no Value counterpart")
            }
            LexprConversionError::UnsupportedVector => {
                f.write_str("vector has no Value counterpart")
            }
            LexprConversionError::UnsupportedRational { num, den } => {
                write!(f, "rational {}/{} has no Value counterpart", num, den)
            }
            LexprConversionError::UnsupportedComplex => {
                f.write_str("complex number has no Value counterpart")
            }
        }
    }
}

impl std::error::Error for LexprConversionError {}

impl From<Value> for lexpr::Value {
    fn from(value: Value) -> lexpr::Value {
        match value {
            Value::Nil => lexpr::Value::Null,
            Value::Bool(b) => lexpr::Value::Bool(b),
            Value::Integer(n) => lexpr::Value::Number(n.into()),
            Value::Float(x) => {
                let n = lexpr::Number::from_f64(x).unwrap_or_else(|| 0i64.into());
                lexpr::Value::Number(n)
            }
            Value::String(s) => lexpr::Value::String(s.into_boxed_str()),
            Value::Symbol(s) => lexpr::Value::Symbol(s.into_boxed_str()),
            Value::Keyword(s) => lexpr::Value::Keyword(s.into_boxed_str()),
            Value::List(items) => list_to_lexpr(items),
            Value::Pair(pair) => {
                let (car, cdr) = *pair;
                lexpr::Value::cons(lexpr::Value::from(car), lexpr::Value::from(cdr))
            }
        }
    }
}

fn list_to_lexpr(items: Vec<Value>) -> lexpr::Value {
    let mut acc = lexpr::Value::Null;
    for item in items.into_iter().rev() {
        acc = lexpr::Value::cons(lexpr::Value::from(item), acc);
    }
    acc
}

impl TryFrom<lexpr::Value> for Value {
    type Error = LexprConversionError;

    fn try_from(value: lexpr::Value) -> Result<Self, Self::Error> {
        match value {
            lexpr::Value::Nil => Ok(Value::Nil),
            lexpr::Value::Null => Ok(Value::Nil),
            lexpr::Value::Bool(b) => Ok(Value::Bool(b)),
            lexpr::Value::Number(n) => convert_number(&n),
            lexpr::Value::Char(c) => Err(LexprConversionError::UnsupportedChar(c)),
            lexpr::Value::String(s) => Ok(Value::String(s.into_string())),
            lexpr::Value::Symbol(s) => Ok(Value::Symbol(s.into_string())),
            lexpr::Value::Keyword(s) => Ok(Value::Keyword(s.into_string())),
            lexpr::Value::Bytes(_) => Err(LexprConversionError::UnsupportedBytes),
            lexpr::Value::Cons(_) => convert_cons_value(value),
            lexpr::Value::Vector(_) => Err(LexprConversionError::UnsupportedVector),
        }
    }
}

fn convert_number(n: &lexpr::Number) -> Result<Value, LexprConversionError> {
    if let Some(i) = n.as_i64() {
        return Ok(Value::Integer(i));
    }
    if n.as_u64().is_some() {
        // Positive integer beyond i64 range.
        return Err(LexprConversionError::BignumOutOfRange);
    }
    if let Some(f) = n.as_f64() {
        return Ok(Value::Float(f));
    }
    Err(LexprConversionError::BignumOutOfRange)
}

fn convert_cons_value(value: lexpr::Value) -> Result<Value, LexprConversionError> {
    let mut items: Vec<Value> = Vec::new();
    let mut current = value;
    loop {
        match current {
            lexpr::Value::Cons(cons) => {
                let (car, cdr) = cons.into_pair();
                items.push(Value::try_from(car)?);
                current = cdr;
            }
            lexpr::Value::Null | lexpr::Value::Nil => {
                return Ok(Value::List(items));
            }
            other => {
                let tail = Value::try_from(other)?;
                if items.is_empty() {
                    // Should be impossible — we entered through a Cons.
                    return Ok(tail);
                }
                if items.len() == 1 {
                    let head = items.pop().unwrap();
                    return Ok(Value::Pair(Box::new((head, tail))));
                }
                let mut acc = tail;
                while let Some(head) = items.pop() {
                    acc = Value::Pair(Box::new((head, acc)));
                }
                return Ok(acc);
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

    fn parse_lexpr(input: &str) -> lexpr::Value {
        lexpr::from_str(input).unwrap()
    }

    #[test]
    fn value_to_lexpr_round_trip_atoms() {
        covers!([SpecItem::McpToolsParserLexprConversion]);

        let cases: Vec<Value> = vec![
            Value::Nil,
            Value::Bool(true),
            Value::Bool(false),
            Value::Integer(42),
            Value::Integer(-7),
            Value::Float(1.5),
            Value::String("hi".into()),
            Value::Symbol("foo".into()),
            Value::Keyword("name".into()),
        ];
        for v in cases {
            let lexpr_v = lexpr::Value::from(v.clone());
            let back = Value::try_from(lexpr_v).unwrap();
            assert_eq!(back, v);
        }
    }

    #[test]
    fn value_to_lexpr_proper_list() {
        covers!([SpecItem::McpToolsParserLexprConversion]);

        let v = Value::List(vec![
            Value::Integer(1),
            Value::Integer(2),
            Value::Integer(3),
        ]);
        let lexpr_v = lexpr::Value::from(v.clone());
        assert!(lexpr_v.is_list());
        let back = Value::try_from(lexpr_v).unwrap();
        assert_eq!(back, v);
    }

    #[test]
    fn value_to_lexpr_dotted_pair() {
        covers!([SpecItem::McpToolsParserLexprConversion, SpecItem::McpToolsParserListRepresentation]);

        let v = Value::Pair(Box::new((Value::Integer(1), Value::Integer(2))));
        let lexpr_v = lexpr::Value::from(v.clone());
        assert!(lexpr_v.is_cons());
        let back = Value::try_from(lexpr_v).unwrap();
        assert_eq!(back, v);
    }

    #[test]
    fn lexpr_to_value_handles_proper_list() {
        covers!([SpecItem::McpToolsParserLexprConversion]);

        let lexpr_v = parse_lexpr("(1 2 3)");
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
    fn lexpr_to_value_handles_dotted_pair() {
        covers!([SpecItem::McpToolsParserLexprConversion]);

        let lexpr_v = parse_lexpr("(1 . 2)");
        let v = Value::try_from(lexpr_v).unwrap();
        assert_eq!(
            v,
            Value::Pair(Box::new((Value::Integer(1), Value::Integer(2))))
        );
    }

    #[test]
    fn lexpr_to_value_errors_on_bignum() {
        covers!([SpecItem::McpToolsMigrationNumericTowerLoss]);

        // 2^63 (one past i64::MAX) appears as PosInt(u64) in lexpr::Number.
        let n = lexpr::Number::from(i64::MAX as u64 + 1);
        let lexpr_v = lexpr::Value::Number(n);
        match Value::try_from(lexpr_v) {
            Err(LexprConversionError::BignumOutOfRange) => {}
            other => panic!("expected BignumOutOfRange, got {:?}", other),
        }
    }

    #[test]
    fn lexpr_to_value_errors_on_char() {
        covers!([SpecItem::McpToolsMigrationLexprConversionLossy]);

        let lexpr_v = lexpr::Value::Char('a');
        match Value::try_from(lexpr_v) {
            Err(LexprConversionError::UnsupportedChar('a')) => {}
            other => panic!("expected UnsupportedChar('a'), got {:?}", other),
        }
    }

    #[test]
    fn lexpr_to_value_errors_on_bytes() {
        covers!([SpecItem::McpToolsMigrationLexprConversionLossy]);

        let lexpr_v = lexpr::Value::bytes(vec![1u8, 2, 3]);
        match Value::try_from(lexpr_v) {
            Err(LexprConversionError::UnsupportedBytes) => {}
            other => panic!("expected UnsupportedBytes, got {:?}", other),
        }
    }

    #[test]
    fn keyword_round_trip_strips_leading_colon() {
        covers!([SpecItem::McpToolsParserKeywordCanonicalization]);

        let v = Value::Keyword("name".into());
        let lexpr_v = lexpr::Value::from(v.clone());
        assert!(lexpr_v.is_keyword());
        let back = Value::try_from(lexpr_v).unwrap();
        assert_eq!(back, v);
    }
}
