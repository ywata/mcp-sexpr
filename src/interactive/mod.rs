//! Generic interactive line loop utilities.
//!
//! This module provides a rustyline-based interactive line loop with
//! configurable prompts, history, and interrupt/EOF handling.
//!
//! # Usage
//!
//! ```rust,no_run
//! use mcp_tools::interactive::{run_line_loop, LineLoopConfig, LoopControl};
//!
//! let config = LineLoopConfig::new(
//!     || "prompt> ".to_string(),
//!     true,  // add to history
//!     || LoopControl::Continue,  // on Ctrl-C
//!     || LoopControl::Break,     // on EOF
//! );
//!
//! run_line_loop(config, |line| {
//!     println!("Got: {}", line);
//!     Ok(LoopControl::Continue)
//! })?;
//! # Ok::<(), anyhow::Error>(())
//! ```

/// Line loop implementation with rustyline.
pub mod line_loop;

pub use line_loop::{default_history_path, run_line_loop, run_line_loop_async, HistoryKind, LineLoopConfig, LoopControl};
