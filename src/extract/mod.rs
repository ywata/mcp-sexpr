//! Type-safe argument extraction utilities for MCP tool calls.
//!
//! This module provides convenient wrappers around the existing `mcp-sexpr`
//! functions (`get_kw_str`, `get_kw_value`, `require_kw_str`, `parse_str_list`)
//! with additional type conversion utilities for common types like integers and booleans.
//!
//! # Usage
//!
//! ```rust
//! use mcp_tools::extract::*;
//!
//! let sexpr = r#"(my-tool :name "example" :count 42 :enabled true)"#;
//! let value = parse_tool_call(sexpr)?;
//!
//! let name = require_string(&value, "name")?;  // "example"
//! let count = get_int(&value, "count")?;       // Some(42)
//! let enabled = get_bool(&value, "enabled")?;  // Some(true)
//! # Ok::<(), anyhow::Error>(())
//! ```

pub mod args;

pub use args::*;
