//! S-expression response formatting utilities for MCP servers.
//!
//! This module provides higher-level formatting functions for building
//! S-expression responses, built on top of the existing `quote_str()` and
//! `render_list()` functions from the root module.
//!
//! # Usage
//!
//! ```rust
//! use mcp_tools::format::*;
//!
//! // Format a success response
//! let response = format_success(&[
//!     ("internal-id", "uuid-123"),
//!     ("status", "complete"),
//! ]);
//! // => "(success :internal-id \"uuid-123\" :status \"complete\")"
//!
//! // Format an error response
//! let error = format_error("Resource not found");
//! // => "(error \"Resource not found\")"
//! ```

pub mod response;

pub use response::*;
