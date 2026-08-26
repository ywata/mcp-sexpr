(goal "nil-expected-class"
  :spec (use "specs/changes/2026-08-26-nil-expected-class.md")
  :outputs ((file "specs/parser/differential-mode.md") (doc "spec-verified") (file "src/parser/differential.rs") (file "tests/parser_differential.rs") (doc "full-tests-passed"))
  :goals (
    (goal "spec-nil-expected-class"
      :spec "In specs/parser/differential-mode.md: (1) under '## Expected Discrepancy Classes' add the table row `Value::Nil` | `lexpr::Value::Symbol(\"nil\")` | equal, a sentence noting the rule is byte-exact per grammar.md (`NIL` is a symbol to both parsers), and extend the asymmetry paragraph to say new-parser Symbol(\"nil\") vs lexpr Nil/Null remains reported at atom; add the change spec's five test obligations to the '### Test obligations' list. (2) Under '## Operating Notes' add a 'nil' bullet to the 'Suppressed by rule' list pointing at the table. Spec-ids unchanged. Preserve: [All existing spec-ids and heading text unchanged, All other sections byte-identical]"
      :descr "transform"
      :atomic true
      :outputs ((file "specs/parser/differential-mode.md")))
    (goal "verify-spec-build"
      :spec "Run cargo build and confirm every ## heading in specs/parser/differential-mode.md still carries a spec-id and that the SpecItem enum is unchanged (no new variants expected). Run spec-trace check-links --dirs specs/ if available."
      :descr "check"
      :atomic true
      :depends ("spec-nil-expected-class")
      :inputs ((file "specs/parser/differential-mode.md"))
      :outputs ((doc "spec-verified")))
    (goal "impl-record-discrepancy-seam"
      :spec "In src/parser/differential.rs split record_discrepancy_if_diverging: keep the comparison + report construction there, and move the post-compare pipeline (input-hash dedup, class dedup, dispatch under the current mode's sink/verbose) into a pub(crate) fn record_discrepancy(report: Discrepancy) -> Result<(), DiscrepancyDispatchError>. The seam derives the input hash from DiscrepancyInput::Hashed { sha256 } directly, or sha256 of the source for DiscrepancyInput::Verbose. Order inside the seam must stay: input dedup -> class dedup -> dispatch (budget lives inside the Stderr sink). Pure refactor: no observable change through parse_value. Preserve: [record_discrepancy_if_diverging behaviour unchanged for every input, Dedup ordering: input hash, then class, then dispatch, All existing tests pass]"
      :descr "transform"
      :atomic true
      :depends ("verify-spec-build")
      :inputs ((doc "spec-verified"))
      :outputs ((file "src/parser/differential.rs")))
    (goal "repoint-class-dedup-tests"
      :spec "Move class_dedup_distinguishes_divergence_position and class_dedup_evicts_least_recently_seen_class from tests/parser_differential.rs into the #[cfg(test)] module of src/parser/differential.rs, rewritten to drive record_discrepancy with hand-built Discrepancy reports (both sides Ok, explicit StructuralPath 1.atom / 2.atom, distinct input hashes per report) under a Callback sink set via set_differential_mode and global_state_lock. Assertions and covers!([SpecItem::McpToolsParserDiscrepancyClassDeduplication]) are preserved exactly; the tests are moved, not deleted. The remaining three class-dedup integration tests (bignum-based) stay where they are. Run cargo test --lib and cargo test --test parser_differential. Preserve: [Every other test and its covers!() intact, Seam implementation unchanged, No test deleted: the two moved tests exist in the unit-test module with the same assertions]"
      :descr "transform"
      :atomic true
      :depends ("impl-record-discrepancy-seam")
      :inputs ((file "src/parser/differential.rs"))
      :outputs ((file "src/parser/differential.rs") (file "tests/parser_differential.rs")))
    (goal "impl-nil-expected-class"
      :spec "In src/parser/differential.rs add one arm to is_expected_class: (Value::Nil, lexpr::Value::Symbol(s)) => &**s == \"nil\". Byte-exact (decision q-case). No other arm changes; the reverse direction is NOT added. Run cargo test --lib, cargo test --test parser_differential, cargo test --test parser_differential_corpus. Preserve: [Every other is_expected_class and compare_values arm unchanged, parse_value return values bit-identical, All existing tests pass]"
      :descr "transform"
      :atomic true
      :depends ("repoint-class-dedup-tests")
      :inputs ((file "src/parser/differential.rs") (file "tests/parser_differential.rs"))
      :outputs ((file "src/parser/differential.rs")))
    (goal "test-nil-expected-class"
      :spec "Add tests, every one starting with covers!([SpecItem::McpToolsParserExpectedDiscrepancyClasses]). Unit tests in src/parser/differential.rs: is_expected_class(Nil, Symbol(\"nil\")) is true and compare_values returns None; the reverse pairs (new Symbol(\"nil\"), lexpr Nil) and (new Symbol(\"nil\"), lexpr Null) are reported at atom; (Nil, Symbol(\"NIL\")) is not expected. Integration tests in tests/parser_differential.rs under test_lock with a Callback sink: (a nil), (a (b (c nil))), (nil . nil) produce no discrepancy; (a NIL) IS reported (path 1.atom). Run cargo test --lib and cargo test --test parser_differential. Preserve: [All existing tests and their covers!() intact, Implementation unchanged]"
      :descr "transform"
      :atomic true
      :depends ("impl-nil-expected-class")
      :inputs ((file "src/parser/differential.rs"))
      :outputs ((file "src/parser/differential.rs") (file "tests/parser_differential.rs")))
    (goal "verify-full"
      :spec "cargo build; cargo test (full suite); cargo clippy --all-targets with no findings in differential.rs beyond the 3 pre-existing ones elsewhere; bin/check_coverage.sh with no uncovered parser/* spec-ids. Confirm the change spec is not in spec-files.txt and that new-spec-ids is (none)."
      :descr "check"
      :atomic true
      :depends ("test-nil-expected-class")
      :inputs ((file "src/parser/differential.rs") (file "tests/parser_differential.rs"))
      :outputs ((doc "full-tests-passed")))))
