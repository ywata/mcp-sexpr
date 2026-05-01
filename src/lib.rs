//! Comprehensive toolkit for MCP (Model Context Protocol) server and client development.
//!
//! This crate (formerly `mcp-sexpr`) provides a comprehensive set of utilities for building
//! MCP servers and clients in Rust. It includes:
//!
//! ## Core S-expression Utilities (always available)
//!
//! - **Parsing**: Position-tracking S-expression parser producing the new [`Value`]
//!   type. The previous `lexpr::Value`-based API is kept under deprecated `*_lexpr`
//!   names through 0.x; see `specs/migration/lexpr-deprecation.md`.
//! - **Keyword extraction**: Extract keyword arguments from tool-call forms.
//! - **TextRef handling**: Parse and render `(use "path")` file references.
//! - **Serialization**: Quote strings and render lists with proper escaping.
//!
//! ## Optional Features
//!
//! Enable features in your `Cargo.toml` to access additional functionality:
//!
//! - **`prompts`**: TOML configuration + markdown prompt building system
//! - **`interactive`**: Generic rustyline-based interactive line loop (sync)
//! - **`interactive-async`**: Async variant of interactive line loop (requires tokio)
//! - **`format`**: S-expression response formatting utilities
//! - **`extract`**: Type-safe argument extraction with type conversion
//! - **`persistence`**: SQLite-based tool call logging and observability
//! - **`log-viewer`**: Interactive CLI for querying tool call logs
//! - **`router`**: MCP server routing patterns with handler registration
//! - **`errors`**: Typed error patterns and examples using thiserror
//!
//! # Example
//!
//! ```rust
//! use mcp_tools::{parse_value, require_kw_str, parse_text_ref, TextRef};
//!
//! let input = r#"(tool :name "example" :spec (use "docs/spec.md"))"#;
//! let value = parse_value(input).unwrap();
//!
//! let name = require_kw_str(&value, "name").unwrap();
//! assert_eq!(name, "example");
//! ```
//!
//! # Feature Flags
//!
//! ```toml
//! [dependencies]
//! mcp-tools = { version = "0.2", features = ["prompts", "interactive", "format"] }
//! ```

#![forbid(unsafe_code)]
#![deny(missing_docs)]

// Generated traceability enum. Always available.
mod traceability_gen;
pub use traceability_gen::SpecItem;

// Position-tracking parser (replaces lexpr::Value as canonical representation).
pub mod parser;

pub use parser::{
    current_differential_mode, flush_discrepancy_dedup, parse_value_with_positions,
    set_differential_mode, set_discrepancy_dedup_capacity, Comment, CommentKind, DifferentialMode,
    Discrepancy, DiscrepancyInput, DiscrepancySink, LexprConversionError, ParseError, PathElement,
    Position, Span, Spanned, SpannedNode, StructuralPath, Value,
};

// Feature-gated modules
#[cfg(feature = "prompts")]
pub mod prompt;

#[cfg(feature = "interactive")]
pub mod interactive;

#[cfg(feature = "format")]
pub mod format;

#[cfg(feature = "extract")]
pub mod extract;

/// SQLite-based tool call persistence and logging.
#[cfg(feature = "persistence")]
pub mod persistence;

#[cfg(feature = "log-viewer")]
pub mod log_viewer;

#[cfg(feature = "router")]
pub mod router;

#[cfg(feature = "errors")]
pub mod errors;

use anyhow::{anyhow, Context, Result};

/// Parse a full S-expression string into the new [`Value`] representation.
///
/// In 0.3, this signature changed from `Result<lexpr::Value>` to `Result<Value>`.
/// Callers who need the previous behavior during the deprecation window can
/// switch to [`parse_value_lexpr`].
///
/// While differential validation is enabled (default in 0.3), this function
/// also runs `lexpr::from_str` and reports any structural divergence via the
/// configured [`DiscrepancySink`]. Differential reporting is non-fatal; the
/// returned `Result` is exactly what the new parser produced.
///
/// # Example
///
/// ```rust
/// use mcp_tools::parse_value;
///
/// let value = parse_value("(tool :key \"value\")").unwrap();
/// assert!(value.as_list().is_some());
/// ```
pub fn parse_value(input: &str) -> Result<Value> {
    let inner = parser::reader::parse_value(input);
    let _ = parser::differential::record_discrepancy_if_diverging(input, &inner);
    inner
        .map_err(anyhow::Error::new)
        .context("failed to parse s-expression")
}

