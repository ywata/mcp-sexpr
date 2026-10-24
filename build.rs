use spec_trace::builder::{Builder, BuilderError};
use std::fs;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};

fn read_spec_files() -> Vec<String> {
    let content = fs::read_to_string("spec-files.txt").expect("Failed to read spec-files.txt");
    content
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| line.to_string())
        .collect()
}

fn main() {
    println!("cargo:rerun-if-changed=spec-files.txt");
    println!("cargo:rerun-if-changed=specs/");

    let docs = read_spec_files();

    for doc in &docs {
        if doc.starts_with("docs/") {
            println!("cargo:rerun-if-changed={}", doc);
        }
    }

    let output = output_path();
    let builder = docs.iter().fold(
        Builder::new().require_prefix("mcp-tools"),
        |builder, doc| builder.add_doc(doc),
    );
    if let Err(err) = builder.output(&output).generate() {
        panic!("{}", generate_failure_message(&err, &output));
    }
    if let Err(err) = strip_inner_attributes(&output) {
        panic!("{}", io_failure_message(&err, &output));
    }
}

/// Generated file name inside `OUT_DIR`; `src/lib.rs` `include!`s it from there.
const OUTPUT_FILE: &str = "traceability_gen.rs";

/// The generated file lives under `OUT_DIR` only: the crate source tree may be
/// cargo's read-only git checkout (specs/packaging/build-script.md).
fn output_path() -> PathBuf {
    PathBuf::from(std::env::var_os("OUT_DIR").expect("cargo sets OUT_DIR for build scripts"))
        .join(OUTPUT_FILE)
}

/// Drop the generator's leading inner attributes (`#![allow(missing_docs)]`):
/// `include!` inside `mod traceability_gen { … }` rejects them, so `src/lib.rs`
/// carries the lint as an outer attribute instead. A no-op when there are none.
fn strip_inner_attributes(path: &Path) -> io::Result<()> {
    let code = fs::read_to_string(path)?;
    let stripped: String = code
        .split_inclusive('\n')
        .filter(|line| !line.trim_start().starts_with("#!["))
        .collect();
    if stripped == code {
        Ok(())
    } else {
        fs::write(path, stripped)
    }
}

/// Explain a generator failure. A denied I/O is reported as an environment
/// problem naming the absolute output path, because it otherwise surfaces
/// attached to whichever dependency change forced this build script to re-run.
fn generate_failure_message(err: &BuilderError, output: &Path) -> String {
    match err {
        BuilderError::Io(io) => io_failure_message(io, output),
        other => format!(
            "Failed to generate traceability code into {}: {other}",
            output.display()
        ),
    }
}

fn io_failure_message(io: &io::Error, output: &Path) -> String {
    match io.kind() {
        ErrorKind::PermissionDenied => format!(
            "mcp-tools build script could not read a spec file listed in spec-files.txt \
             or write the generated traceability file: {io}\n\
             output path: {}\n\
             This is an environment problem, not a code problem: the build script writes \
             only under OUT_DIR (cargo's target directory) and reads only spec-files.txt \
             and the specs it lists. Re-run with those spec files readable and the target \
             directory writable, or unsandboxed.",
            output.display()
        ),
        _ => format!(
            "Failed to generate traceability code into {}: {io}",
            output.display()
        ),
    }
}
