# mcp-sexpr

S-expression utilities for MCP (Model Context Protocol) tool development.

## Features

- **Parse S-expressions** with `lexpr`
- **Extract keyword arguments** from tool-call forms (`:key value` pairs)
- **Handle `(use "path")` file references** with the `TextRef` type
- **Serialize strings** with proper escaping

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
mcp-sexpr = "0.1"
```

## Usage

### Parse and Extract Keywords

```rust
use mcp_sexpr::{parse_value, require_kw_str, get_kw_str};

let input = r#"(my-tool :name "example" :version "1.0")"#;
let value = parse_value(input)?;

let name = require_kw_str(&value, "name")?;
// name == "example"

let version = get_kw_str(&value, "version")?;
// version == Some("1.0")

let missing = get_kw_str(&value, "missing")?;
// missing == None
```

### Handle File References

```rust
use mcp_sexpr::{parse_value, get_kw_value, parse_text_ref, TextRef};

let input = r#"(define :spec (use "docs/spec.md"))"#;
let value = parse_value(input)?;

if let Some(spec_value) = get_kw_value(&value, "spec")? {
    match parse_text_ref(&spec_value)? {
        TextRef::Literal(text) => println!("Inline: {}", text),
        TextRef::UsePath(path) => println!("File: {}", path),
    }
}
```

### Parse String Lists

```rust
use mcp_sexpr::{parse_value, get_kw_value, parse_str_list};

let input = r#"(build :requires ("lib-a" "lib-b" "lib-c"))"#;
let value = parse_value(input)?;

if let Some(reqs_value) = get_kw_value(&value, "requires")? {
    let reqs = parse_str_list(&reqs_value)?;
    // reqs == vec!["lib-a", "lib-b", "lib-c"]
}
```

### Serialize S-expressions

```rust
use mcp_sexpr::{quote_str, render_list, render_text_ref, TextRef};

let quoted = quote_str("hello \"world\"");
// quoted == "\"hello \\\"world\\\"\""

let list = render_list(vec![quote_str("a"), quote_str("b")]);
// list == "\"a\" \"b\""

let text_ref = render_text_ref(&TextRef::UsePath("docs/spec.md".to_string()));
// text_ref == "(use \"docs/spec.md\")"
```

## API Reference

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

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
