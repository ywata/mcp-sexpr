//! Constructor functions for [`Value`].
//!
//! See `specs/build/api.md` for the canonical specification.
//!
//! These are short, allocation-free helpers (apart from the necessary
//! `String::from`s) that make programmatic `Value` construction concise.
//! They produce the same `Value` shapes that `parse_value` produces, so
//! values built this way compare equal to parsed values of the same source.
//!
//! ```
//! use mcp_tools::build::{list, symbol, integer};
//! use mcp_tools::parse_value;
//!
//! let built = list(vec![symbol("define"), symbol("x"), integer(42)]);
//! let parsed = parse_value("(define x 42)").unwrap();
//! assert_eq!(built, parsed);
//! ```
//!
//! `cons` is intentionally literal: it always produces `Value::Pair`, never
//! a `Value::List`. Use [`list`] when you want a proper list.

use crate::Value;

/// Constructs a dotted pair `(car . cdr)`.
///
/// Always returns [`Value::Pair`]; does not collapse cons-list shapes into
/// [`Value::List`].
pub fn cons(car: Value, cdr: Value) -> Value {
    Value::Pair(Box::new((car, cdr)))
}

/// Constructs a proper list `(item1 item2 ... itemN)`.
///
/// `list(vec![])` returns [`Value::Nil`] — the empty list and `nil` are the
/// same value in this representation.
pub fn list(items: Vec<Value>) -> Value {
    if items.is_empty() {
        Value::Nil
    } else {
        Value::List(items)
    }
}

/// Constructs `Value::Keyword(name)`.
///
/// `name` is **without** the leading colon. `keyword("foo")` renders as `:foo`.
pub fn keyword(name: &str) -> Value {
    Value::Keyword(name.to_string())
}

/// Constructs `Value::Symbol(name)`.
pub fn symbol(name: &str) -> Value {
    Value::Symbol(name.to_string())
}

/// Constructs `Value::String(s)`. The string content is stored verbatim;
/// escaping is applied at render time.
pub fn string(s: &str) -> Value {
    Value::String(s.to_string())
}

/// Constructs `Value::Integer(n)`.
pub fn integer(n: i64) -> Value {
    Value::Integer(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{parse_value, SpecItem};

    macro_rules! covers {
        ([$($item:expr),* $(,)?]) => {
            { $( let _ = $item; )* }
        };
    }

    #[test]
    fn cons_produces_pair() {
        covers!([SpecItem::McpToolsBuildCons]);
        let v = cons(integer(1), integer(2));
        assert_eq!(v, Value::Pair(Box::new((Value::Integer(1), Value::Integer(2)))));
        assert!(v.is_pair());
        assert!(v.as_list().is_none());
    }

    #[test]
    fn cons_does_not_flatten_into_list() {
        covers!([SpecItem::McpToolsBuildCons]);
        // cons(a, cons(b, Nil)) is NOT equal to list(vec![a, b]) — see spec.
        let nested = cons(symbol("a"), cons(symbol("b"), Value::Nil));
        let proper = list(vec![symbol("a"), symbol("b")]);
        assert_ne!(nested, proper);
    }

    #[test]
    fn list_empty_returns_nil() {
        covers!([SpecItem::McpToolsBuildList]);
        assert_eq!(list(vec![]), Value::Nil);
    }

    #[test]
    fn list_nonempty_returns_list_variant() {
        covers!([SpecItem::McpToolsBuildList]);
        let v = list(vec![integer(1), integer(2), integer(3)]);
        assert!(v.is_list());
        assert_eq!(v.as_list().unwrap().len(), 3);
    }

    #[test]
    fn list_round_trip_with_parse() {
        covers!([SpecItem::McpToolsBuildList, SpecItem::McpToolsBuildSymbol, SpecItem::McpToolsBuildInteger]);
        let built = list(vec![symbol("define"), symbol("x"), integer(42)]);
        let parsed = parse_value("(define x 42)").unwrap();
        assert_eq!(built, parsed);
    }

    #[test]
    fn keyword_does_not_strip_colon() {
        covers!([SpecItem::McpToolsBuildKeyword]);
        // The function does not validate; it stores verbatim.
        let kw = keyword("name");
        assert_eq!(kw, Value::Keyword("name".to_string()));

        // Passing ":name" produces a wrong but representable Keyword(":name").
        // Documented in the spec; this test pins the behavior.
        let wrong = keyword(":name");
        assert_eq!(wrong, Value::Keyword(":name".to_string()));
    }

    #[test]
    fn symbol_stores_verbatim() {
        covers!([SpecItem::McpToolsBuildSymbol]);
        assert_eq!(symbol("foo"), Value::Symbol("foo".to_string()));
        assert_eq!(symbol("foo-bar"), Value::Symbol("foo-bar".to_string()));
    }

    #[test]
    fn string_does_not_quote() {
        covers!([SpecItem::McpToolsBuildString]);
        // string("hi") is Value::String("hi"), which Display-renders as "\"hi\"".
        let v = string("hi");
        assert_eq!(v, Value::String("hi".to_string()));
        assert_eq!(v.to_string(), "\"hi\"");
    }

    #[test]
    fn string_preserves_special_characters() {
        covers!([SpecItem::McpToolsBuildString]);
        let v = string("a\nb\\c");
        assert_eq!(v.as_str(), Some("a\nb\\c"));
    }

    #[test]
    fn integer_stores_i64() {
        covers!([SpecItem::McpToolsBuildInteger]);
        assert_eq!(integer(0), Value::Integer(0));
        assert_eq!(integer(i64::MAX), Value::Integer(i64::MAX));
        assert_eq!(integer(i64::MIN), Value::Integer(i64::MIN));
    }
}
