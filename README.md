# mcp-tools

Comprehensive toolkit for MCP (Model Context Protocol) server and client development in Rust.

## Features

### Core S-expression Utilities (always available)

- **Parse S-expressions** with `lexpr`
- **Extract keyword arguments** from tool-call forms (`:key value` pairs)
- **Handle `(use "path")` file references** with the `TextRef` type
- **Serialize strings** with proper escaping

### Optional Features

Enable additional functionality via feature flags:

- **`prompts`** - TOML configuration + markdown prompt building system
- **`interactive`** - Generic rustyline-based interactive line loop (sync)
- **`interactive-async`** - Async variant of interactive line loop (requires tokio)
- **`format`** - S-expression response formatting utilities
- **`extract`** - Type-safe argument extraction with type conversion
- **`persistence`** - SQLite-based tool call logging and observability
- **`log-viewer`** - Interactive CLI for querying tool call logs
- **`router`** - MCP server routing patterns with handler registration
- **`errors`** - Typed error patterns and examples using thiserror

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
mcp-tools = "0.2"

# Or with specific features
mcp-tools = { version = "0.2", features = ["prompts", "interactive", "format"] }
```

## Quick Start

### Core S-expression Utilities

```rust
use mcp_tools::{parse_value, require_kw_str, get_kw_str};

let input = r#"(my-tool :name "example" :version "1.0")"#;
let value = parse_value(input)?;

let name = require_kw_str(&value, "name")?;
// name == "example"

let version = get_kw_str(&value, "version")?;
// version == Some("1.0")
```

### Prompt System (feature = "prompts")

```rust
use mcp_tools::prompt::PromptBuilder;

let builder = PromptBuilder::new("tools.toml", "docs")?;
let init_prompt = builder.build_initialize_prompt()?;
let tool_prompt = builder.build_tool_prompt("my-tool")?;
```

### Type-Safe Argument Extraction (feature = "extract")

```rust
use mcp_tools::extract::*;

let value = parse_tool_call("(tool :count 42 :enabled true)")?;
let count = get_int(&value, "count")?;      // Some(42)
let enabled = get_bool(&value, "enabled")?; // Some(true)
```

### Response Formatting (feature = "format")

```rust
use mcp_tools::format::*;

let response = format_success(&[
    ("id", "123"),
    ("status", "complete"),
]);
// "(success :id \"123\" :status \"complete\")"
```

### Interactive Line Loop (feature = "interactive")

```rust
use mcp_tools::interactive::{run_line_loop, LineLoopConfig, LoopControl};

let config = LineLoopConfig::new(
    || "prompt> ".to_string(),
    true,  // add to history
    || LoopControl::Continue,  // on Ctrl-C
    || LoopControl::Break,     // on EOF
);

run_line_loop(config, |line| {
    println!("Got: {}", line);
    Ok(LoopControl::Continue)
})?;
```

### Router Pattern (feature = "router")

```rust
use mcp_tools::router::Router;

let mut router = Router::new();
router.register("echo", |args| {
    Ok(format!("(success :echo {})", args))
});

let result = router.route("echo", "(echo :msg \"hello\")")?;
```

## Documentation

- **[API Documentation](https://docs.rs/mcp-tools)** - Full API reference
- **[Developer Guide](docs/developer-guide.md)** - Comprehensive usage guide
- **[Examples](examples/)** - Example implementations

## Core API Reference

### Parsing

- `parse_value(input: &str) -> Result<lexpr::Value>` — Parse S-expression string
- `parse_str_list(value: &lexpr::Value) -> Result<Vec<String>>` — Parse list of strings
- `parse_text_ref(value: &lexpr::Value) -> Result<TextRef>` — Parse string or `(use "path")`
- `iter_list(value: &lexpr::Value) -> Result<impl Iterator<Item = lexpr::Value>>` — Iterate list items

### Keyword Extraction

- `get_kw_value(root, key) -> Result<Option<lexpr::Value>>` — Get raw keyword value
- `get_kw_str(root, key) -> Result<Option<String>>` — Get keyword as string
- `require_kw_str(root, key) -> Result<String>` — Get required keyword as string

### Serialization

- `quote_str(s: &str) -> String` — Quote and escape string
- `render_list(items) -> String` — Join items with spaces
- `render_text_ref(value: &TextRef) -> String` — Render TextRef to S-expression

### Types

- `TextRef` — Either `Literal(String)` or `UsePath(String)`

See the [API documentation](https://docs.rs/mcp-tools) for complete details on all features.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
