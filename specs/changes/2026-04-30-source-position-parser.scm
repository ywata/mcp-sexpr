(goal "source-position-parser"
  :spec (use "specs/changes/2026-04-30-source-position-parser.md")
  :outputs ((doc "bootstrap-decision") (doc "reexport-decision") (doc "verbose-decision") (doc "tracing-decision") (doc "indexing-decision") (directory "specs/parser") (directory "specs/migration") (file "spec-files.txt") (file "build.rs") (file "Cargo.toml") (file "specs/parser/grammar.md") (file "specs/parser/value-types.md") (file "specs/parser/source-positions.md") (file "specs/parser/api.md") (file "specs/parser/differential-mode.md") (file "specs/migration/lexpr-deprecation.md") (doc "spec-trace-gen-ok") (file "src/parser/mod.rs") (file "src/parser/types.rs") (file "src/parser/lexer.rs") (file "src/parser/reader.rs") (file "src/parser/lexpr_compat.rs") (file "src/parser/differential.rs") (file "src/lib.rs") (file "src/extract/args.rs") (file "src/format/response.rs") (file "tests/parser_grammar.rs") (file "tests/parser_types.rs") (file "tests/parser_positions.rs") (file "tests/parser_api.rs") (file "tests/parser_lexpr_conversion.rs") (file "tests/parser_differential.rs") (directory "tests/compat_corpus") (file "tests/parser_differential_corpus.rs") (doc "0.3-implementation-verified"))
  :goals (
    (lazy "decide-decide-bootstrap-infra"
      :human-instructions "Decide whether to bootstrap spec-trace infrastructure (specs/, spec-files.txt, build.rs, spec-trace build+runtime deps) as part of this plan, or defer to a precursor change spec. Default position: bootstrap here, since this is the first change spec in mcp-sexpr. If the decision is 'defer', abandon this plan and resume only after the precursor lands."
      :goal (goal "decide-bootstrap-infra"
        :spec "Decide whether to bootstrap spec-trace infrastructure (specs/, spec-files.txt, build.rs, spec-trace build+runtime deps) as part of this plan, or defer to a precursor change spec. Default position: bootstrap here, since this is the first change spec in mcp-sexpr. If the decision is 'defer', abandon this plan and resume only after the precursor lands."
        :outputs ((doc "bootstrap-decision"))))
    (lazy "decide-decide-lexpr-reexport"
      :human-instructions "Decide whether mcp_tools::lexpr re-exports the lexpr crate during the 0.3 migration window so consumers do not need a separate Cargo dependency, or whether consumers must declare lexpr themselves."
      :goal (goal "decide-lexpr-reexport"
        :spec "Decide whether mcp_tools::lexpr re-exports the lexpr crate during the 0.3 migration window so consumers do not need a separate Cargo dependency, or whether consumers must declare lexpr themselves."
        :outputs ((doc "reexport-decision"))))
    (lazy "decide-decide-verbose-interface"
      :human-instructions "Decide how discrepancy verbose mode is configured: env var only (MCP_TOOLS_DIFFERENTIAL_PARSE=verbose), API only (set_differential_mode with verbose flag), or both."
      :goal (goal "decide-verbose-interface"
        :spec "Decide how discrepancy verbose mode is configured: env var only (MCP_TOOLS_DIFFERENTIAL_PARSE=verbose), API only (set_differential_mode with verbose flag), or both."
        :outputs ((doc "verbose-decision"))))
    (lazy "decide-decide-tracing-sink"
      :human-instructions "Decide whether DiscrepancySink::Tracing ships in 0.3 behind a tracing feature flag, or whether tracing integration is deferred to a later release."
      :goal (goal "decide-tracing-sink"
        :spec "Decide whether DiscrepancySink::Tracing ships in 0.3 behind a tracing feature flag, or whether tracing integration is deferred to a later release."
        :outputs ((doc "tracing-decision"))))
    (lazy "decide-decide-position-indexing"
      :human-instructions "Decide line/column indexing convention for Position: 1-indexed for human display (most common), 0-indexed for LSP compatibility, or both via separate accessors."
      :goal (goal "decide-position-indexing"
        :spec "Decide line/column indexing convention for Position: 1-indexed for human display (most common), 0-indexed for LSP compatibility, or both via separate accessors."
        :outputs ((doc "indexing-decision"))))
    (goal "create-specs-tree"
      :spec "Create specs/parser/ and specs/migration/ directories. This is the first specs/ content in mcp-sexpr."
      :descr "realize"
      :atomic true
      :depends ("decide-decide-lexpr-reexport" "decide-decide-verbose-interface" "decide-decide-tracing-sink" "decide-decide-position-indexing" "decide-decide-bootstrap-infra")
      :inputs ((doc "bootstrap-decision"))
      :outputs ((directory "specs/parser") (directory "specs/migration")))
    (goal "create-spec-files-txt"
      :spec "Create spec-files.txt at the repository root, initially empty. Permanent spec file paths are appended in a later step once the files exist."
      :descr "realize"
      :atomic true
      :depends ("create-specs-tree")
      :inputs ((directory "specs/parser") (directory "specs/migration"))
      :outputs ((file "spec-files.txt")))
    (goal "create-build-rs"
      :spec "Create build.rs that invokes spec-trace codegen against spec-files.txt to generate src/traceability_gen.rs (containing the SpecItem enum)."
      :descr "realize"
      :atomic true
      :depends ("create-spec-files-txt")
      :inputs ((file "spec-files.txt"))
      :outputs ((file "build.rs")))
    (goal "add-spec-trace-deps"
      :spec "Add spec-trace as both [build-dependencies] (for codegen) and [dependencies] (for the runtime covers!() macro and SpecItem enum) in Cargo.toml. Preserve: [Existing crate metadata (name, version, edition, license, repository, documentation), All existing feature flags and their dependency relationships, All existing runtime dependencies (anyhow, lexpr, thiserror, optional features)]"
      :descr "transform"
      :atomic true
      :depends ("create-build-rs")
      :inputs ((file "build.rs"))
      :outputs ((file "Cargo.toml")))
    (goal "spec-grammar"
      :spec "Write specs/parser/grammar.md. Spec-ids on second-level headings: mcp-tools/parser/grammar (accepted forms — atoms, lists, dotted pairs, quotes), mcp-tools/parser/string-escapes (the five recognized escapes plus error policy), mcp-tools/parser/comments (line and block comment forms). Mark non-testable rationale sections appropriately."
      :descr "realize"
      :atomic true
      :depends ("add-spec-trace-deps")
      :inputs ((file "Cargo.toml"))
      :outputs ((file "specs/parser/grammar.md")))
    (goal "spec-value-types"
      :spec "Write specs/parser/value-types.md. Spec-ids: mcp-tools/parser/value-type (Value enum), mcp-tools/parser/spanned-type (Spanned + SpannedNode), mcp-tools/parser/numeric-tower (i64/f64 only with rationale), mcp-tools/parser/keyword-canonicalization (no leading colon stored), mcp-tools/parser/list-representation (Vec-backed proper lists; Pair only for dotted)."
      :descr "realize"
      :atomic true
      :depends ("add-spec-trace-deps")
      :inputs ((file "Cargo.toml"))
      :outputs ((file "specs/parser/value-types.md")))
    (goal "spec-source-positions"
      :spec "Write specs/parser/source-positions.md. Spec-ids: mcp-tools/parser/spans (Position struct, indexing convention from q-indexing decision, byte_offset semantics, 4GB source limit), mcp-tools/parser/comment-retention (leading_comments and trailing_comments on Spanned)."
      :descr "realize"
      :atomic true
      :depends ("add-spec-trace-deps" "decide-decide-position-indexing")
      :inputs ((file "Cargo.toml") (doc "indexing-decision"))
      :outputs ((file "specs/parser/source-positions.md")))
    (goal "spec-api"
      :spec "Write specs/parser/api.md. Spec-ids: mcp-tools/parser/parse-value (new Value-returning signature), mcp-tools/parser/parse-value-with-positions (Spanned-returning), mcp-tools/parser/lexpr-conversion (bidirectional From impls; lossy direction errors), mcp-tools/parser/api-deprecation (every lexpr::Value-based helper has a Value-based counterpart and a #[deprecated] note). Incorporate the q-reexport decision."
      :descr "realize"
      :atomic true
      :depends ("add-spec-trace-deps" "decide-decide-lexpr-reexport")
      :inputs ((file "Cargo.toml") (doc "reexport-decision"))
      :outputs ((file "specs/parser/api.md")))
    (goal "spec-differential"
      :spec "Write specs/parser/differential-mode.md. Spec-ids: mcp-tools/parser/differential-mode (DifferentialMode/DiscrepancySink types, default-on policy in 0.3), mcp-tools/parser/discrepancy-reporting (Discrepancy fields, hashed-vs-verbose input handling per q-verbose, optional tracing sink per q-tracing), mcp-tools/parser/discrepancy-deduplication (LRU bound, flush API)."
      :descr "realize"
      :atomic true
      :depends ("add-spec-trace-deps" "decide-decide-verbose-interface" "decide-decide-tracing-sink")
      :inputs ((file "Cargo.toml") (doc "verbose-decision") (doc "tracing-decision"))
      :outputs ((file "specs/parser/differential-mode.md")))
    (goal "spec-migration"
      :spec "Write specs/migration/lexpr-deprecation.md. Spec-ids: mcp-tools/migration/phase-1-deprecation (0.3 surface), mcp-tools/migration/phase-2-window (0.4 deprecation window with diff-mode opt-in), mcp-tools/migration/phase-3-removal (1.0 lexpr removal and drop criteria), mcp-tools/migration/numeric-tower-loss (lexpr→Value conversion errors on out-of-range numerics), mcp-tools/migration/lexpr-conversion-lossy (consumer migration guidance for rationals/bignums)."
      :descr "realize"
      :atomic true
      :depends ("add-spec-trace-deps")
      :inputs ((file "Cargo.toml"))
      :outputs ((file "specs/migration/lexpr-deprecation.md")))
    (goal "register-spec-files"
      :spec "Append all 6 new permanent spec file paths to spec-files.txt. Do NOT add the change spec at specs/changes/ — change specs are documentation only and never enter the build system. Preserve: [All previously listed paths (initially empty on first creation)]"
      :descr "transform"
      :atomic true
      :depends ("spec-grammar" "spec-value-types" "spec-source-positions" "spec-api" "spec-differential" "spec-migration")
      :inputs ((file "specs/parser/grammar.md") (file "specs/parser/value-types.md") (file "specs/parser/source-positions.md") (file "specs/parser/api.md") (file "specs/parser/differential-mode.md") (file "specs/migration/lexpr-deprecation.md"))
      :outputs ((file "spec-files.txt")))
    (goal "verify-spec-trace-gen"
      :spec "Run cargo build to regenerate src/traceability_gen.rs. The build must succeed and the SpecItem enum must contain a variant for every testable spec-id from the new permanent specs."
      :descr "check"
      :atomic true
      :depends ("register-spec-files")
      :inputs ((file "spec-files.txt"))
      :outputs ((doc "spec-trace-gen-ok")))
    (goal "create-parser-module"
      :spec "Create src/parser/mod.rs declaring submodules (types, lexer, reader, lexpr_compat, differential) and re-exporting the public surface (Value, Spanned, Position, Span, Comment, parser entry points)."
      :descr "realize"
      :atomic true
      :depends ("verify-spec-trace-gen")
      :inputs ((doc "spec-trace-gen-ok"))
      :outputs ((file "src/parser/mod.rs")))
    (goal "impl-parser-types"
      :spec "Implement value types in src/parser/types.rs: Value enum (Nil, Bool, Integer(i64), Float(f64), String, Symbol, Keyword, List(Vec<Value>), Pair(Box<(Value,Value)>)), Spanned struct (value + span + leading_comments + trailing_comments), SpannedNode mirror enum, Position (line/column/byte_offset per q-indexing), Span, Comment. Provide as_* accessors mirroring the lexpr::Value surface."
      :descr "realize"
      :atomic true
      :depends ("create-parser-module")
      :inputs ((file "src/parser/mod.rs"))
      :outputs ((file "src/parser/types.rs")))
    (goal "impl-parser-lexer"
      :spec "Implement hand-rolled tokenizer in src/parser/lexer.rs producing a token stream with byte spans. Handles parens, dot, quotes (' ` , ,@), atoms (symbols/keywords/numbers/booleans/nil), strings with escape recognition (\\\\ \\\" \\n \\r \\t — error on others), line comments (;), block comments (#| |#), whitespace tracking. Tracks line/column for position computation."
      :descr "realize"
      :atomic true
      :depends ("impl-parser-types")
      :inputs ((file "src/parser/types.rs"))
      :outputs ((file "src/parser/lexer.rs")))
    (goal "impl-parser-reader"
      :spec "Implement recursive-descent reader in src/parser/reader.rs producing Spanned values from the lexer's token stream. Desugars 'expr/`expr/,expr/,@expr to (quote …) etc. Handles dotted pairs, proper-list-as-Vec collection, leading/trailing comment attachment. Returns Result<Spanned> with parse errors carrying source positions."
      :descr "realize"
      :atomic true
      :depends ("impl-parser-lexer")
      :inputs ((file "src/parser/lexer.rs"))
      :outputs ((file "src/parser/reader.rs")))
    (goal "impl-lexpr-conversion"
      :spec "Implement bidirectional conversion in src/parser/lexpr_compat.rs: From<lexpr::Value> for Value (errors on rationals/complex/bignums outside i64 range — no silent truncation) and From<Value> for lexpr::Value (total). Keyword normalization collapses lexpr's mixed Keyword/Symbol(:foo) handling into canonical Keyword(name)."
      :descr "realize"
      :atomic true
      :depends ("impl-parser-reader")
      :inputs ((file "src/parser/reader.rs"))
      :outputs ((file "src/parser/lexpr_compat.rs")))
    (goal "impl-differential-mode"
      :spec "Implement DifferentialMode/DiscrepancySink/Discrepancy and the runtime comparison wrapper in src/parser/differential.rs. Reads MCP_TOOLS_DIFFERENTIAL_PARSE per q-verbose; supports Stderr/Callback (and optionally Tracing per q-tracing). Bounded LRU dedup of input hashes (default 1024). Reports first divergence with structural path. Reporting is non-fatal — caller always receives the new parser's result."
      :descr "realize"
      :atomic true
      :depends ("impl-lexpr-conversion")
      :inputs ((file "src/parser/lexpr_compat.rs"))
      :outputs ((file "src/parser/differential.rs")))
    (goal "wire-public-api"
      :spec "Wire the new parser into the public API. In src/lib.rs: rename existing parse_value to parse_value_lexpr (mark #[deprecated]); add new parse_value -> Result<Value> and parse_value_with_positions -> Result<Spanned> routed through src/parser/ with differential-mode wrapping; add Value-based counterparts for every existing helper (get_kw_value, get_kw_str, require_kw_str, iter_list, parse_str_list, parse_text_ref, render_text_ref) and mark the lexpr::Value versions #[deprecated]. In src/extract/args.rs and src/format/response.rs: update internal callers to use the deprecated _lexpr forms (or new Value forms where straightforward) so the workspace continues to compile. Preserve: [src/lib.rs: every existing public function remains callable (with deprecation warnings) until 1.0; TextRef enum, quote_str, render_list semantics unchanged; existing in-module unit tests pass without modification beyond expected deprecation warnings, src/extract/args.rs: parse_tool_call and the require_*/get_* helpers continue to return their existing types under deprecation; behavior on existing test inputs is byte-for-byte identical, src/format/response.rs: format_success/format_error/format_complete/format_blocked/serialize_string_list/serialize_resource produce identical output strings for identical inputs, Differential mode default-on with stderr sink; no consumer-visible behavior change beyond compiler deprecation warnings]"
      :descr "transform"
      :atomic true
      :depends ("impl-differential-mode")
      :inputs ((file "src/parser/differential.rs"))
      :outputs ((file "src/lib.rs") (file "src/extract/args.rs") (file "src/format/response.rs")))
    (goal "tests-grammar"
      :spec "Add tests/parser_grammar.rs covering atoms, lists, dotted pairs, quote forms, all five recognized string escapes, error on unrecognized backslash escapes, line comments, block comments. covers!([McpToolsParserGrammar, McpToolsParserStringEscapes, McpToolsParserComments])."
      :descr "realize"
      :atomic true
      :depends ("wire-public-api")
      :inputs ((file "src/lib.rs") (file "src/extract/args.rs") (file "src/format/response.rs"))
      :outputs ((file "tests/parser_grammar.rs")))
    (goal "tests-types"
      :spec "Add tests/parser_types.rs covering Value construction, Spanned::into_value, numeric-tower limits at i64 boundary (no rational support), keyword canonicalization (no leading colon), Vec-backed list ergonomics, Pair only for dotted forms. covers!([McpToolsParserValueType, McpToolsParserSpannedType, McpToolsParserNumericTower, McpToolsParserKeywordCanonicalization, McpToolsParserListRepresentation])."
      :descr "realize"
      :atomic true
      :depends ("wire-public-api")
      :inputs ((file "src/lib.rs") (file "src/extract/args.rs") (file "src/format/response.rs"))
      :outputs ((file "tests/parser_types.rs")))
    (goal "tests-positions"
      :spec "Add tests/parser_positions.rs covering Span correctness on nested forms, multi-byte char position handling, CRLF handling, indexing convention from q-indexing, leading/trailing comment retention through parse-and-walk. covers!([McpToolsParserSpans, McpToolsParserCommentRetention])."
      :descr "realize"
      :atomic true
      :depends ("wire-public-api")
      :inputs ((file "src/lib.rs") (file "src/extract/args.rs") (file "src/format/response.rs"))
      :outputs ((file "tests/parser_positions.rs")))
    (goal "tests-api"
      :spec "Add tests/parser_api.rs covering parse_value -> Value, parse_value_with_positions -> Spanned, parse_value_lexpr backward compatibility, error round-trip. Use trybuild to confirm #[deprecated] attributes are present on the lexpr-based public surface. covers!([McpToolsParserParseValue, McpToolsParserParseValueWithPositions, McpToolsParserApiDeprecation])."
      :descr "realize"
      :atomic true
      :depends ("wire-public-api")
      :inputs ((file "src/lib.rs") (file "src/extract/args.rs") (file "src/format/response.rs"))
      :outputs ((file "tests/parser_api.rs")))
    (goal "tests-lexpr-conversion"
      :spec "Add tests/parser_lexpr_conversion.rs covering Value -> lexpr::Value (round-trip equality), lexpr::Value -> Value with errors on rationals/complex/bignums (no silent truncation), keyword/symbol disambiguation. covers!([McpToolsParserLexprConversion, McpToolsMigrationNumericTowerLoss, McpToolsMigrationLexprConversionLossy])."
      :descr "realize"
      :atomic true
      :depends ("wire-public-api")
      :inputs ((file "src/lib.rs") (file "src/extract/args.rs") (file "src/format/response.rs"))
      :outputs ((file "tests/parser_lexpr_conversion.rs")))
    (goal "tests-differential"
      :spec "Add tests/parser_differential.rs covering sink dispatch (Stderr, Callback, optional Tracing), LRU dedup behavior at the bound, env var override per q-verbose, non-fatal reporting (caller always receives a result). covers!([McpToolsParserDifferentialMode, McpToolsParserDiscrepancyReporting, McpToolsParserDiscrepancyDeduplication])."
      :descr "realize"
      :atomic true
      :depends ("wire-public-api")
      :inputs ((file "src/lib.rs") (file "src/extract/args.rs") (file "src/format/response.rs"))
      :outputs ((file "tests/parser_differential.rs")))
    (goal "seed-differential-corpus"
      :spec "Create tests/compat_corpus/ with curated S-expression inputs covering edge cases: numeric tower (integers, floats, negatives, i64 boundary), all string escapes plus unicode and control chars, dotted pairs at varying depths, deeply nested lists, line and block comments mixed with values, keyword/symbol distinction. Add tests/parser_differential_corpus.rs that for every input asserts lexpr::from_str(input) structurally equals lexpr::Value::from(parse_value(input)?). Seeds the long-term differential-test harness."
      :descr "realize"
      :atomic true
      :depends ("tests-grammar" "tests-types" "tests-positions" "tests-api" "tests-lexpr-conversion" "tests-differential")
      :inputs ((file "tests/parser_grammar.rs") (file "tests/parser_types.rs") (file "tests/parser_positions.rs") (file "tests/parser_api.rs") (file "tests/parser_lexpr_conversion.rs") (file "tests/parser_differential.rs"))
      :outputs ((directory "tests/compat_corpus") (file "tests/parser_differential_corpus.rs")))
    (goal "verify-build-and-coverage"
      :spec "Final gate: SPEC_TRACE_DB=coverage.db cargo test must succeed; bin/check_coverage.sh must report coverage for every testable spec-id introduced by this change spec; the differential corpus run must report zero discrepancies."
      :descr "check"
      :atomic true
      :depends ("seed-differential-corpus")
      :inputs ((directory "tests/compat_corpus") (file "tests/parser_differential_corpus.rs"))
      :outputs ((doc "0.3-implementation-verified")))))
