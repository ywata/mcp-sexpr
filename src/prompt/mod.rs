//! Prompt system for MCP servers.
//!
//! This module provides a configuration-driven prompt building system that
//! combines TOML configuration files with markdown documentation.
//!
//! # Usage
//!
//! ```rust,no_run
//! use mcp_tools::prompt::PromptBuilder;
//!
//! let builder = PromptBuilder::new("tools.toml", ".")?;
//! let init_prompt = builder.build_initialize_prompt()?;
//! let tool_prompt = builder.build_tool_prompt("my-tool")?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

pub mod builder;
pub mod config;
pub mod markdown;

pub use builder::{PromptBuilder, PromptError, PromptResult};
pub use config::{Config, ConfigError, ConfigResult, InitializeConfig, ToolConfig};
pub use markdown::{extract_section, extract_sections, load_and_extract, MarkdownError, MarkdownResult};
