//! S-expression utilities for MCP (Model Context Protocol) tool development.
//!
//! This crate provides common utilities for working with S-expressions in MCP tools:
//!
//! - **Parsing**: Parse S-expression strings using `lexpr`
//! - **Keyword extraction**: Extract keyword arguments from tool-call forms
//! - **TextRef handling**: Parse and render `(use "path")` file references
//! - **Serialization**: Quote strings and render lists with proper escaping
//!
//! # Example
//!
//! ```rust
//! use mcp_sexpr::{parse_value, require_kw_str, parse_text_ref, TextRef};
//!
//! let input = r#"(tool :name "example" :spec (use "docs/spec.md"))"#;
//! let value = parse_value(input).unwrap();
//!
//! let name = require_kw_str(&value, "name").unwrap();
//! assert_eq!(name, "example");
//! ```

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use anyhow::{anyhow, Context, Result};

/// Parse a full S-expression string into a `lexpr::Value`.
///
/// # Example
///
/// ```rust
/// use mcp_sexpr::parse_value;
///
/// let value = parse_value("(tool :key \"value\")").unwrap();
/// assert!(value.as_cons().is_some());
/// ```
pub fn parse_value(input: &str) -> Result<lexpr::Value> {
    lexpr::from_str(input).context("failed to parse s-expression")
}

fn normalize_kw(key: &lexpr::Value) -> Option<&str> {
    if let Some(sym) = key.as_symbol() {
        Some(sym.strip_prefix(':').unwrap_or(sym))
    } else if let Some(kw) = key.as_keyword() {
        Some(kw)
    } else {
        None
    }
}

/// Extract the raw `lexpr::Value` for a keyword argument from a tool-call form.
///
/// Returns `Ok(None)` when the keyword is not present.
///
/// # Example
///
/// ```rust
/// use mcp_sexpr::{parse_value, get_kw_value};
///
/// let value = parse_value("(tool :key \"value\")").unwrap();
/// let kv = get_kw_value(&value, "key").unwrap();
/// assert!(kv.is_some());
/// ```
pub fn get_kw_value(root: &lexpr::Value, key: &str) -> Result<Option<lexpr::Value>> {
    let list = root
        .as_cons()
        .ok_or_else(|| anyhow!("expected list (tool call form)"))?;

    let mut cur = list.cdr();
    while let Some(cons) = cur.as_cons() {
        let k = cons.car();
        let Some(found) = normalize_kw(k) else {
            break;
        };

        cur = cons.cdr();
        let val_cons = cur
            .as_cons()
            .ok_or_else(|| anyhow!("expected value after keyword :{}", found))?;
        let v = val_cons.car();

        if found == key {
            return Ok(Some(v.clone()));
        }

        cur = val_cons.cdr();
    }

    Ok(None)
}

/// Extract a keyword argument as a string.
///
/// Returns `Ok(None)` when the keyword is not present.
///
/// # Example
///
/// ```rust
/// use mcp_sexpr::{parse_value, get_kw_str};
///
/// let value = parse_value("(tool :name \"example\")").unwrap();
/// assert_eq!(get_kw_str(&value, "name").unwrap(), Some("example".to_string()));
/// assert_eq!(get_kw_str(&value, "missing").unwrap(), None);
/// ```
pub fn get_kw_str(root: &lexpr::Value, key: &str) -> Result<Option<String>> {
    match get_kw_value(root, key)? {
        None => Ok(None),
        Some(v) => v
            .as_str()
            .map(|s| Some(s.to_string()))
            .ok_or_else(|| anyhow!(":{} must be a string", key)),
    }
}

/// Extract a required keyword argument as a string.
///
/// Errors when missing.
///
/// # Example
///
/// ```rust
/// use mcp_sexpr::{parse_value, require_kw_str};
///
/// let value = parse_value("(tool :name \"example\")").unwrap();
/// assert_eq!(require_kw_str(&value, "name").unwrap(), "example");
/// ```
pub fn require_kw_str(root: &lexpr::Value, key: &str) -> Result<String> {
    get_kw_str(root, key)?.ok_or_else(|| anyhow!("missing required keyword :{}", key))
}

/// Iterate over a proper list.
///
/// Returns an error if `value` is not a list.
///
/// # Example
///
/// ```rust
/// use mcp_sexpr::{parse_value, iter_list};
///
/// let value = parse_value("(a b c)").unwrap();
/// let items: Vec<_> = iter_list(&value).unwrap().collect();
/// assert_eq!(items.len(), 3);
/// ```
pub fn iter_list(value: &lexpr::Value) -> Result<impl Iterator<Item = lexpr::Value>> {
    let mut out: Vec<lexpr::Value> = Vec::new();
    let mut cur = value;

    while let Some(cons) = cur.as_cons() {
        out.push(cons.car().clone());
        cur = cons.cdr();
    }

    Ok(out.into_iter())
}

/// Parse a proper list of strings into `Vec<String>`.
///
/// # Example
///
/// ```rust
/// use mcp_sexpr::{parse_value, parse_str_list};
///
/// let value = parse_value("(\"a\" \"b\" \"c\")").unwrap();
/// assert_eq!(parse_str_list(&value).unwrap(), vec!["a", "b", "c"]);
/// ```
pub fn parse_str_list(value: &lexpr::Value) -> Result<Vec<String>> {
    let mut out = Vec::new();
    for item in iter_list(value)? {
        let s = item
            .as_str()
            .ok_or_else(|| anyhow!("expected string item in list"))?;
        out.push(s.to_string());
    }
    Ok(out)
}

