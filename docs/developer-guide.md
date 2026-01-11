# Developer Guide for MCP Tool Authors

## Overview

This document describes how to use the `mcp-sexpr` crate when implementing MCP (Model Context Protocol) tools.

The crate provides utilities for:

- Parsing S-expression tool calls
- Extracting keyword arguments
- Handling `(use "path")` file references
- Serializing S-expression fragments

## Tool Call Shape Conventions

A typical MCP tool call in S-expression form looks like:

```lisp
(tool-name :arg1 "value" :arg2 ("a" "b") :arg3 (use "path"))
```

Conventions:

- The head symbol is the tool name (e.g., `my-tool`)
- Arguments are keyword/value pairs
- Keywords may appear in any order
- Implementations MUST reject missing required keywords with clear errors

## Recommended Parsing Workflow

A typical parsing workflow for tool handlers:

1. **Parse** the input string into `lexpr::Value` using `parse_value`
2. **Validate** the top-level call is a list form
3. **Extract** keyword arguments using crate helpers:
   - `require_kw_str` for required string arguments
   - `get_kw_str` for optional string arguments
   - `get_kw_value` when the value may be non-string
4. **Convert** extracted values into your project's domain types

### Example

```rust
use mcp_sexpr::{parse_value, require_kw_str, get_kw_str, get_kw_value, parse_str_list};

fn handle_tool_call(input: &str) -> anyhow::Result<()> {
    // Step 1: Parse
    let value = parse_value(input)?;
    
    // Step 3: Extract keyword arguments
    let name = require_kw_str(&value, "name")?;
    let version = get_kw_str(&value, "version")?; // Optional
    
    // For list arguments
    if let Some(tags_value) = get_kw_value(&value, "tags")? {
        let tags = parse_str_list(&tags_value)?;
        // Use tags...
    }
    
    // Step 4: Convert to domain types (your code)
    Ok(())
}
```

## Keyword Argument Extraction

The crate normalizes keyword handling so tool implementations don't need to care how `lexpr` represents keywords.

### Supported Formats

Both of these are accepted:
- Symbols like `:name` (symbol starting with `:`)
- Keyword values if `lexpr` produces them

### Error Handling

Errors include the keyword name when:
- A required keyword is missing
- A keyword is present but has the wrong value type

```rust
// This will error with "missing required keyword :name"
let name = require_kw_str(&value, "name")?;

// This will error with ":version must be a string" if version is not a string
let version = get_kw_str(&value, "version")?;
```

## Handling `(use "path")` References

Many MCP workflows need a compact way to reference longer text stored in files.

### TextRef Type

```rust
pub enum TextRef {
    Literal(String),    // Plain string value
    UsePath(String),    // File reference from (use "path")
}
```

### Parsing Rules

- `"literal"` parses as `TextRef::Literal`
- `(use "path")` parses as `TextRef::UsePath`
- Any other form produces an error

### Example

```rust
use mcp_sexpr::{parse_value, get_kw_value, parse_text_ref, TextRef};

let input = r#"(define :spec (use "docs/spec.md"))"#;
let value = parse_value(input)?;

if let Some(spec_value) = get_kw_value(&value, "spec")? {
    match parse_text_ref(&spec_value)? {
        TextRef::Literal(text) => {
            // Use inline text directly
            println!("Spec: {}", text);
        }
        TextRef::UsePath(path) => {
            // Read file at path (caller responsibility)
            let content = std::fs::read_to_string(&path)?;
            println!("Spec from {}: {}", path, content);
        }
    }
}
```

**Important**: The crate only parses/serializes these references. Whether files are read is a **caller responsibility**.

## Serialization

The crate provides serialization primitives:

### `quote_str` — Quote and Escape Strings

```rust
use mcp_sexpr::quote_str;

let quoted = quote_str("hello");           // "\"hello\""
let escaped = quote_str("say \"hi\"");     // "\"say \\\"hi\\\"\""
```

### `render_list` — Join Rendered Items

```rust
use mcp_sexpr::{quote_str, render_list};

let items = vec![quote_str("a"), quote_str("b"), quote_str("c")];
let list = render_list(items);  // "\"a\" \"b\" \"c\""
```

### `render_text_ref` — Render TextRef

```rust
use mcp_sexpr::{render_text_ref, TextRef};

let literal = render_text_ref(&TextRef::Literal("hello".into()));
// "\"hello\""

let use_path = render_text_ref(&TextRef::UsePath("docs/spec.md".into()));
// "(use \"docs/spec.md\")"
```

## Error Message Guidance

When building tools with this crate, aim for errors that are:

- **Actionable** — Tell the user what keyword/value is wrong
- **Precise** — Specify expected shape (string vs list vs `(use ...)`)
- **Compatible** — Work with MCP tool error surfaces

The crate attaches context to parsing failures automatically.

## Best Practices

1. **Use `require_kw_str` for required arguments** — Provides clear error messages
2. **Use `get_kw_str` for optional arguments** — Returns `None` when missing
3. **Use `get_kw_value` for complex values** — Then parse with `parse_str_list` or `parse_text_ref`
4. **Keep domain-specific parsing separate** — This crate handles S-expression mechanics; your code handles domain logic
