//! Form-shape pattern matching for `(head <pos...> :k v ...)` S-expressions.
//!
//! See `specs/match-form/api.md` for the canonical specification.
//!
//! ```
//! use mcp_tools::{parse_value, match_form};
//!
//! let v = parse_value("(define x 42 :doc \"counter\")").unwrap();
//! let m = match_form(&v, "define").unwrap();
//! assert_eq!(m.head(), "define");
//! assert_eq!(m.positional_at(0).unwrap().as_symbol(), Some("x"));
//! assert_eq!(m.positional_at(1).unwrap().as_i64(), Some(42));
//! assert_eq!(m.require_keyword("doc").unwrap().as_str(), Some("counter"));
//! ```

use crate::Value;
use anyhow::{anyhow, Result};

/// Borrowed view over a form matched against an expected head.
///
/// Constructed by [`match_form`]. Holds borrows into the original `Value`
/// and does not allocate copies of sub-values.
#[derive(Debug)]
pub struct FormMatch<'a> {
    head: &'a str,
    positional: Vec<&'a Value>,
    keywords: Vec<(&'a str, &'a Value)>,
}

impl<'a> FormMatch<'a> {
    /// The head symbol's name. Equal to the `expected_head` passed to
    /// [`match_form`] by construction.
    pub fn head(&self) -> &str {
        self.head
    }

    /// All positional arguments in source order.
    pub fn positional(&self) -> &[&'a Value] {
        &self.positional
    }

    /// The positional argument at `idx`, or an error mentioning the head.
    pub fn positional_at(&self, idx: usize) -> Result<&'a Value> {
        self.positional.get(idx).copied().ok_or_else(|| {
            anyhow!(
                "{}: missing positional argument at index {}",
                self.head,
                idx
            )
        })
    }

    /// The value for the first `:name` keyword, or `None` if absent.
    /// `name` is **without** the leading colon.
    pub fn keyword(&self, name: &str) -> Option<&'a Value> {
        self.keywords
            .iter()
            .find(|(k, _)| *k == name)
            .map(|(_, v)| *v)
    }

    /// The value for the first `:name` keyword, or an error if absent.
    pub fn require_keyword(&self, name: &str) -> Result<&'a Value> {
        self.keyword(name).ok_or_else(|| {
            anyhow!("{}: missing required keyword :{}", self.head, name)
        })
    }
}

/// Match a `Value` against the shape `(<expected_head> <pos...> :k v ...)`.
///
/// See `specs/match-form/api.md` for the full set of accepted shapes and
/// error cases.
pub fn match_form<'a>(value: &'a Value, expected_head: &str) -> Result<FormMatch<'a>> {
    let items = match value {
        Value::List(items) => items.as_slice(),
        Value::Nil => {
            return Err(anyhow!(
                "expected list form ({} ...), got ()",
                expected_head
            ))
        }
        other => {
            return Err(anyhow!(
                "expected list form (head ...), got {}",
                variant_name(other)
            ))
        }
    };

    let (head_value, rest) = items
        .split_first()
        .ok_or_else(|| anyhow!("expected list form ({} ...), got ()", expected_head))?;

    let head = head_value
        .as_symbol()
        .ok_or_else(|| anyhow!("expected symbol head in form, got {}", variant_name(head_value)))?;

    if head != expected_head {
        return Err(anyhow!(
            "expected form head '{}', got '{}'",
            expected_head,
            head
        ));
    }

    let mut positional: Vec<&'a Value> = Vec::new();
    let mut keywords: Vec<(&'a str, &'a Value)> = Vec::new();
    let mut last_kw: Option<&'a str> = None;
    let mut iter = rest.iter();

    // Phase 1: consume positional arguments (until first Keyword).
    while let Some(next) = iter.clone().next() {
        if next.is_keyword() {
            break;
        }
        positional.push(next);
        iter.next();
    }

    // Phase 2: consume keyword/value pairs.
    while let Some(item) = iter.next() {
        let kw = item.as_keyword().ok_or_else(|| {
            // A non-keyword in keyword phase means a positional follows a keyword.
            let prev = last_kw.unwrap_or("?");
            anyhow!("{}: positional argument follows keyword :{}", head, prev)
        })?;
        let val = iter
            .next()
            .ok_or_else(|| anyhow!("{}: keyword :{} has no value", head, kw))?;
        keywords.push((kw, val));
        last_kw = Some(kw);
    }

    Ok(FormMatch {
        head,
        positional,
        keywords,
    })
}

