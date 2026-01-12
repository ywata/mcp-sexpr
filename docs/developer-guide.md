# Developer Guide for mcp-tools

## Overview

This guide covers how to use the `mcp-tools` crate (v0.2.0) when building MCP (Model Context Protocol) servers and clients.

The crate provides:

### Core Features (always available)
- Parsing S-expression tool calls
- Extracting keyword arguments
- Handling `(use "path")` file references
- Serializing S-expression fragments

### Optional Features (via feature flags)
- **prompts** - Configuration-driven prompt building
- **interactive** - Interactive line loops with history
- **format** - Response formatting utilities
- **extract** - Type-safe argument extraction
- **persistence** - SQLite-based logging
- **log-viewer** - Interactive log query tool
- **router** - Tool routing patterns
- **errors** - Typed error examples

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
use mcp_tools::{parse_value, require_kw_str, get_kw_str, get_kw_value, parse_str_list};

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

---

## Feature Guide: Type-Safe Extraction (feature = "extract")

The `extract` module provides type-safe argument extraction with automatic type conversion.

### Available Functions

- `parse_tool_call(sexpr)` - Parse S-expression into lexpr::Value
- `require_string(value, key)` - Required string argument
- `get_string(value, key)` - Optional string argument
- `get_bool(value, key)` - Optional boolean (true/false/#t/#f)
- `get_int(value, key)` - Optional integer (i64)
- `get_uint(value, key)` - Optional unsigned integer (usize)
- `extract_string_list(value)` - Extract list of strings

### Example

```rust
use mcp_tools::extract::*;

let value = parse_tool_call("(tool :count 42 :enabled true)")?;
let count = get_int(&value, "count")?;      // Some(42)
let enabled = get_bool(&value, "enabled")?; // Some(true)
```

---

## Feature Guide: Response Formatting (feature = "format")

Build S-expression responses with consistent formatting.

```rust
use mcp_tools::format::*;

// Success response
let response = format_success(&[("id", "123"), ("status", "ok")]);
// "(success :id \"123\" :status \"ok\")"

// Error response
let error = format_error("Not found");
// "(error \"Not found\")"

// Blocked response with waiting items
let blocked = format_blocked(
    &["item1".to_string(), "item2".to_string()],
    &[("reason", "waiting")]
);
// "(blocked :waiting-goals (\"item1\" \"item2\") :reason \"waiting\")"
```

---

## Feature Guide: Prompt System (feature = "prompts")

Configuration-driven prompt building from TOML + markdown.

### Configuration File (tools.toml)

```toml
[initialize]
prompt_doc = "overview.md"
prompt_sections = ["# Introduction", "## Features"]

[tools.my-tool]
prompt_doc = "api-spec.md"
prompt_sections = ["## my-tool"]
```

### Usage

```rust
use mcp_tools::prompt::PromptBuilder;

let builder = PromptBuilder::new("tools.toml", "docs")?;

// Build initialize prompt
let init_prompt = builder.build_initialize_prompt()?;

// Build tool-specific prompt
let tool_prompt = builder.build_tool_prompt("my-tool")?;

// Get all tool names
let tools = builder.get_tool_names();
```

---

## Feature Guide: Interactive Line Loop (feature = "interactive")

Generic rustyline-based interactive loop with history support.

```rust
use mcp_tools::interactive::{run_line_loop, LineLoopConfig, LoopControl};

let config = LineLoopConfig::new(
    || "prompt> ".to_string(),
    true,  // add to history
    || LoopControl::Continue,  // on Ctrl-C
    || LoopControl::Break,     // on EOF
);

run_line_loop(config, |line| {
    // Process line
    println!("Got: {}", line);
    Ok(LoopControl::Continue)
})?;
```

For async support, use `run_line_loop_async` with the `interactive-async` feature.

---

## Feature Guide: Router Pattern (feature = "router")

Register and route tool calls to handlers.

```rust
use mcp_tools::router::Router;

let mut router = Router::new();

// Register handlers
router.register("echo", |args| {
    Ok(format!("(success :echo {})", args))
});

router.register("add", |args| {
    // Parse args and return result
    Ok("(success :result 42)".to_string())
});

// Register alias
router.register_alias("alias-tool", "echo");

// Route calls
let result = router.route("echo", "(echo :msg \"hello\")")?;
```

---

## Feature Guide: Persistence (feature = "persistence")

SQLite-based tool call logging for observability.

```rust
use mcp_tools::persistence::{SqlitePersistence, ToolCallEvent};

let db = SqlitePersistence::open("logs.db")?;

// Log tool call
let event = ToolCallEvent {
    transport: "stdio".to_string(),
    client_name: Some("my-client".to_string()),
    tool_name: "my-tool".to_string(),
    canonical_tool_name: "my-tool".to_string(),
    request_sexpr: "(my-tool :arg \"value\")".to_string(),
    response_sexpr: "(success)".to_string(),
    is_error: false,
    context_id: Some("session-123".to_string()),
};

db.insert_tool_call_event(&event)?;
```

---

## Feature Guide: Error Patterns (feature = "errors")

The `errors` module provides examples of typed error design using `thiserror`.

```rust
use mcp_tools::errors::{StateError, TransitionError, DependencyError};

// Example error types demonstrating best practices
// See module documentation for complete examples
```

These are example error types you can use as templates for your own error handling.

---

## Choosing Features

Select features based on your needs:

- **Core only** - Just S-expression parsing: `mcp-tools = "0.2"`
- **Server basics** - Add routing and formatting: `features = ["router", "format"]`
- **Full server** - All server features: `features = ["router", "format", "extract", "prompts", "persistence"]`
- **Interactive tools** - Add CLI support: `features = ["interactive", "log-viewer"]`
- **Everything** - All features: `features = ["all"]`

