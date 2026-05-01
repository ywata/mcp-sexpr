//! Differential test corpus.
//!
//! For every `.sexpr` file in `tests/compat_corpus/`, asserts that the file's
//! single top-level S-expression parses to the same structural value through
//! both the new parser (then converted to `lexpr::Value`) and `lexpr::Parser`.
//!
//! The lexpr parser is configured with `NilSymbol::EmptyList` and
//! `KeywordSyntax::ColonPrefix` to match the new parser's canonicalization
//! choices — these are documented in `specs/parser/value-types.md` and are not
//! considered divergences.
//!
//! Any new edge case found in production should be added here as a new file.

mod common;

use std::fs;
use std::path::PathBuf;

use common::{covers, SpecItem};
use lexpr::parse::{KeywordSyntax, NilSymbol, Options};
use mcp_tools::parse_value;

fn corpus_dir() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("tests");
    p.push("compat_corpus");
    p
}

fn corpus_files() -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = fs::read_dir(corpus_dir())
        .expect("corpus dir exists")
        .filter_map(|entry| entry.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("sexpr"))
        .collect();
    out.sort();
    out
}

fn lexpr_options() -> Options {
    Options::default()
        .with_nil_symbol(NilSymbol::EmptyList)
        .with_keyword_syntax(KeywordSyntax::ColonPrefix)
}

fn parse_lexpr(input: &str) -> Result<lexpr::Value, String> {
    let mut parser = lexpr::Parser::from_str_custom(input, lexpr_options());
    parser.expect_value().map_err(|e| e.to_string())
}

#[test]
fn corpus_has_inputs() {
    covers!([SpecItem::McpToolsParserParseValue]);

    let files = corpus_files();
    assert!(
        files.len() >= 5,
        "corpus should cover at least 5 distinct categories, has {}",
        files.len()
    );
}

#[test]
fn every_corpus_file_parses_identically_through_both_parsers() {
    covers!([
        SpecItem::McpToolsParserDifferentialMode,
        SpecItem::McpToolsParserLexprConversion,
    ]);

    let mut failures: Vec<String> = Vec::new();

    for file in corpus_files() {
        let content = fs::read_to_string(&file).unwrap();
        let label = file.file_name().unwrap().to_string_lossy().into_owned();

        let lexpr_v = match parse_lexpr(&content) {
            Ok(v) => v,
            Err(e) => {
                failures.push(format!("{}: lexpr failed to parse: {}", label, e));
                continue;
            }
        };

        let new_v = match parse_value(&content) {
            Ok(v) => v,
            Err(e) => {
                failures.push(format!("{}: new parser failed: {}", label, e));
                continue;
            }
        };

        let converted: lexpr::Value = lexpr::Value::from(new_v.clone());
        if converted != lexpr_v {
            failures.push(format!(
                "{}: differential mismatch\n  lexpr  = {:?}\n  new    = {:?}\n  conv   = {:?}",
                label, lexpr_v, new_v, converted
            ));
        }
    }

    if !failures.is_empty() {
        panic!(
            "differential corpus reported {} failure(s):\n{}",
            failures.len(),
            failures.join("\n\n")
        );
    }
}

#[test]
fn corpus_files_round_trip_through_format_and_reparse() {
    covers!([SpecItem::McpToolsParserParseValue]);

    for file in corpus_files() {
        let content = fs::read_to_string(&file).unwrap();
        let label = file.file_name().unwrap().to_string_lossy().into_owned();

        let v = parse_value(&content)
            .unwrap_or_else(|e| panic!("{}: new parser failed: {}", label, e));

        let formatted = format!("{}", v);
        let reparsed = parse_value(&formatted)
            .unwrap_or_else(|e| panic!("{}: reparse of formatted output failed: {}", label, e));

        assert_eq!(v, reparsed, "{}: round-trip diverged", label);
    }
}
