//! Error type patterns and examples for MCP server development.
//!
//! This module provides examples of strongly-typed error enums using `thiserror`,
//! demonstrating the design principle: **"Define enum to convey meaning so that
//! specification changes produce compile-time errors"**.
//!
//! # When to Use `thiserror` vs `anyhow`
//!
//! - **Use `thiserror`** for library code and domain-specific errors where:
//!   - You want compile-time guarantees about error variants
//!   - Callers need to match on specific error types
//!   - Error types represent your domain model
//!
//! - **Use `anyhow`** for application code where:
//!   - You need flexible error propagation with context
//!   - Specific error types don't matter to the caller
//!   - You want convenient error handling with `?` operator
//!
//! # Error Design Patterns
//!
//! ## Pattern 1: State Management Errors
//!
//! Use enums to represent different failure modes in state operations:
//!
//! ```rust
//! use thiserror::Error;
//!
//! #[derive(Debug, Error)]
//! pub enum StateError {
//!     #[error("Resource not found: {0}")]
//!     NotFound(String),
//!     
//!     #[error("Invalid state transition from {from:?} to {to:?}")]
//!     InvalidTransition { from: String, to: String },
//! }
//! ```
//!
//! ## Pattern 2: Error Conversion
//!
//! Use `#[from]` to enable automatic conversion with `?` operator:
//!
//! ```rust
//! use thiserror::Error;
//! use mcp_tools::errors::StateError;
//!
//! #[derive(Debug, Error)]
//! pub enum MyError {
//!     #[error("State error: {0}")]
//!     StateError(#[from] StateError),
//!     
//!     #[error("IO error: {0}")]
//!     IoError(#[from] std::io::Error),
//! }
//! ```
//!
//! ## Pattern 3: Context Propagation
//!
//! Combine with `anyhow::Context` for rich error messages:
//!
//! ```rust
//! use anyhow::{Context, Result};
//!
//! fn process_file(path: &str) -> Result<String> {
//!     std::fs::read_to_string(path)
//!         .with_context(|| format!("Failed to read file: {}", path))?;
//!     Ok("success".to_string())
//! }
//! ```
//!
//! # Example Error Types
//!
//! See [`types`] module for real-world error type examples demonstrating these patterns.

pub mod types;

pub use types::{DependencyError, StateError, TransitionError};
