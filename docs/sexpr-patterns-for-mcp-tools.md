# S-Expression Patterns for MCP Tools

This document describes the generic S-expression protocol patterns extracted from mcp-planner for reuse via the `mcp-sexpr` crate. These patterns are applicable to any MCP server — they are not specific to goal management or any particular domain.

mcp-planner's Goal AST is one application of these patterns. For Goal AST specifics, see `specs/goal-ast.md` and `specs/goal/syntax.md`.

## Overview

All tool communication uses S-expressions for both input and output. Each tool accepts an S-expression string and returns an S-expression string.

How this integrates with the MCP transport layer is described in Pattern 7 (Server Wiring).

Benefits:
- Uniform input/output format across all tools
- Keyword-argument parsing with required/optional field semantics
- Symbols for enums — no string-to-enum mapping boilerplate
- Nested domain-specific data embedded naturally without extra encoding

## Foundation: `mcp-sexpr` Crate

The `mcp-sexpr` crate provides the low-level primitives:

| Function | Purpose |
|---|---|
| `parse_value(s)` | Parse an S-expression string into `lexpr::Value` |
| `get_kw_str(val, key)` | Extract optional string for `:key` → `Result<Option<String>>` |
| `require_kw_str(val, key)` | Extract required string for `:key` → `Result<String>` |
| `get_kw_value(val, key)` | Extract optional raw `lexpr::Value` for `:key` |
| `quote_str(s)` | Escape and quote a string for S-expression output |
| `render_list(iter)` | Join items with spaces into a flat list |

## Pattern 1: Tool Input — Keyword-Argument Form

Tool input is an S-expression using keyword-argument form:

```lisp
(tool-name :key1 "string-value" :key2 symbol-value :key3 (nested expression))
```

The head symbol (`tool-name`) is present for readability but not required by the keyword extraction functions — they scan for `:key` keywords regardless of list structure.

### Value types in keyword arguments

| Type | Syntax | Extracted via |
|---|---|---|
| String | `:name "Alice"` | `get_kw_str`, `require_kw_str` |
| Symbol | `:format json` | `get_kw_value` → `.as_symbol()` |
| Boolean | `:verbose true` | `get_kw_value` → `.as_symbol()` |
| Nested S-expr | `:config (timeout 30)` | `get_kw_value` → traverse |
| List of strings | `:tags ("a" "b")` | `get_kw_value` → iterate |

### Implementation: Argument Extractor

Each tool has a dedicated `extract_*_args()` that returns a typed tuple:

```rust
use mcp_tools::{parse_value, get_kw_str, require_kw_str};

fn extract_search_args(sexpr: &str) -> Result<(String, Option<String>, bool)> {
    let value = parse_value(sexpr)?;
    let query = require_kw_str(&value, "query")?;
    let filter = get_kw_str(&value, "filter")?;
    let exact = get_kw_value(&value, "exact")?
        .and_then(|v| v.as_symbol().map(|s| s == "true"))
        .unwrap_or(false);
    Ok((query, filter, exact))
}
```

Example inputs:

```lisp
;; Required keyword only
(search :query "memory leak")

;; With optional keyword
(search :query "memory leak" :filter "*.rs")

;; With boolean symbol
(search :query "memory leak" :filter "*.rs" :exact true)
```

### Legacy alias support

A tool can accept alternative keyword names for backward compatibility:

```rust
fn extract_id(value: &lexpr::Value) -> Result<String> {
    let new_name = get_kw_str(value, "project-id")?;
    let old_name = get_kw_str(value, "pid")?;  // legacy alias

    match (new_name, old_name) {
        (Some(_), Some(_)) => Err(anyhow!("Provide exactly one of :project-id or :pid")),
        (Some(id), None) | (None, Some(id)) => Ok(id),
        (None, None) => Err(anyhow!("Missing project-id")),
    }
}
```

## Pattern 2: Success Response

Success responses use `(success ...)` as the head with keyword fields:

