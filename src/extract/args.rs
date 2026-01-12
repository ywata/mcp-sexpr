//! Argument extraction functions with type conversion.
//!
//! These functions build on the existing `mcp-sexpr` keyword extraction
//! functions to provide type-safe argument parsing with clear error messages.

use anyhow::{Context, Result};
use crate::{get_kw_str, get_kw_value, parse_str_list, parse_value, require_kw_str};

/// Parse a tool call S-expression into a lexpr::Value.
///
/// # Example
///
/// ```rust
/// use mcp_tools::extract::parse_tool_call;
///
/// let value = parse_tool_call("(my-tool :arg \"value\")")?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn parse_tool_call(sexpr: &str) -> Result<lexpr::Value> {
    parse_value(sexpr).context("failed to parse tool call s-expression")
}

/// Extract a required string keyword argument.
///
/// Returns an error if the keyword is missing.
///
/// # Example
///
/// ```rust
/// use mcp_tools::extract::*;
///
/// let value = parse_tool_call("(tool :name \"example\")")?;
/// let name = require_string(&value, "name")?;
/// assert_eq!(name, "example");
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn require_string(value: &lexpr::Value, key: &str) -> Result<String> {
    require_kw_str(value, key).with_context(|| format!("Missing required keyword :{}", key))
}

/// Extract an optional string keyword argument.
///
/// Returns `Ok(None)` if the keyword is not present.
///
/// # Example
///
/// ```rust
/// use mcp_tools::extract::*;
///
/// let value = parse_tool_call("(tool :name \"example\")")?;
/// assert_eq!(get_string(&value, "name")?, Some("example".to_string()));
/// assert_eq!(get_string(&value, "missing")?, None);
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn get_string(value: &lexpr::Value, key: &str) -> Result<Option<String>> {
    get_kw_str(value, key).with_context(|| format!("Error extracting keyword :{}", key))
}

/// Extract a required keyword argument as raw lexpr::Value.
///
/// # Example
///
/// ```rust
/// use mcp_tools::extract::*;
///
/// let value = parse_tool_call("(tool :data (list 1 2 3))")?;
/// let data = require_value(&value, "data")?;
/// assert!(data.as_cons().is_some());
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn require_value(value: &lexpr::Value, key: &str) -> Result<lexpr::Value> {
    get_kw_value(value, key)?
        .ok_or_else(|| anyhow::anyhow!("Missing required keyword :{}", key))
}

/// Extract an optional keyword argument as raw lexpr::Value.
///
/// # Example
///
/// ```rust
/// use mcp_tools::extract::*;
///
/// let value = parse_tool_call("(tool :data (list 1 2 3))")?;
/// let data = get_value(&value, "data")?;
/// assert!(data.is_some());
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn get_value(value: &lexpr::Value, key: &str) -> Result<Option<lexpr::Value>> {
    get_kw_value(value, key)
}

/// Extract a string list from a lexpr::Value.
///
/// # Example
///
/// ```rust
/// use mcp_tools::extract::*;
///
/// let value = parse_tool_call("(tool :items (\"a\" \"b\" \"c\"))")?;
/// let items_value = require_value(&value, "items")?;
/// let items = extract_string_list(&items_value)?;
/// assert_eq!(items, vec!["a", "b", "c"]);
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn extract_string_list(value: &lexpr::Value) -> Result<Vec<String>> {
    parse_str_list(value).context("Failed to parse string list")
}

/// Extract an optional boolean keyword argument.
///
/// Accepts: `true`, `false`, `#t`, `#f`, `"true"`, `"false"`.
///
/// # Example
///
/// ```rust
/// use mcp_tools::extract::*;
///
/// let value = parse_tool_call("(tool :enabled true)")?;
/// assert_eq!(get_bool(&value, "enabled")?, Some(true));
/// assert_eq!(get_bool(&value, "missing")?, None);
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn get_bool(value: &lexpr::Value, key: &str) -> Result<Option<bool>> {
    match get_kw_value(value, key)? {
        None => Ok(None),
        Some(v) => {
            if let Some(b) = v.as_bool() {
                return Ok(Some(b));
            }
            if let Some(s) = v.as_str() {
                match s {
                    "true" => return Ok(Some(true)),
                    "false" => return Ok(Some(false)),
                    _ => {}
                }
            }
            if let Some(sym) = v.as_symbol() {
                match sym {
                    "true" => return Ok(Some(true)),
                    "false" => return Ok(Some(false)),
                    _ => {}
                }
            }
            Err(anyhow::anyhow!(
                ":{} must be a boolean (true/false), got: {:?}",
                key,
                v
            ))
        }
    }
}

