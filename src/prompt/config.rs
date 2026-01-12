//! Configuration parsing for tools.toml
//!
//! This module parses the tools.toml configuration file that specifies
//! which documentation sections to include in prompts.

use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

/// Errors that can occur during configuration parsing.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// IO error reading configuration file
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// TOML parsing error
    #[error("TOML parse error: {0}")]
    TomlError(#[from] toml::de::Error),

    /// Missing configuration for a tool
    #[error("Missing configuration for: {0}")]
    MissingConfig(String),
}

/// Result type for configuration operations.
pub type ConfigResult<T> = Result<T, ConfigError>;

/// Configuration for a single tool
#[derive(Debug, Clone, Deserialize)]
pub struct ToolConfig {
    /// Path to the documentation file
    pub prompt_doc: String,
    /// List of section headings to extract
    pub prompt_sections: Vec<String>,
    /// Optional alias pointing to the canonical tool name
    #[serde(default)]
    pub alias_for: Option<String>,
    /// Optional extra configuration fields (for extensibility)
    #[serde(flatten)]
    pub extra: HashMap<String, toml::Value>,
}

/// Configuration for the initialize response
#[derive(Debug, Clone, Deserialize)]
pub struct InitializeConfig {
    /// Path to the documentation file
    pub prompt_doc: String,
    /// List of section headings to extract
    pub prompt_sections: Vec<String>,
    /// Optional extra configuration fields (for extensibility)
    #[serde(flatten)]
    pub extra: HashMap<String, toml::Value>,
}

/// Top-level configuration
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// Initialize configuration
    pub initialize: InitializeConfig,
    /// Tool configurations
    pub tools: HashMap<String, ToolConfig>,
    /// Optional extra configuration fields (for extensibility)
    #[serde(flatten)]
    pub extra: HashMap<String, toml::Value>,
}

impl Config {
    /// Load configuration from a file
    pub fn from_file(path: impl AsRef<Path>) -> ConfigResult<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    /// Get tool configuration by name
    pub fn get_tool(&self, tool_name: &str) -> ConfigResult<&ToolConfig> {
        self.tools
            .get(tool_name)
            .ok_or_else(|| ConfigError::MissingConfig(tool_name.to_string()))
    }

    /// Get custom configuration value by key path (e.g., "my_app.settings")
    pub fn get_custom_config<T: serde::de::DeserializeOwned>(
        &self,
        key: &str,
    ) -> ConfigResult<Option<T>> {
        if let Some(value) = self.extra.get(key) {
            let result = T::deserialize(value.clone())
                .map_err(|e| ConfigError::TomlError(e))?;
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_config() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "[initialize]").unwrap();
        writeln!(file, "prompt_doc = \"spec.md\"").unwrap();
        writeln!(
            file,
            "prompt_sections = [\"# Overview\", \"## Usage\"]"
        )
        .unwrap();
        writeln!(file, "").unwrap();
        writeln!(file, "[tools.my-tool]").unwrap();
        writeln!(file, "prompt_doc = \"api-spec.md\"").unwrap();
        writeln!(file, "prompt_sections = [\"### 1. my-tool\"]").unwrap();
        writeln!(file, "").unwrap();
        writeln!(file, "[tools.another-tool]").unwrap();
        writeln!(file, "prompt_doc = \"api-spec.md\"").unwrap();
        writeln!(file, "prompt_sections = [\"### 2. another-tool\"]").unwrap();
        file.flush().unwrap();

        let config = Config::from_file(file.path()).unwrap();

        assert_eq!(config.initialize.prompt_doc, "spec.md");
        assert_eq!(config.initialize.prompt_sections.len(), 2);

        let tool_config = config.get_tool("my-tool").unwrap();
        assert_eq!(tool_config.prompt_doc, "api-spec.md");
        assert_eq!(tool_config.prompt_sections.len(), 1);
    }

    #[test]
    fn test_missing_tool() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "[initialize]").unwrap();
        writeln!(file, "prompt_doc = \"spec.md\"").unwrap();
        writeln!(file, "prompt_sections = [\"# Overview\"]").unwrap();
        writeln!(file, "").unwrap();
        writeln!(file, "[tools]").unwrap();
        file.flush().unwrap();

        let config = Config::from_file(file.path()).unwrap();
        let result = config.get_tool("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_extra_config() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "[initialize]").unwrap();
        writeln!(file, "prompt_doc = \"spec.md\"").unwrap();
        writeln!(file, "prompt_sections = [\"# Spec\"]").unwrap();
        writeln!(file, "").unwrap();
        writeln!(file, "[tools]").unwrap();
        writeln!(file, "").unwrap();
        writeln!(file, "[my_app]").unwrap();
        writeln!(file, "max_retries = 3").unwrap();
        file.flush().unwrap();

        let config = Config::from_file(file.path()).unwrap();
        
        #[derive(Deserialize)]
        struct MyAppConfig {
            max_retries: u32,
        }
        
        let custom: Option<MyAppConfig> = config.get_custom_config("my_app").unwrap();
        assert!(custom.is_some());
        assert_eq!(custom.unwrap().max_retries, 3);
    }
}