```lisp
;; Minimal — identifier returned
(success :id "item-42")

;; With status
(success :id "job-7" :status running)

;; With optional fields (omit when absent)
(success :id "job-7" :status complete :result "42 matches found")

;; Without optional fields
(success :id "job-7" :status pending)

;; With list values
(success :matched ("foo.rs" "bar.rs" "baz.rs") :count 3)
```

### Implementation

```rust
fn format_response(id: &str, status: Status, detail: Option<&str>) -> String {
    let mut parts = vec![
        format!(":id \"{}\"", escape_sexpr(id)),
        format!(":status {}", status.to_sexpr()),
    ];
    if let Some(d) = detail {
        parts.push(format!(":result \"{}\"", escape_sexpr(d)));
    }
    format!("(success {})", parts.join(" "))
}
```

Key conventions:
- Strings are always quoted: `:name "value"`
- Symbols (enums/status) are unquoted: `:status running`
- Nested S-expressions are inlined: `:config (timeout 30 :retries 3)`
- Lists are parenthesized: `:tags ("fast" "unstable")`
- Optional fields are omitted entirely when None/empty

## Pattern 3: Error Response

### Simple Error (backward compatible)

```lisp
(error "Connection refused")
```

### Structured Error

```lisp
(error
  :category io-error
  :code connection-refused
  :message "Failed to connect to database"
  :details "Host db.internal:5432 unreachable after 3 retries"
  :hint "Check network configuration or database status")
```

Field semantics:

| Field | Required | Type | Purpose |
|---|---|---|---|
| `:category` | yes | symbol | Error class for programmatic dispatch |
| `:code` | yes | symbol | Specific error code within category |
| `:message` | yes | string | Human-readable summary |
| `:details` | no | string | Additional context |
| `:location` | no | nested | Where in the input the error occurred |
| `:hint` | no | string | Actionable recovery suggestion |
| `:*` (extra) | no | string | Tool-specific extension fields |

### Error Location

Location is itself a nested S-expression with optional fields:

```lisp
;; Source position
(:line 42 :column 15)

;; Structural position
(:path ["config" "database" "host"] :field "port" :index 2)
```

### Multiple Errors

```lisp
(error
  :multiple true
  :count 2
  :errors (
    (error :category validation-error :code missing-field :message "Missing 'host'")
    (error :category validation-error :code invalid-port :message "Port must be 1-65535")))
```

### Recommended Error Categories

| Category | When |
|---|---|
| `parse-error` | S-expression syntax failures |
| `validation-error` | Semantic validation failures |
| `state-error` | Invalid state for the requested operation |
| `not-found` | Resource or entity not found |
| `parameter-error` | Missing or invalid parameters |
| `constraint-violation` | Business rule violation |
| `io-error` | File system or network errors |
| `internal-error` | Unexpected server failures |

### Implementation

```rust
struct StructuredError {
    category: String,
    code: String,
    message: String,
    details: Option<String>,
    hint: Option<String>,
    extra: HashMap<String, String>,
}

impl StructuredError {
    fn to_sexpr(&self) -> String {
        let mut parts = vec![
            format!(":category {}", self.category),
            format!(":code {}", self.code),
            format!(":message \"{}\"", escape_sexpr(&self.message)),
        ];
        if let Some(ref d) = self.details {
            parts.push(format!(":details \"{}\"", escape_sexpr(d)));
        }
        if let Some(ref h) = self.hint {
            parts.push(format!(":hint \"{}\"", escape_sexpr(h)));
        }
        for (k, v) in &self.extra {
            parts.push(format!(":{} \"{}\"", k, escape_sexpr(v)));
        }
        format!("(error {})", parts.join(" "))
    }
}
```

## Pattern 4: Enum-as-Symbol

Rust enums serialize to/from unquoted kebab-case symbols.

### Serialization (Rust enum → symbol)

```rust
enum Priority { Low, Normal, High, Critical }

impl Priority {
    fn to_sexpr(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Normal => "normal",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }
}
```

