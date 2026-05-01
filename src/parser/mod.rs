//! Position-tracking S-expression parser.
//!
//! This module provides the new parser that replaces `lexpr::Value` as the canonical
//! S-expression representation in `mcp-tools`. See `specs/parser/` for the
//! specification.
//!
//! The public surface is re-exported at the crate root; consumers do not normally
//! reach into `mcp_tools::parser::*` directly.
//!
//! Module layout:
//!
//! - [`types`] — `Value`, `Spanned`, `SpannedNode`, `Position`, `Span`, `Comment`.
//! - [`lexer`] — Hand-rolled tokenizer with byte-span tracking.
//! - [`reader`] — Recursive-descent parser building `Spanned` trees.
//! - [`lexpr_compat`] — Bidirectional `Value` ↔ `lexpr::Value` conversion.
//! - [`differential`] — Runtime comparison wrapper against `lexpr::from_str`.

pub mod types;
pub mod lexer;
pub mod reader;
pub mod lexpr_compat;
pub mod differential;

pub use types::{Comment, CommentKind, Position, Span, Spanned, SpannedNode, Value};
pub use reader::{parse_value, parse_value_with_positions, ParseError};
pub use lexpr_compat::LexprConversionError;
pub use differential::{
    current_differential_mode, flush_discrepancy_dedup, set_differential_mode,
    set_discrepancy_dedup_capacity, DifferentialMode, Discrepancy, DiscrepancyInput,
    DiscrepancySink, PathElement, StructuralPath,
};
