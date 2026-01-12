//! Common routing patterns for MCP servers.
//!
//! This module demonstrates patterns for building MCP server routers
//! with consistent error handling and progress tracking.

use anyhow::{Context, Result};
use std::collections::HashMap;

/// A tool handler function that takes S-expression arguments and returns a result.
pub type ToolHandler = Box<dyn Fn(&str) -> Result<String> + Send + Sync>;

/// A router that maps tool names to handler functions.
pub struct Router {
    handlers: HashMap<String, ToolHandler>,
    aliases: HashMap<String, String>,
}

impl Router {
    /// Create a new empty router.
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
            aliases: HashMap::new(),
        }
    }

    /// Register a tool handler.
    pub fn register<F>(&mut self, tool_name: impl Into<String>, handler: F)
    where
        F: Fn(&str) -> Result<String> + Send + Sync + 'static,
    {
        self.handlers.insert(tool_name.into(), Box::new(handler));
    }

    /// Register an alias for a tool.
    pub fn register_alias(&mut self, alias: impl Into<String>, canonical: impl Into<String>) {
        self.aliases.insert(alias.into(), canonical.into());
    }

    /// Route a tool call to its handler.
    pub fn route(&self, tool_name: &str, sexpr: &str) -> Result<String> {
        // Resolve alias if present
        let canonical_name = self.aliases.get(tool_name).map(|s| s.as_str()).unwrap_or(tool_name);

        // Find and call handler
        let handler = self
            .handlers
            .get(canonical_name)
            .ok_or_else(|| anyhow::anyhow!("Unknown tool: {}", tool_name))?;

        handler(sexpr).with_context(|| format!("Error executing tool: {}", tool_name))
    }

    /// Get all registered tool names (excluding aliases).
    pub fn tool_names(&self) -> Vec<String> {
        self.handlers.keys().cloned().collect()
    }

    /// Check if a tool is registered.
    pub fn has_tool(&self, tool_name: &str) -> bool {
        let canonical_name = self.aliases.get(tool_name).map(|s| s.as_str()).unwrap_or(tool_name);
        self.handlers.contains_key(canonical_name)
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

/// Progress event information for tracking tool execution.
#[derive(Debug, Clone)]
pub struct ProgressEvent {
    /// The name of the tool that was executed
    pub tool_name: &'static str,
    /// Additional context about the execution
    pub context: String,
}

/// Result of routing a tool call, including optional progress event.
pub struct RouteResult {
    /// The response from the tool handler
    pub response: String,
    /// Optional progress event for tracking
    pub progress_event: Option<ProgressEvent>,
}

impl RouteResult {
    /// Create a result with no progress event.
    pub fn new(response: String) -> Self {
        Self {
            response,
            progress_event: None,
        }
    }

    /// Create a result with a progress event.
    pub fn with_progress(response: String, tool_name: &'static str, context: String) -> Self {
        Self {
            response,
            progress_event: Some(ProgressEvent { tool_name, context }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router_basic() {
        let mut router = Router::new();
        router.register("echo", |args| Ok(format!("(success :echo {})", args)));

        let result = router.route("echo", "(echo :msg \"hello\")").unwrap();
        assert!(result.contains("hello"));
    }

    #[test]
    fn test_router_alias() {
        let mut router = Router::new();
        router.register("canonical-tool", |_| Ok("(success)".to_string()));
        router.register_alias("alias-tool", "canonical-tool");

        let result = router.route("alias-tool", "()").unwrap();
        assert_eq!(result, "(success)");
    }

    #[test]
    fn test_router_unknown_tool() {
        let router = Router::new();
        let result = router.route("unknown", "()");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown tool"));
    }

    #[test]
    fn test_router_tool_names() {
        let mut router = Router::new();
        router.register("tool1", |_| Ok("()".to_string()));
        router.register("tool2", |_| Ok("()".to_string()));

        let names = router.tool_names();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"tool1".to_string()));
        assert!(names.contains(&"tool2".to_string()));
    }

    #[test]
    fn test_has_tool() {
        let mut router = Router::new();
        router.register("existing", |_| Ok("()".to_string()));

        assert!(router.has_tool("existing"));
        assert!(!router.has_tool("nonexistent"));
    }
}