In output: `:priority high` (not `:priority "high"`)

### Deserialization (symbol → Rust enum)

```rust
fn parse_priority(value: &lexpr::Value, key: &str) -> Result<Priority> {
    let raw = get_kw_value(value, key)?
        .ok_or_else(|| anyhow!("Missing {}", key))?;
    let sym = raw.as_symbol()
        .ok_or_else(|| anyhow!("{} must be a symbol", key))?;
    match sym {
        "low" => Ok(Priority::Low),
        "normal" => Ok(Priority::Normal),
        "high" => Ok(Priority::High),
        "critical" => Ok(Priority::Critical),
        other => Err(anyhow!("Invalid {}: {}", key, other)),
    }
}
```

### Conditional validation based on symbol value

```rust
// :reason is required only when :status is failed
let reason = get_kw_str(&value, "reason")?;
if matches!(status, Status::Failed) && reason.is_none() {
    return Err(anyhow!("Missing reason for failed status"));
}
```

Convention: symbols are always lowercase kebab-case (`in-progress`, `read-only`, `high`).

## Pattern 5: Multi-Variant Response

When a tool can return structurally different responses, use **distinct head symbols** as discriminants. Clients dispatch on the first token.

```lisp
;; Results available
(results :items ("item-1" "item-2") :total 42)

;; Nothing found
(empty)

;; Operation failed
(failed :reason "timeout after 30s")

;; Still processing
(pending :retry-after 5)
```

### Implementation

```rust
enum QueryResponse {
    Results { items: Vec<String>, total: usize },
    Empty,
    Failed { reason: String },
    Pending { retry_after: u32 },
}

fn format_query_response(resp: QueryResponse) -> String {
    match resp {
        QueryResponse::Results { items, total } =>
            format!("(results :items ({}) :total {})",
                serialize_string_list(&items), total),
        QueryResponse::Empty =>
            "(empty)".to_string(),
        QueryResponse::Failed { reason } =>
            format!("(failed :reason \"{}\")", escape_sexpr(&reason)),
        QueryResponse::Pending { retry_after } =>
            format!("(pending :retry-after {})", retry_after),
    }
}
```

This maps directly to Rust's `enum` — each variant becomes a distinct head symbol with its own keyword fields.

## Pattern 6: Embedded Domain Data

Tool responses can embed domain-specific S-expressions as opaque nested values. The protocol layer does not interpret them — it wraps them into the response structure.

```lisp
(success :id "session-42"
  :data (my-domain-type "content" :field1 "x" :field2 (nested 1 2 3))
  :action continue)
```

### Implementation

The domain layer serializes its data to an S-expression string. The response formatter embeds it without quoting:

```rust
// Domain layer produces an S-expression string
let data_sexpr: String = my_domain::serialize(&data);

// Response formatter embeds it as-is (not quoted)
format!("(success :id \"{}\" :data {} :action {})",
    id, data_sexpr, action.to_sexpr())
```

The embedded `data_sexpr` appears as a nested S-expression in the output, not as a string containing an S-expression. This is the boundary between the generic protocol layer and domain-specific data.

In mcp-planner, the domain data is Goal AST. In another project it could be an AST for a different language, a configuration tree, a query plan, etc.

## Pattern 7: Embedding S-Expressions in the MCP Transport

MCP uses JSON for its transport layer. Our S-expression protocol is embedded inside this JSON transport — each tool receives its S-expression input as a JSON string value, and returns its S-expression output as a JSON text content block.

### How S-Expressions Are Carried Over JSON

**Input**: The S-expression is embedded as a string in a JSON object. We use a single field `"s-expr"` shared by all tools:

```json
{
  "type": "object",
  "properties": {
    "s-expr": { "type": "string", "description": "S-expression command" }
  },
  "required": ["s-expr"]
}
```

**Output**: The result S-expression is returned as MCP text content (a JSON string).

