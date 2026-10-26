//! Guard for the rule that tests record coverage through spec-trace's
//! `covers!`, never a local definition that only type-checks.
//!
//! Spec: `specs/development/covers-macro.md`.

mod common;

use common::{covers, SpecItem};
use std::fs;
use std::path::{Path, PathBuf};

/// The forbidden definition head, assembled so that no line of this file
/// starts with it (the sweep below also reads this file).
const LOCAL_DEFINITION: &str = concat!("macro_rules!", " covers");

/// 1-based line numbers of `lines` that start a local `covers!` definition.
fn local_covers_definitions<'a>(lines: impl Iterator<Item = (usize, &'a str)>) -> Vec<usize> {
    lines
        .filter(|(_, line)| {
            line.trim_start()
                .strip_prefix(LOCAL_DEFINITION)
                .is_some_and(|rest| !rest.starts_with(|c: char| c.is_alphanumeric() || c == '_'))
        })
        .map(|(number, _)| number)
        .collect()
}

fn numbered(text: &str) -> impl Iterator<Item = (usize, &str)> {
    text.lines()
        .enumerate()
        .map(|(index, line)| (index + 1, line))
}

/// Lines inside fenced code blocks of a markdown text, with their line numbers.
fn code_block_lines(markdown: &str) -> impl Iterator<Item = (usize, &str)> {
    numbered(markdown)
        .scan(false, |in_block, (number, line)| {
            let fence = line.trim_start().starts_with("```");
            let inside = *in_block && !fence;
            *in_block ^= fence;
            Some((inside, number, line))
        })
        .filter(|(inside, _, _)| *inside)
        .map(|(_, number, line)| (number, line))
}

fn rust_files(dir: &Path) -> Vec<PathBuf> {
    fs::read_dir(dir)
        .unwrap_or_else(|err| panic!("cannot read {}: {err}", dir.display()))
        .map(|entry| entry.expect("directory entry").path())
        .flat_map(|path| {
            if path.is_dir() {
                rust_files(&path)
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                vec![path]
            } else {
                vec![]
            }
        })
        .collect()
}

fn hits(path: &Path, numbers: Vec<usize>) -> Vec<String> {
    numbers
        .into_iter()
        .map(|number| format!("{}:{number}", path.display()))
        .collect()
}

#[test]
fn no_local_covers_definition_in_sources_or_claude_md() {
    covers!([SpecItem::McpToolsDevelopmentCoversMustRecord]);
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));

    let source_hits = ["src", "tests"]
        .iter()
        .flat_map(|dir| rust_files(&root.join(dir)))
        .flat_map(|path| {
            let text = fs::read_to_string(&path).expect("readable source file");
            hits(&path, local_covers_definitions(numbered(&text)))
        });
    let claude_md = root.join("CLAUDE.md");
    let claude_text = fs::read_to_string(&claude_md).expect("readable CLAUDE.md");
    let claude_hits = hits(
        &claude_md,
        local_covers_definitions(code_block_lines(&claude_text)),
    );
    let all: Vec<String> = source_hits.chain(claude_hits).collect();

    assert!(
        all.is_empty(),
        "local covers! definitions record no coverage; use `use spec_trace::covers;` \
         (specs/development/covers-macro.md):\n{}",
        all.join("\n")
    );
}

#[test]
fn detector_finds_a_planted_local_definition() {
    covers!([SpecItem::McpToolsDevelopmentCoversMustRecord]);
    let local = format!(
        "mod tests {{\n    {LOCAL_DEFINITION} {{\n        ([$($item:expr),*]) => {{ $( let _ = $item; )* }};\n    }}\n}}\n"
    );
    let recording = "mod tests {\n    use spec_trace::covers;\n}\n";
    let markdown = format!("text\n```rust\n{local}```\n{LOCAL_DEFINITION} outside a block\n");

    assert_eq!(local_covers_definitions(numbered(&local)), vec![2]);
    assert!(local_covers_definitions(numbered(recording)).is_empty());
    assert_eq!(
        local_covers_definitions(code_block_lines(&markdown)),
        vec![4]
    );
}