/// Previous behavior of [`parse_value`]: returns `lexpr::Value`.
///
/// Kept through the deprecation window for callers that have not yet migrated.
#[deprecated(note = "use parse_value -> Value; lexpr::Value is removed in 1.0")]
pub fn parse_value_lexpr(input: &str) -> Result<lexpr::Value> {
    lexpr::from_str(input).context("failed to parse s-expression")
}

#[allow(deprecated)]
fn normalize_kw_lexpr(key: &lexpr::Value) -> Option<&str> {
    if let Some(sym) = key.as_symbol() {
        Some(sym.strip_prefix(':').unwrap_or(sym))
    } else if let Some(kw) = key.as_keyword() {
        Some(kw)
    } else {
        None
    }
}

/// Extract the raw [`Value`] for a keyword argument from a tool-call form.
///
/// Returns `Ok(None)` when the keyword is not present.
///
/// # Example
///
/// ```rust
/// use mcp_tools::{parse_value, get_kw_value};
///
/// let value = parse_value("(tool :key \"value\")").unwrap();
/// let kv = get_kw_value(&value, "key").unwrap();
/// assert!(kv.is_some());
/// ```
pub fn get_kw_value(root: &Value, key: &str) -> Result<Option<Value>> {
    let items = root
        .as_list()
        .ok_or_else(|| anyhow!("expected list (tool call form)"))?;
    let mut idx = 1; // skip head
    while idx < items.len() {
        let Value::Keyword(name) = &items[idx] else {
            break;
        };
        idx += 1;
        let v = items
            .get(idx)
            .ok_or_else(|| anyhow!("expected value after keyword :{}", name))?;
        if name == key {
            return Ok(Some(v.clone()));
        }
        idx += 1;
    }
    Ok(None)
}

