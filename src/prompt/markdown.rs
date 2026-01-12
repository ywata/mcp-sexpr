//! Markdown section extraction
//!
//! This module extracts specific sections from markdown files based on headings.

use std::path::Path;
use thiserror::Error;

/// Errors that can occur during markdown extraction.
#[derive(Debug, Error)]
pub enum MarkdownError {
    /// IO error reading markdown file
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Requested section not found in markdown
    #[error("Section not found: {0}")]
    SectionNotFound(String),
}

/// Result type for markdown operations.
pub type MarkdownResult<T> = Result<T, MarkdownError>;

/// Extract a section from a markdown file
/// The section starts at the given heading and continues until the next heading of equal or higher level
pub fn extract_section(content: &str, section_heading: &str) -> MarkdownResult<String> {
    let lines: Vec<&str> = content.lines().collect();

    // Determine the heading level
    let heading_level = section_heading.chars().take_while(|&c| c == '#').count();

    // Find the start of the section
    let start_idx = lines
        .iter()
        .position(|line| line.trim() == section_heading.trim())
        .ok_or_else(|| MarkdownError::SectionNotFound(section_heading.to_string()))?;

    // Find the end of the section (next heading of equal or higher level)
    let end_idx = lines[start_idx + 1..]
        .iter()
        .position(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with('#') {
                let level = trimmed.chars().take_while(|&c| c == '#').count();
                level <= heading_level
            } else {
                false
            }
        })
        .map(|i| start_idx + 1 + i)
        .unwrap_or(lines.len());

    // Extract the section
    let section_lines = &lines[start_idx..end_idx];
    Ok(section_lines.join("\n"))
}

/// Extract multiple sections from a markdown file
pub fn extract_sections(content: &str, section_headings: &[String]) -> MarkdownResult<String> {
    let mut result = Vec::new();

    for heading in section_headings {
        let section = extract_section(content, heading)?;
        result.push(section);
    }

    Ok(result.join("\n\n"))
}

/// Load markdown file and extract sections
pub fn load_and_extract(
    path: impl AsRef<Path>,
    section_headings: &[String],
) -> MarkdownResult<String> {
    let content = std::fs::read_to_string(path)?;
    extract_sections(&content, section_headings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_section() {
        let content = r#"# Heading 1
Content 1

## Heading 2
Content 2

### Heading 3
Content 3

## Heading 4
Content 4
"#;

        let result = extract_section(content, "## Heading 2").unwrap();
        assert!(result.contains("## Heading 2"));
        assert!(result.contains("Content 2"));
        assert!(result.contains("### Heading 3"));
        assert!(!result.contains("## Heading 4"));
    }

    #[test]
    fn test_extract_section_not_found() {
        let content = "# Heading 1\nContent";
        let result = extract_section(content, "## Nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_multiple_sections() {
        let content = r#"# Heading 1
Content 1

## Heading 2
Content 2

## Heading 3
Content 3
"#;

        let sections = vec!["## Heading 2".to_string(), "## Heading 3".to_string()];
        let result = extract_sections(content, &sections).unwrap();

        assert!(result.contains("## Heading 2"));
        assert!(result.contains("Content 2"));
        assert!(result.contains("## Heading 3"));
        assert!(result.contains("Content 3"));
    }

    #[test]
    fn test_extract_top_level_section() {
        let content = r#"# Heading 1
Content 1

# Heading 2
Content 2
"#;

        let result = extract_section(content, "# Heading 1").unwrap();
        assert!(result.contains("# Heading 1"));
        assert!(result.contains("Content 1"));
        assert!(!result.contains("# Heading 2"));
    }
}