This means S-expressions are serialized to strings twice — once as S-expression syntax, then again as a JSON string value. The server handles this boundary: it extracts the `"s-expr"` string from JSON, processes it using the S-expression patterns (Patterns 1-6), and wraps the result string back into JSON.

### Example: S-Expression Embedded in JSON

**Request** — the MCP client sends a JSON `tools/call` request. The S-expression is a string value inside the `"arguments"` object:

```json
{
  "method": "tools/call",
  "params": {
    "name": "search",
    "arguments": {
      "s-expr": "(search :query \"memory leak\" :filter \"*.rs\" :exact true)"
    }
  }
}
```

**Response** — the server returns the result S-expression as a text content block:

```json
{
  "content": [
    {
      "type": "text",
      "text": "(success :matched (\"alloc.rs\" \"pool.rs\") :count 2)"
    }
  ]
}
```

Note how quotes inside the S-expression are escaped as `\"` in the JSON string. The MCP framework handles this JSON encoding/decoding — the S-expression layer works with the unescaped strings.

### Server Processing Pipeline

```
JSON input  →  extract "s-expr" string
                    │
                    ▼
              S-expression processing
                extract_*_args(sexpr)     Parse keyword args → typed tuple
                handler.operation()       Execute business logic
                format_*_response(resp)   Serialize result → S-expr string
                    │
                    ▼
JSON output  ←  wrap as MCP text content
```

### Key implementation points

1. **Uniform embedding**: All tools share the same `"s-expr"` JSON field — the JSON schema is identical for every tool. Tool-specific structure lives entirely in the S-expression layer.
2. **Router dispatches by name**: A `match` on the canonical tool name calls the tool-specific extractor and formatter.
3. **Extractors return typed tuples**: Each `extract_*_args()` converts raw S-expr string into `Result<(T1, T2, ...)>`.
4. **Formatters produce strings**: Each `format_*_response()` takes a typed response and returns an S-expr string.
5. **Errors convert uniformly**: `StructuredError::to_sexpr()` provides a single error format across all tools.
6. **Domain data is opaque**: The wiring layer does not parse domain-specific nested S-expressions. It passes them as strings between the domain layer and the response formatter.

### Tool description delivery

Tool descriptions (for MCP `tools/list`) are loaded from documentation files via a configuration file (`tools.toml`), not hardcoded. Each tool's description includes S-expression usage examples so LLM clients know the expected input format.

## String Escaping

Always escape strings before embedding in S-expressions. Use `mcp_tools::quote_str()` — it handles both escaping and wrapping in double quotes.

The escape policy (chosen to round-trip through `parse_value` / `lexpr`):

| Input char | Emitted as |
|---|---|
| `\` | `\\` |
| `"` | `\"` |
| LF (U+000A) | `\n` |
| CR (U+000D) | `\r` |
| TAB (U+0009) | `\t` |

Other control characters are passed through verbatim — lexpr does not accept hex escapes on parse, so there is no portable way to escape e.g. NUL. If you need to carry binary data, encode it (base64, hex) before quoting.

## Summary of Conventions

| Convention | Rule | Example |
|---|---|---|
| Input form | `(tool-name :key1 val1 :key2 val2)` | `(search :query "bug" :exact true)` |
| Success response | `(success :field val ...)` | `(success :id "x" :status complete)` |
| Error response | `(error :category cat :code code :message "msg")` | `(error :category not-found ...)` |
| Multi-variant | Distinct head symbols per variant | `(results ...)`, `(empty)`, `(pending ...)` |
| Enum values | Unquoted kebab-case symbols | `in-progress`, `read-only`, `high` |
| String values | Quoted, escaped | `"hello \"world\""` |
| Boolean values | Symbols `true` / `false` | `:verbose true` |
| Lists | Parenthesized | `("a" "b" "c")` |
| Optional fields | Omitted when None/empty | (field absent from output) |
| Domain data | Inline nested S-expr, not quoted | `:data (my-type ...)` |
| Legacy aliases | Accept old keyword, reject if both | `:project-id` / `:pid` |
