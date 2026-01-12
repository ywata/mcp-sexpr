//! Response formatting functions for MCP tool responses.
//!
//! These functions build on the existing `quote_str()` and `render_list()`
//! functions to provide convenient response builders for common MCP patterns.

use crate::{quote_str, render_list};

/// Format a success response with keyword arguments.
///
/// # Example
///
/// ```rust
/// use mcp_tools::format::format_success;
///
/// let response = format_success(&[
///     ("internal-id", "uuid-123"),
///     ("status", "complete"),
/// ]);
/// assert_eq!(response, "(success :internal-id \"uuid-123\" :status \"complete\")");
/// ```
pub fn format_success(fields: &[(&str, &str)]) -> String {
    let field_strs: Vec<String> = fields
        .iter()
        .map(|(key, value)| format!(":{} {}", key, quote_str(value)))
        .collect();
    format!("(success {})", field_strs.join(" "))
}

/// Format an error response.
///
/// # Example
///
/// ```rust
/// use mcp_tools::format::format_error;
///
/// let response = format_error("Resource not found");
/// assert_eq!(response, "(error \"Resource not found\")");
/// ```
pub fn format_error(message: &str) -> String {
    format!("(error {})", quote_str(message))
}

/// Format a complete response with optional fields.
///
/// # Example
///
/// ```rust
/// use mcp_tools::format::format_complete;
///
/// let response = format_complete(&[("message-to-llm", "all-complete")]);
/// assert_eq!(response, "(complete :message-to-llm \"all-complete\")");
/// ```
pub fn format_complete(fields: &[(&str, &str)]) -> String {
    if fields.is_empty() {
        "(complete)".to_string()
    } else {
        let field_strs: Vec<String> = fields
            .iter()
            .map(|(key, value)| format!(":{} {}", key, quote_str(value)))
            .collect();
        format!("(complete {})", field_strs.join(" "))
    }
}

/// Format a blocked response with waiting goals.
///
/// # Example
///
/// ```rust
/// use mcp_tools::format::format_blocked;
///
/// let response = format_blocked(
///     &["goal1".to_string(), "goal2".to_string()],
///     &[("message-to-llm", "blocked-waiting")],
/// );
/// assert_eq!(
///     response,
///     "(blocked :waiting-goals (\"goal1\" \"goal2\") :message-to-llm \"blocked-waiting\")"
/// );
/// ```
pub fn format_blocked(waiting_goals: &[String], fields: &[(&str, &str)]) -> String {
    let goals_str = serialize_string_list(waiting_goals);
    let field_strs: Vec<String> = fields
        .iter()
        .map(|(key, value)| format!(":{} {}", key, quote_str(value)))
        .collect();
    
    if field_strs.is_empty() {
        format!("(blocked :waiting-goals ({}))", goals_str)
    } else {
        format!(
            "(blocked :waiting-goals ({}) {})",
            goals_str,
            field_strs.join(" ")
        )
    }
}

/// Serialize a list of strings as space-separated quoted strings.
///
/// This wraps the existing `render_list()` function with automatic quoting.
///
/// # Example
///
/// ```rust
/// use mcp_tools::format::serialize_string_list;
///
/// let items = vec!["goal1".to_string(), "goal2".to_string()];
/// let result = serialize_string_list(&items);
/// assert_eq!(result, "\"goal1\" \"goal2\"");
/// ```
pub fn serialize_string_list(items: &[String]) -> String {
    render_list(items.iter().map(|s| quote_str(s)))
}

/// Format a resource reference as `(type "value")`.
///
/// # Example
///
/// ```rust
/// use mcp_tools::format::serialize_resource;
///
/// let result = serialize_resource("file", "src/main.rs");
/// assert_eq!(result, "(file \"src/main.rs\")");
/// ```
pub fn serialize_resource(resource_type: &str, value: &str) -> String {
    format!("({} {})", resource_type, quote_str(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_success() {
        let result = format_success(&[("id", "123"), ("status", "ok")]);
        assert_eq!(result, "(success :id \"123\" :status \"ok\")");
    }

    #[test]
    fn test_format_error() {
        let result = format_error("Not found");
        assert_eq!(result, "(error \"Not found\")");
    }

    #[test]
    fn test_format_complete() {
        let result = format_complete(&[]);
        assert_eq!(result, "(complete)");

        let result = format_complete(&[("message", "done")]);
        assert_eq!(result, "(complete :message \"done\")");
    }

    #[test]
    fn test_format_blocked() {
        let goals = vec!["g1".to_string(), "g2".to_string()];
        let result = format_blocked(&goals, &[("msg", "waiting")]);
        assert_eq!(
            result,
            "(blocked :waiting-goals (\"g1\" \"g2\") :msg \"waiting\")"
        );
    }

    #[test]
    fn test_serialize_string_list() {
        let items = vec!["a".to_string(), "b".to_string()];
        let result = serialize_string_list(&items);
        assert_eq!(result, "\"a\" \"b\"");
    }

    #[test]
    fn test_serialize_resource() {
        let result = serialize_resource("file", "test.rs");
        assert_eq!(result, "(file \"test.rs\")");
    }
}