/// Generic representation for values that are either a literal string or a `(use "path")` reference.
///
/// This is commonly used in MCP tools for specification fields that can either be
/// inline text or a file reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextRef {
    /// A literal string value.
    Literal(String),
    /// A file path reference from `(use "path")`.
    UsePath(String),
}

/// Parse either a string literal or `(use "path")`.
///
/// # Example
///
/// ```rust
/// use mcp_sexpr::{parse_value, parse_text_ref, TextRef};
///
/// let literal = parse_value("\"hello\"").unwrap();
/// assert_eq!(parse_text_ref(&literal).unwrap(), TextRef::Literal("hello".to_string()));
///
/// let use_path = parse_value("(use \"docs/spec.md\")").unwrap();
/// assert_eq!(parse_text_ref(&use_path).unwrap(), TextRef::UsePath("docs/spec.md".to_string()));
/// ```
pub fn parse_text_ref(value: &lexpr::Value) -> Result<TextRef> {
    if let Some(s) = value.as_str() {
        return Ok(TextRef::Literal(s.to_string()));
    }

    let list = value
        .as_cons()
        .ok_or_else(|| anyhow!("expected string or (use \"path\")"))?;

    let head = list
        .car()
        .as_symbol()
        .ok_or_else(|| anyhow!("expected (use \"path\")"))?;

    if head != "use" {
        return Err(anyhow!("expected (use \"path\")"));
    }

    let arg_cons = list
        .cdr()
        .as_cons()
        .ok_or_else(|| anyhow!("(use ...) missing argument"))?;

    let path = arg_cons
        .car()
        .as_str()
        .ok_or_else(|| anyhow!("(use ...) path must be a string"))?;

    Ok(TextRef::UsePath(path.to_string()))
}

/// Render a `TextRef` back to an S-expression fragment.
///
/// # Example
///
/// ```rust
/// use mcp_sexpr::{render_text_ref, TextRef};
///
/// let literal = TextRef::Literal("hello".to_string());
/// assert_eq!(render_text_ref(&literal), "\"hello\"");
///
/// let use_path = TextRef::UsePath("docs/spec.md".to_string());
/// assert_eq!(render_text_ref(&use_path), "(use \"docs/spec.md\")");
/// ```
pub fn render_text_ref(value: &TextRef) -> String {
    match value {
        TextRef::Literal(s) => quote_str(s),
        TextRef::UsePath(path) => format!("(use {})", quote_str(path)),
    }
}

/// Quote and minimally escape a string for use inside an S-expression string literal.
///
/// Escaping policy:
/// - `\` → `\\`
/// - `"` → `\"`
/// - `\n` → `\n` (literal backslash-n)
///
/// # Example
///
/// ```rust
/// use mcp_sexpr::quote_str;
///
/// assert_eq!(quote_str("hello"), "\"hello\"");
/// assert_eq!(quote_str("say \"hi\""), "\"say \\\"hi\\\"\"");
/// ```
pub fn quote_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

/// Render a space-separated list from already-rendered items.
///
/// # Example
///
/// ```rust
/// use mcp_sexpr::render_list;
///
/// let items = vec!["\"a\"".to_string(), "\"b\"".to_string()];
/// assert_eq!(render_list(items), "\"a\" \"b\"");
/// ```
pub fn render_list(items: impl IntoIterator<Item = String>) -> String {
    items.into_iter().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_value_parses() {
        let v = parse_value("(tool :a \"b\")").unwrap();
        assert!(v.as_cons().is_some());
    }

    #[test]
    fn kw_extraction_string() {
        let v = parse_value("(tool :name \"abc\")").unwrap();
        assert_eq!(require_kw_str(&v, "name").unwrap(), "abc");
        assert_eq!(get_kw_str(&v, "missing").unwrap(), None);
    }

    #[test]
    fn kw_extraction_wrong_type() {
        let v = parse_value("(tool :name (x))").unwrap();
        assert!(get_kw_str(&v, "name").is_err());
    }

    #[test]
    fn parse_str_list_ok() {
        let v = parse_value("(\"a\" \"b\")").unwrap();
        assert_eq!(parse_str_list(&v).unwrap(), vec!["a", "b"]);
    }

    #[test]
    fn text_ref_literal_and_use() {
        let lit = parse_value("\"hello\"").unwrap();
        assert_eq!(
            parse_text_ref(&lit).unwrap(),
            TextRef::Literal("hello".to_string())
        );

        let usev = parse_value("(use \"docs/spec.md\")").unwrap();
        assert_eq!(
            parse_text_ref(&usev).unwrap(),
            TextRef::UsePath("docs/spec.md".to_string())
        );

        let rendered = render_text_ref(&TextRef::UsePath("x".to_string()));
        assert_eq!(rendered, "(use \"x\")");
    }

    #[test]
    fn quote_str_escapes() {
        assert_eq!(quote_str("a\"b"), "\"a\\\"b\"");
        assert_eq!(quote_str("a\\b"), "\"a\\\\b\"");
        assert_eq!(quote_str("a\nb"), "\"a\\nb\"");
    }
}