/// Lexpr-based counterpart of [`get_kw_value`].
#[deprecated(note = "use the Value-based get_kw_value; lexpr::Value is removed in 1.0")]
#[allow(deprecated)]
pub fn get_kw_value_lexpr(root: &lexpr::Value, key: &str) -> Result<Option<lexpr::Value>> {
    let list = root
        .as_cons()
        .ok_or_else(|| anyhow!("expected list (tool call form)"))?;

    let mut cur = list.cdr();
    while let Some(cons) = cur.as_cons() {
        let k = cons.car();
        let Some(found) = normalize_kw_lexpr(k) else {
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
/// use mcp_tools::{parse_value, get_kw_str};
///
/// let value = parse_value("(tool :name \"example\")").unwrap();
/// assert_eq!(get_kw_str(&value, "name").unwrap(), Some("example".to_string()));
/// assert_eq!(get_kw_str(&value, "missing").unwrap(), None);
/// ```
pub fn get_kw_str(root: &Value, key: &str) -> Result<Option<String>> {
    match get_kw_value(root, key)? {
        None => Ok(None),
        Some(v) => v
            .as_str()
            .map(|s| Some(s.to_string()))
            .ok_or_else(|| anyhow!(":{} must be a string", key)),
    }
}

/// Lexpr-based counterpart of [`get_kw_str`].
#[deprecated(note = "use the Value-based get_kw_str; lexpr::Value is removed in 1.0")]
#[allow(deprecated)]
pub fn get_kw_str_lexpr(root: &lexpr::Value, key: &str) -> Result<Option<String>> {
    match get_kw_value_lexpr(root, key)? {
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
/// use mcp_tools::{parse_value, require_kw_str};
///
/// let value = parse_value("(tool :name \"example\")").unwrap();
/// assert_eq!(require_kw_str(&value, "name").unwrap(), "example");
/// ```
pub fn require_kw_str(root: &Value, key: &str) -> Result<String> {
    get_kw_str(root, key)?.ok_or_else(|| anyhow!("missing required keyword :{}", key))
}

/// Lexpr-based counterpart of [`require_kw_str`].
#[deprecated(note = "use the Value-based require_kw_str; lexpr::Value is removed in 1.0")]
#[allow(deprecated)]
pub fn require_kw_str_lexpr(root: &lexpr::Value, key: &str) -> Result<String> {
    get_kw_str_lexpr(root, key)?.ok_or_else(|| anyhow!("missing required keyword :{}", key))
}

/// Iterate over a proper list.
///
/// Returns an iterator over the list items, owned via clone. For non-list inputs
/// this returns an empty iterator (matching the previous lexpr-based behavior on
/// non-cons inputs).
///
/// # Example
///
/// ```rust
/// use mcp_tools::{parse_value, iter_list};
///
/// let value = parse_value("(a b c)").unwrap();
/// let items: Vec<_> = iter_list(&value).unwrap().collect();
/// assert_eq!(items.len(), 3);
/// ```
pub fn iter_list(value: &Value) -> Result<impl Iterator<Item = Value>> {
    let items: Vec<Value> = match value {
        Value::List(items) => items.clone(),
        Value::Nil => Vec::new(),
        Value::Pair(_) => {
            let mut out = Vec::new();
            let mut cur = value.clone();
            while let Value::Pair(pair) = cur {
                let (car, cdr) = *pair;
                out.push(car);
                cur = cdr;
            }
            out
        }
        _ => Vec::new(),
    };
    Ok(items.into_iter())
}

/// Lexpr-based counterpart of [`iter_list`].
#[deprecated(note = "use the Value-based iter_list; lexpr::Value is removed in 1.0")]
pub fn iter_list_lexpr(value: &lexpr::Value) -> Result<impl Iterator<Item = lexpr::Value>> {
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
/// use mcp_tools::{parse_value, parse_str_list};
///
/// let value = parse_value("(\"a\" \"b\" \"c\")").unwrap();
/// assert_eq!(parse_str_list(&value).unwrap(), vec!["a", "b", "c"]);
/// ```
pub fn parse_str_list(value: &Value) -> Result<Vec<String>> {
    let mut out = Vec::new();
    for item in iter_list(value)? {
        let s = item
            .as_str()
            .ok_or_else(|| anyhow!("expected string item in list"))?;
        out.push(s.to_string());
    }
    Ok(out)
}

/// Lexpr-based counterpart of [`parse_str_list`].
#[deprecated(note = "use the Value-based parse_str_list; lexpr::Value is removed in 1.0")]
#[allow(deprecated)]
pub fn parse_str_list_lexpr(value: &lexpr::Value) -> Result<Vec<String>> {
    let mut out = Vec::new();
    for item in iter_list_lexpr(value)? {
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
/// use mcp_tools::{parse_value, parse_text_ref, TextRef};
///
/// let literal = parse_value("\"hello\"").unwrap();
/// assert_eq!(parse_text_ref(&literal).unwrap(), TextRef::Literal("hello".to_string()));
///
/// let use_path = parse_value("(use \"docs/spec.md\")").unwrap();
/// assert_eq!(parse_text_ref(&use_path).unwrap(), TextRef::UsePath("docs/spec.md".to_string()));
/// ```
pub fn parse_text_ref(value: &Value) -> Result<TextRef> {
    if let Some(s) = value.as_str() {
        return Ok(TextRef::Literal(s.to_string()));
    }

    let items = value
        .as_list()
        .ok_or_else(|| anyhow!("expected string or (use \"path\")"))?;

    let head = items
        .first()
        .and_then(Value::as_symbol)
        .ok_or_else(|| anyhow!("expected (use \"path\")"))?;

    if head != "use" {
        return Err(anyhow!("expected (use \"path\")"));
    }

    let path = items
        .get(1)
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("(use ...) path must be a string"))?;

    Ok(TextRef::UsePath(path.to_string()))
}

/// Lexpr-based counterpart of [`parse_text_ref`].
#[deprecated(note = "use the Value-based parse_text_ref; lexpr::Value is removed in 1.0")]
pub fn parse_text_ref_lexpr(value: &lexpr::Value) -> Result<TextRef> {
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
/// use mcp_tools::{render_text_ref, TextRef};
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

/// Quote and escape a string for use inside an S-expression string literal.
///
/// Escaping policy (matches the subset of escapes `lexpr` accepts on parse, so
/// emitted strings round-trip through `parse_value`):
/// - `\` → `\\`
/// - `"` → `\"`
/// - LF (U+000A) → `\n`
/// - CR (U+000D) → `\r`
/// - TAB (U+0009) → `\t`
///
/// Other control characters (e.g. NUL, BEL) are passed through verbatim — lexpr
/// has no portable escape for them, and hex escapes are not accepted on parse.
///
/// # Example
///
/// ```rust
/// use mcp_tools::quote_str;
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
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
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
/// use mcp_tools::render_list;
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
        assert!(v.as_list().is_some());
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
        assert_eq!(quote_str("a\rb"), "\"a\\rb\"");
        assert_eq!(quote_str("a\tb"), "\"a\\tb\"");
    }

    #[test]
    fn quote_str_round_trips_through_lexpr() {
        for original in [
            "plain",
            "with \"quote\"",
            "back\\slash",
            "tab\there",
            "cr\rhere",
            "lf\nhere",
            "mixed \"\\\n\r\t end",
        ] {
            let quoted = quote_str(original);
            let parsed = lexpr::from_str(&quoted).expect("lexpr parse");
            assert_eq!(parsed.as_str(), Some(original), "round-trip failed for {:?}", original);
        }
    }
}
