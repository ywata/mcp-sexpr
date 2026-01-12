//! MCP server routing patterns.
//!
//! This module provides patterns for routing MCP tool calls to handler functions
//! with consistent error handling and progress event tracking.
//!
//! # Usage
//!
//! ```rust,no_run
//! use mcp_tools::router::*;
//! use std::collections::HashMap;
//!
//! // Define your handler type
//! type Handler = Box<dyn Fn(&str) -> anyhow::Result<String>>;
//!
//! // Create a router
//! let mut router: HashMap<String, Handler> = HashMap::new();
//! router.insert("my-tool".to_string(), Box::new(|args| {
//!     Ok(format!("(success :result \"processed: {}\")", args))
//! }));
//!
//! // Route a call
//! if let Some(handler) = router.get("my-tool") {
//!     let result = handler("(my-tool :arg \"value\")")?;
//!     println!("{}", result);
//! }
//! # Ok::<(), anyhow::Error>(())
//! ```

pub mod patterns;

pub use patterns::*;