fn variant_name(v: &Value) -> &'static str {
    match v {
        Value::Nil => "nil",
        Value::Bool(_) => "bool",
        Value::Integer(_) => "integer",
        Value::Float(_) => "float",
        Value::String(_) => "string",
        Value::Symbol(_) => "symbol",
        Value::Keyword(_) => "keyword",
        Value::List(_) => "list",
        Value::Pair(_) => "pair",
    }
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
    fn match_form_accepts_basic_shape() {
        covers!([SpecItem::McpToolsMatchFormMatchForm]);
        let v = parse_value("(define x 42)").unwrap();
        let m = match_form(&v, "define").unwrap();
        assert_eq!(m.head(), "define");
        assert_eq!(m.positional().len(), 2);
    }

    #[test]
    fn match_form_accepts_no_args() {
        covers!([SpecItem::McpToolsMatchFormMatchForm]);
        let v = parse_value("(reset)").unwrap();
        let m = match_form(&v, "reset").unwrap();
        assert_eq!(m.positional().len(), 0);
        assert!(m.keyword("anything").is_none());
    }

    #[test]
    fn match_form_accepts_keyword_only() {
        covers!([SpecItem::McpToolsMatchFormMatchForm, SpecItem::McpToolsMatchFormKeyword]);
        let v = parse_value(r#"(config :name "x" :value 1)"#).unwrap();
        let m = match_form(&v, "config").unwrap();
        assert_eq!(m.positional().len(), 0);
        assert_eq!(m.keyword("name").unwrap().as_str(), Some("x"));
        assert_eq!(m.keyword("value").unwrap().as_i64(), Some(1));
    }

    #[test]
    fn match_form_accepts_positional_then_keyword() {
        covers!([SpecItem::McpToolsMatchFormMatchForm, SpecItem::McpToolsMatchFormPositional, SpecItem::McpToolsMatchFormKeyword]);
        let v = parse_value(r#"(define foo 42 :doc "counter" :max 100)"#).unwrap();
        let m = match_form(&v, "define").unwrap();
        assert_eq!(m.positional().len(), 2);
        assert_eq!(m.positional_at(0).unwrap().as_symbol(), Some("foo"));
        assert_eq!(m.positional_at(1).unwrap().as_i64(), Some(42));
        assert_eq!(m.require_keyword("doc").unwrap().as_str(), Some("counter"));
        assert_eq!(m.require_keyword("max").unwrap().as_i64(), Some(100));
    }

    #[test]
    fn form_match_borrows_lifetime() {
        covers!([SpecItem::McpToolsMatchFormFormMatchType, SpecItem::McpToolsMatchFormLifetime]);
        let v = parse_value("(f 1)").unwrap();
        let m = match_form(&v, "f").unwrap();
        // Returned reference outlives `m` and is bounded by `v`.
        let pos = m.positional_at(0).unwrap();
        drop(m);
        assert_eq!(pos.as_i64(), Some(1));
    }

    #[test]
    fn head_returns_expected_symbol() {
        covers!([SpecItem::McpToolsMatchFormHead]);
        let v = parse_value("(my-tool)").unwrap();
        let m = match_form(&v, "my-tool").unwrap();
        assert_eq!(m.head(), "my-tool");
    }

    #[test]
    fn positional_at_errors_for_missing_index() {
        covers!([SpecItem::McpToolsMatchFormPositional]);
        let v = parse_value("(f 1)").unwrap();
        let m = match_form(&v, "f").unwrap();
        let err = m.positional_at(5).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("f:"), "got: {}", msg);
        assert!(msg.contains("index 5"), "got: {}", msg);
    }

    #[test]
    fn keyword_returns_first_match_for_duplicates() {
        covers!([SpecItem::McpToolsMatchFormKeyword]);
        let v = parse_value(r#"(f :k "first" :k "second")"#).unwrap();
        let m = match_form(&v, "f").unwrap();
        assert_eq!(m.keyword("k").unwrap().as_str(), Some("first"));
    }

    #[test]
    fn require_keyword_errors_when_missing() {
        covers!([SpecItem::McpToolsMatchFormKeyword]);
        let v = parse_value("(f :a 1)").unwrap();
        let m = match_form(&v, "f").unwrap();
        let err = m.require_keyword("missing").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("f:"));
        assert!(msg.contains(":missing"));
    }

    #[test]
    fn keyword_name_must_not_have_leading_colon() {
        covers!([SpecItem::McpToolsMatchFormKeyword]);
        let v = parse_value("(f :k 1)").unwrap();
        let m = match_form(&v, "f").unwrap();
        // The kwarg is :k, stored as Keyword("k"). Looking up ":k" must miss.
        assert!(m.keyword(":k").is_none());
        assert!(m.keyword("k").is_some());
    }

    #[test]
    fn error_when_input_is_not_a_list() {
        covers!([SpecItem::McpToolsMatchFormErrorCases]);
        let v = parse_value("42").unwrap();
        let err = match_form(&v, "f").unwrap_err();
        assert!(err.to_string().contains("expected list"));
        assert!(err.to_string().contains("integer"));
    }

    #[test]
    fn error_when_input_is_pair() {
        covers!([SpecItem::McpToolsMatchFormErrorCases]);
        let v = parse_value("(a . b)").unwrap();
        let err = match_form(&v, "f").unwrap_err();
        assert!(err.to_string().contains("pair"));
    }

    #[test]
    fn error_when_input_is_empty_list() {
        covers!([SpecItem::McpToolsMatchFormErrorCases]);
        let v = parse_value("()").unwrap();
        let err = match_form(&v, "f").unwrap_err();
        assert!(err.to_string().contains("()"));
    }

    #[test]
    fn error_when_head_is_not_symbol() {
        covers!([SpecItem::McpToolsMatchFormErrorCases]);
        let v = parse_value("(42 1 2)").unwrap();
        let err = match_form(&v, "f").unwrap_err();
        assert!(err.to_string().contains("symbol head"));
        assert!(err.to_string().contains("integer"));
    }

    #[test]
    fn error_when_head_does_not_match() {
        covers!([SpecItem::McpToolsMatchFormErrorCases]);
        let v = parse_value("(actual 1)").unwrap();
        let err = match_form(&v, "expected").unwrap_err();
        assert!(err.to_string().contains("'expected'"));
        assert!(err.to_string().contains("'actual'"));
    }

    #[test]
    fn error_when_keyword_has_no_value() {
        covers!([SpecItem::McpToolsMatchFormErrorCases]);
        let v = parse_value("(f :k)").unwrap();
        let err = match_form(&v, "f").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains(":k"), "got: {}", msg);
        assert!(msg.contains("no value"), "got: {}", msg);
    }

    #[test]
    fn error_when_positional_follows_keyword() {
        covers!([SpecItem::McpToolsMatchFormErrorCases]);
        let v = parse_value(r#"(f :a 1 oops)"#).unwrap();
        let err = match_form(&v, "f").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("positional"), "got: {}", msg);
        assert!(msg.contains(":a"), "got: {}", msg);
    }
}
