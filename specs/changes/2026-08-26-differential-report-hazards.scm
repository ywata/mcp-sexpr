(goal "differential-report-hazards"
  :spec (use "specs/changes/2026-08-26-differential-report-hazards.md")
  :outputs ((file "specs/parser/differential-mode.md") (file "specs/migration/lexpr-deprecation.md") (doc "spec-items-regenerated") (file "src/parser/differential.rs") (file "tests/parser_differential.rs") (file "Cargo.toml") (doc "full-tests-passed") (file "docs/2026-08-26-differential-parse-stderr-reply.md"))
  :goals (
    (goal "spec-expected-classes"
      :spec "In specs/parser/differential-mode.md add a new second-level section '## Expected Discrepancy Classes' with <!-- spec-id: mcp-tools/parser/expected-discrepancy-classes --> placed immediately after '## Discrepancy Reporting'. Content per change spec section 'Expected discrepancy classes': definition of an expected class, the table (Keyword(k) vs lexpr Symbol(\":k\") is equal; Keyword vs Keyword equal), the explicit non-symmetry rule (new Symbol(\":k\") vs lexpr Keyword(\"k\") remains reported at atom), and the four test obligations. Also edit '## Operating Notes': move the 'Keyword normalization' bullet out of 'Expected discrepancy categories' into a new 'Suppressed by rule' list that points to the new section. Preserve: [All existing spec-ids and their heading text unchanged, All sections other than Operating Notes byte-identical apart from the inserted section]"
      :descr "transform"
      :atomic true
      :outputs ((file "specs/parser/differential-mode.md")))
    (goal "spec-hashed-line"
      :spec "In specs/parser/differential-mode.md rewrite '### Stderr sink format' under mcp-tools/parser/discrepancy-reporting per change spec section 'Hashed-mode stderr line omits tree contents': two format blocks (verbose:false prints new=<kind> lexpr=<kind> path=<path> then input-sha256=<hex>; verbose:true prints the existing <repr> form then input=<source>), the definition of <kind> as variant name only including Err(<kind>) with no message, the hashed-mode invariant (no byte derived from the source other than its SHA-256 reaches stderr), the statement that the Discrepancy struct passed to Callback sinks is unchanged, and the three test obligations. Mention that the line formatter is a pure fn format_report(&Discrepancy, verbose: bool) -> String. Preserve: [Spec-id mcp-tools/parser/discrepancy-reporting unchanged, Expected Discrepancy Classes section from the previous step intact, Callback sink subsection unchanged]"
      :descr "transform"
      :atomic true
      :depends ("spec-expected-classes")
      :inputs ((file "specs/parser/differential-mode.md"))
      :outputs ((file "specs/parser/differential-mode.md")))
    (goal "spec-stderr-budget"
      :spec "In specs/parser/differential-mode.md add a new second-level section '## Stderr Report Budget' with <!-- spec-id: mcp-tools/parser/stderr-report-budget --> placed immediately after '## Discrepancy Deduplication'. Content per change spec section 'Per-process stderr report budget': STDERR_REPORT_BUDGET = 64, the exact terminal line text, drop-before-format after exhaustion, counter never reset by set_differential_mode, Stderr-only (Callback unlimited), budget checked after both dedup caches, the < 16 KB bound and why it removes the deadlock, and the three test obligations (pure decision fn returning an Emit/EmitExhausted/Drop enum). Preserve: [All previously added sections intact, Discrepancy Deduplication section content unchanged in this step]"
      :descr "transform"
      :atomic true
      :depends ("spec-hashed-line")
      :inputs ((file "specs/parser/differential-mode.md"))
      :outputs ((file "specs/parser/differential-mode.md")))
    (goal "spec-class-dedup"
      :spec "In specs/parser/differential-mode.md add a new second-level section '## Discrepancy Class Deduplication' with <!-- spec-id: mcp-tools/parser/discrepancy-class-deduplication --> placed between '## Discrepancy Deduplication' and '## Stderr Report Budget'. Content per change spec section 'Class-keyed deduplication': the DiscrepancyClass triple (path with indices, new_kind, lexpr_kind) with ValueKind/LexprKind as enums not strings, the two-condition suppression rule (input LRU capacity 1024 AND class LRU default capacity 256), cache update on miss before dispatch, set_discrepancy_class_dedup_capacity(usize) with 0 disabling class dedup, the indices-kept rationale, and the five test obligations. Add one sentence to '## Discrepancy Deduplication' noting the second key and pointing to the new section. Preserve: [Spec-id mcp-tools/parser/discrepancy-deduplication unchanged, All previously added sections intact]"
      :descr "transform"
      :atomic true
      :depends ("spec-stderr-budget")
      :inputs ((file "specs/parser/differential-mode.md"))
      :outputs ((file "specs/parser/differential-mode.md")))
    (goal "spec-version-applicability"
      :spec "In specs/parser/differential-mode.md add '## Version Applicability' with <!-- spec-id: mcp-tools/parser/version-applicability non-testable --> directly after the overview paragraph and before '## Differential Mode' (or the first existing ## heading). Land the exact text from change spec section 'Version applicability': default-on applies to >= 0.3.0, < 0.4.0; < 0.3.0 ships no comparison; 0.4.0+ ships Off; determinable from Cargo.lock alone. Preserve: [All other sections intact]"
      :descr "transform"
      :atomic true
      :depends ("spec-class-dedup")
      :inputs ((file "specs/parser/differential-mode.md"))
      :outputs ((file "specs/parser/differential-mode.md")))
    (goal "spec-deprecation-xref"
      :spec "In specs/migration/lexpr-deprecation.md, under '## Phase 1 — 0.3 Surface', add a single sentence cross-referencing specs/parser/differential-mode.md '## Version Applicability' for the exact version range in which differential mode is default-on. No spec-id changes. Preserve: [All existing spec-ids and content unchanged apart from the added sentence]"
      :descr "transform"
      :atomic true
      :depends ("spec-class-dedup")
      :inputs ((file "specs/parser/differential-mode.md"))
      :outputs ((file "specs/migration/lexpr-deprecation.md")))
    (goal "regenerate-spec-items"
      :spec "Run cargo build and confirm the four new SpecItem variants exist in the generated traceability code: McpToolsParserExpectedDiscrepancyClasses, McpToolsParserDiscrepancyClassDeduplication, McpToolsParserStderrReportBudget, McpToolsParserVersionApplicability. Also run spec-trace check-links --dirs specs/ if the CLI is available, and confirm every ## heading in differential-mode.md still carries a spec-id."
      :descr "check"
      :atomic true
      :depends ("spec-version-applicability" "spec-deprecation-xref")
      :inputs ((file "specs/parser/differential-mode.md") (file "specs/migration/lexpr-deprecation.md"))
      :outputs ((doc "spec-items-regenerated")))
    (goal "impl-expected-classes"
      :spec "In src/parser/differential.rs, add an expected-class rule to compare_values: (Value::Keyword(k), lexpr::Value::Symbol(s)) where s == format!(\":{k}\") returns None. Do NOT add the reverse direction. Express the rule as a small pure fn is_expected_class(&Value, &lexpr::Value) -> bool called from compare_values, so the class table is one place. Run cargo test --test parser_differential and cargo test --lib afterwards; the existing corpus test tests/parser_differential_corpus.rs must still pass. Preserve: [Every existing compare_values arm and its result unchanged, parse_value return values bit-identical to before, All existing tests pass]"
      :descr "transform"
      :atomic true
      :depends ("regenerate-spec-items")
      :inputs ((doc "spec-items-regenerated"))
      :outputs ((file "src/parser/differential.rs")))
    (goal "test-expected-classes"
      :spec "Add tests for mcp-tools/parser/expected-discrepancy-classes. In tests/parser_differential.rs (respecting the file's serialisation convention for global state), using a Callback sink: (tool :key \"v\") yields no discrepancy; (:a . :b) yields none; a keyword nested three lists deep yields none. In the #[cfg(test)] module of src/parser/differential.rs: the reverse pair (new Symbol(\":k\"), lexpr Keyword(\"k\")) fed to compare_values IS reported at path atom. Every test starts with covers!([SpecItem::McpToolsParserExpectedDiscrepancyClasses]). Run cargo test --test parser_differential and cargo test --lib. Preserve: [All existing tests and their covers!() intact, Implementation from the previous step unchanged]"
      :descr "transform"
      :atomic true
      :depends ("impl-expected-classes")
      :inputs ((file "src/parser/differential.rs"))
      :outputs ((file "tests/parser_differential.rs") (file "src/parser/differential.rs")))
    (goal "impl-format-report"
      :spec "In src/parser/differential.rs extract the stderr line formatting into a pure fn format_report(report: &Discrepancy, verbose: bool) -> String. In hashed mode (verbose=false) emit new=<kind> lexpr=<kind> where <kind> is the variant name only: for Ok values the Value / lexpr::Value variant name (e.g. Keyword, Symbol, List, Cons); for Err on the new side Err(<ParseErrorRepr.kind>) with no message; for Err on the lexpr side Err with no message text. Then the input-sha256 line. In verbose mode keep the existing Ok(<debug>) / Err(<kind>: <message>) form and the input= line. write_stderr becomes eprint!(\"{}\", format_report(..)). Introduce ValueKind and LexprKind enums (with Display) for the variant names — they are reused by class dedup in a later step — with an exhaustive match over Value so a new variant is a compile error. Preserve: [Verbose-mode output byte-identical to before, Discrepancy struct fields unchanged, Callback sink receives the same Discrepancy as before, All existing tests pass]"
      :descr "transform"
      :atomic true
      :depends ("test-expected-classes")
      :inputs ((file "tests/parser_differential.rs") (file "src/parser/differential.rs"))
      :outputs ((file "src/parser/differential.rs")))
    (goal "test-hashed-line"
      :spec "Add tests for the modified mcp-tools/parser/discrepancy-reporting using format_report directly (unit tests in the src/parser/differential.rs test module, since the fn is crate-private; make it pub(crate)). Cases: hashed mode with an input containing SECRET-9f3a engineered to diverge (e.g. a bignum sibling) — output does not contain SECRET-9f3a; verbose mode, same input — output contains it; hashed mode with an Err on one side — output contains the error kind and not the message text. Every test starts with covers!([SpecItem::McpToolsParserDiscrepancyReporting]). Run cargo test --lib and cargo test --test parser_differential. Preserve: [All existing tests and their covers!() intact, format_report implementation unchanged]"
      :descr "transform"
      :atomic true
      :depends ("impl-format-report")
      :inputs ((file "src/parser/differential.rs"))
      :outputs ((file "src/parser/differential.rs")))
    (goal "impl-stderr-budget"
      :spec "In src/parser/differential.rs implement the per-process stderr report budget. Add const STDERR_REPORT_BUDGET: usize = 64, a process-global monotonic AtomicUsize counter (never reset by set_differential_mode), and a pure fn budget_decision(count_before: usize, budget: usize) -> BudgetDecision where enum BudgetDecision { Emit, EmitExhausted, Drop }. In dispatch, for DiscrepancySink::Stderr only, fetch_add the counter and act on the decision: Emit writes format_report; EmitExhausted writes exactly the terminal line '[mcp-tools differential] report budget (64) exhausted; further reports suppressed. Use DiscrepancySink::Callback or MCP_TOOLS_DIFFERENTIAL_PARSE=off.'; Drop returns before formatting. The budget check must run after both dedup caches (it stays inside dispatch, which is already after dedup). Callback sink is untouched. Preserve: [Callback sink behaviour unchanged, Dedup ordering: dedup runs before the budget check, format_report and expected-class rule unchanged, All existing tests pass]"
      :descr "transform"
      :atomic true
      :depends ("test-hashed-line")
      :inputs ((file "src/parser/differential.rs"))
      :outputs ((file "src/parser/differential.rs")))
    (goal "test-stderr-budget"
      :spec "Add tests for mcp-tools/parser/stderr-report-budget in the src/parser/differential.rs test module: drive budget_decision over counts 0..70 with budget 64 and assert 64 Emit, 1 EmitExhausted, 5 Drop; assert that switching mode to Callback and back does not reset the global counter (expose a pub(crate) accessor for the counter value); assert dedup-before-budget by feeding a duplicate input after 63 fresh reports and checking the counter did not advance. Every test starts with covers!([SpecItem::McpToolsParserStderrReportBudget]). Note that the global counter is shared across the test binary — tests must reason about deltas, not absolute values. Run cargo test --lib and cargo test --test parser_differential. Preserve: [All existing tests and their covers!() intact, Budget implementation unchanged]"
      :descr "transform"
      :atomic true
      :depends ("impl-stderr-budget")
      :inputs ((file "src/parser/differential.rs"))
      :outputs ((file "src/parser/differential.rs")))
    (goal "impl-class-dedup"
      :spec "In src/parser/differential.rs add class-keyed deduplication. Define struct DiscrepancyClass { path: StructuralPath, new_kind: ValueKind-or-error-kind, lexpr_kind: LexprKind-or-Err } (Hash + Eq), a second bounded LRU in GlobalState with default capacity 256, and pub fn set_discrepancy_class_dedup_capacity(usize) where 0 disables class dedup. In record_discrepancy_if_diverging, a report is dispatched only if the input hash misses the input LRU AND the class misses the class LRU; both caches record on miss before dispatch. Reuse the existing DedupCache type generically or by parameterising the key. Keep the existing set_discrepancy_dedup_capacity semantics (input cache only). Preserve: [Input-hash dedup behaviour and capacity unchanged, Budget, format_report, and expected-class rule unchanged, All existing tests pass]"
      :descr "transform"
      :atomic true
      :depends ("test-stderr-budget")
      :inputs ((file "src/parser/differential.rs"))
      :outputs ((file "src/parser/differential.rs")))
    (goal "test-class-dedup"
      :spec "Add tests for mcp-tools/parser/discrepancy-class-deduplication in tests/parser_differential.rs (respecting the file's serialisation convention), each using a counting Callback sink and resetting both caches via the capacity setters: 100 distinct inputs all diverging as (1.atom, Integer, bignum) → callback invoked once; two inputs diverging at 1.atom and 2.atom → twice; same input twice → once; set_discrepancy_class_dedup_capacity(0) then 100 distinct inputs → 100 times; class capacity 2 with three classes in rotation → first class reported again after eviction. Every test starts with covers!([SpecItem::McpToolsParserDiscrepancyClassDeduplication]). Run cargo test --test parser_differential and cargo test --lib. Preserve: [All existing tests and their covers!() intact, Implementation unchanged]"
      :descr "transform"
      :atomic true
      :depends ("impl-class-dedup")
      :inputs ((file "src/parser/differential.rs"))
      :outputs ((file "tests/parser_differential.rs")))
    (goal "bump-crate-version"
      :spec "Set version = \"0.3.0\" in Cargo.toml (decision q-version, confirmed 2026-08-26). Update Cargo.lock via cargo build. No other manifest changes. Preserve: [All dependencies, features, and metadata other than version unchanged]"
      :descr "transform"
      :atomic true
      :depends ("test-class-dedup")
      :inputs ((file "tests/parser_differential.rs"))
      :outputs ((file "Cargo.toml")))
    (goal "verify-full"
      :spec "Full verification: cargo build, cargo test (whole suite), cargo clippy if configured, then bin/check_coverage.sh (or SPEC_TRACE_DB=coverage.db cargo test followed by the spec-trace coverage report) and confirm the four new testable spec-ids plus the modified discrepancy-reporting spec-id each have at least one covering test. Confirm all new-spec-ids from the change spec metadata exist in specs/parser/differential-mode.md. Confirm specs/changes/ files are not in spec-files.txt."
      :descr "check"
      :atomic true
      :depends ("bump-crate-version")
      :inputs ((file "Cargo.toml"))
      :outputs ((doc "full-tests-passed")))
    (goal "draft-origin-reply"
      :spec "Write docs/2026-08-26-differential-parse-stderr-reply.md: a reply to the origin report docs/2026-08-26-differential-parse-stderr.md addressed to the deep-rev project. Per item: D1 fixed (expected-class suppression — keyword payloads now produce zero reports), D2 fixed (class-keyed dedup, default capacity 256), D3 fixed both halves (hashed mode prints variant kinds only; stderr sink capped at 64 reports per process, < 16 KB total), A1 fixed (version-applicability section; crate is now 0.3.0). State that the default sink stays Stderr and why. Recommend DiscrepancySink::Callback forwarding to their own logger for a stdio server that wants unbounded reports. Include the exact rev to pin once committed (leave a placeholder if not yet committed)."
      :descr "realize"
      :atomic true
      :depends ("verify-full")
      :inputs ((doc "full-tests-passed"))
      :outputs ((file "docs/2026-08-26-differential-parse-stderr-reply.md")))))
