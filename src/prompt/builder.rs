//! Prompt building
//!
//! This module combines configuration and markdown extraction to build prompts.

use super::config::{Config, ConfigResult, InitializeConfig, ToolConfig};
use super::markdown::load_and_extract;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Errors that can occur during prompt building.
#[derive(Debug, Error)]
pub enum PromptError {
    /// Configuration error
    #[error("Config error: {0}")]
    ConfigError(#[from] crate::prompt::config::ConfigError),

    /// Markdown extraction error
    #[error("Markdown error: {0}")]
    MarkdownError(#[from] crate::prompt::markdown::MarkdownError),
}

/// Result type for prompt operations.
pub type PromptResult<T> = Result<T, PromptError>;

/// Prompt builder that loads configuration and extracts markdown sections
pub struct PromptBuilder {
    config: Config,
    docs_dir: PathBuf,
}

impl PromptBuilder {
    /// Create a new prompt builder
    /// config_path: Path to tools.toml
    /// docs_dir: Directory containing the markdown documentation files
    pub fn new(config_path: impl AsRef<Path>, docs_dir: impl AsRef<Path>) -> ConfigResult<Self> {
        let config = Config::from_file(config_path)?;
        Ok(Self {
            config,
            docs_dir: docs_dir.as_ref().to_path_buf(),
        })
    }

    /// Build the initialize prompt
    pub fn build_initialize_prompt(&self) -> PromptResult<String> {
        let init_config = &self.config.initialize;
        self.build_prompt_from_init_config(init_config)
    }

    /// Build a tool prompt
    pub fn build_tool_prompt(&self, tool_name: &str) -> PromptResult<String> {
        let tool_config = self.config.get_tool(tool_name)?;
        self.build_prompt_from_tool_config(tool_config)
    }

    /// Build prompt from initialize configuration
    fn build_prompt_from_init_config(&self, config: &InitializeConfig) -> PromptResult<String> {
        let doc_path = self.docs_dir.join(&config.prompt_doc);
        let content = load_and_extract(&doc_path, &config.prompt_sections)?;
        Ok(content)
    }

    /// Build prompt from a tool configuration
    fn build_prompt_from_tool_config(&self, config: &ToolConfig) -> PromptResult<String> {
        let doc_path = self.docs_dir.join(&config.prompt_doc);
        let content = load_and_extract(&doc_path, &config.prompt_sections)?;
        Ok(content)
    }

    /// Get all tool names from configuration
    pub fn get_tool_names(&self) -> Vec<String> {
        self.config.tools.keys().cloned().collect()
    }

    /// Get tool configuration by name
    pub fn get_tool_config(&self, tool_name: &str) -> PromptResult<&ToolConfig> {
        self.config.get_tool(tool_name).map_err(|e| e.into())
    }

    /// Get custom configuration value by key path (e.g., "my_app.settings")
    pub fn get_custom_config<T: serde::de::DeserializeOwned>(
        &self,
        key: &str,
    ) -> ConfigResult<Option<T>> {
        self.config.get_custom_config(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_setup() -> (TempDir, PathBuf, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let docs_dir = temp_dir.path().join("docs");
        std::fs::create_dir(&docs_dir).unwrap();

        // Create a test markdown file
        let md_path = docs_dir.join("test.md");
        let mut md_file = std::fs::File::create(&md_path).unwrap();
        writeln!(md_file, "# Section 1").unwrap();
        writeln!(md_file, "Content 1").unwrap();
        writeln!(md_file, "").unwrap();
        writeln!(md_file, "## Section 2").unwrap();
        writeln!(md_file, "Content 2").unwrap();

        // Create a test config file
        let config_path = temp_dir.path().join("tools.toml");
        let mut config_file = std::fs::File::create(&config_path).unwrap();
        writeln!(config_file, "[initialize]").unwrap();
        writeln!(config_file, "prompt_doc = \"test.md\"").unwrap();
        writeln!(config_file, "prompt_sections = [\"# Section 1\"]").unwrap();
        writeln!(config_file, "").unwrap();
        writeln!(config_file, "[tools.test-tool]").unwrap();
        writeln!(config_file, "prompt_doc = \"test.md\"").unwrap();
        writeln!(config_file, "prompt_sections = [\"## Section 2\"]").unwrap();

        (temp_dir, config_path, docs_dir)
    }

    #[test]
    fn test_prompt_builder_initialize() {
        let (_temp_dir, config_path, docs_dir) = create_test_setup();

        let builder = PromptBuilder::new(&config_path, &docs_dir).unwrap();
        let prompt = builder.build_initialize_prompt().unwrap();

        assert!(prompt.contains("# Section 1"));
        assert!(prompt.contains("Content 1"));
    }

    #[test]
    fn test_prompt_builder_tool() {
        let (_temp_dir, config_path, docs_dir) = create_test_setup();

        let builder = PromptBuilder::new(&config_path, &docs_dir).unwrap();
        let prompt = builder.build_tool_prompt("test-tool").unwrap();

        assert!(prompt.contains("## Section 2"));
        assert!(prompt.contains("Content 2"));
    }

    #[test]
    fn test_get_tool_names() {
        let (_temp_dir, config_path, docs_dir) = create_test_setup();

        let builder = PromptBuilder::new(&config_path, &docs_dir).unwrap();
        let tools = builder.get_tool_names();

        assert!(tools.contains(&"test-tool".to_string()));
    }
}