/// Extract an optional integer keyword argument.
///
/// # Example
///
/// ```rust
/// use mcp_tools::extract::*;
///
/// let value = parse_tool_call("(tool :count 42)")?;
/// assert_eq!(get_int(&value, "count")?, Some(42));
/// assert_eq!(get_int(&value, "missing")?, None);
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn get_int(value: &lexpr::Value, key: &str) -> Result<Option<i64>> {
    match get_kw_value(value, key)? {
        None => Ok(None),
        Some(v) => {
            if let Some(n) = v.as_i64() {
                return Ok(Some(n));
            }
            if let Some(n) = v.as_u64() {
                return Ok(Some(n as i64));
            }
            if let Some(s) = v.as_str() {
                if let Ok(n) = s.parse::<i64>() {
                    return Ok(Some(n));
                }
            }
            Err(anyhow::anyhow!(":{} must be an integer, got: {:?}", key, v))
        }
    }
}

/// Extract an optional unsigned integer keyword argument.
///
/// # Example
///
/// ```rust
/// use mcp_tools::extract::*;
///
/// let value = parse_tool_call("(tool :limit 100)")?;
/// assert_eq!(get_uint(&value, "limit")?, Some(100));
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn get_uint(value: &lexpr::Value, key: &str) -> Result<Option<usize>> {
    match get_int(value, key)? {
        None => Ok(None),
        Some(n) if n >= 0 => Ok(Some(n as usize)),
        Some(n) => Err(anyhow::anyhow!(
            ":{} must be a non-negative integer, got: {}",
            key,
            n
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tool_call() {
        let value = parse_tool_call("(tool :arg \"value\")").unwrap();
        assert!(value.as_cons().is_some());
    }

    #[test]
    fn test_require_string() {
        let value = parse_tool_call("(tool :name \"test\")").unwrap();
        assert_eq!(require_string(&value, "name").unwrap(), "test");
        assert!(require_string(&value, "missing").is_err());
    }

    #[test]
    fn test_get_string() {
        let value = parse_tool_call("(tool :name \"test\")").unwrap();
        assert_eq!(get_string(&value, "name").unwrap(), Some("test".to_string()));
        assert_eq!(get_string(&value, "missing").unwrap(), None);
    }

    #[test]
    fn test_get_bool() {
        let value = parse_tool_call("(tool :enabled true :disabled false)").unwrap();
        assert_eq!(get_bool(&value, "enabled").unwrap(), Some(true));
        assert_eq!(get_bool(&value, "disabled").unwrap(), Some(false));
        assert_eq!(get_bool(&value, "missing").unwrap(), None);
    }

    #[test]
    fn test_get_int() {
        let value = parse_tool_call("(tool :count 42)").unwrap();
        assert_eq!(get_int(&value, "count").unwrap(), Some(42));
        assert_eq!(get_int(&value, "missing").unwrap(), None);
    }

    #[test]
    fn test_get_uint() {
        let value = parse_tool_call("(tool :limit 100)").unwrap();
        assert_eq!(get_uint(&value, "limit").unwrap(), Some(100));
    }

    #[test]
    fn test_extract_string_list() {
        let value = parse_tool_call("(tool :items (\"a\" \"b\" \"c\"))").unwrap();
        let items_value = require_value(&value, "items").unwrap();
        let items = extract_string_list(&items_value).unwrap();
        assert_eq!(items, vec!["a", "b", "c"]);
    }
}
