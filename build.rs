use spec_trace::builder::{Builder, BuilderError};
use std::fs;
use std::io::ErrorKind;

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

    let mut builder = Builder::new().require_prefix("mcp-tools");
    for doc in &docs {
        builder = builder.add_doc(doc);
    }
    if let Err(err) = builder.output(OUTPUT).generate() {
        panic!("{}", generate_failure_message(&err));
    }
}

/// Generated file, relative to the crate root (`CARGO_MANIFEST_DIR`).
const OUTPUT: &str = "src/traceability_gen.rs";

/// Explain a generator failure. A denied I/O is reported as an environment
/// problem naming the absolute output path, because it otherwise surfaces
/// attached to whichever dependency change forced this build script to re-run.
fn generate_failure_message(err: &BuilderError) -> String {
    let output = std::env::var("CARGO_MANIFEST_DIR")
        .map(|dir| format!("{}/{}", dir, OUTPUT))
        .unwrap_or_else(|_| OUTPUT.to_string());
    match err {
        BuilderError::Io(io) if io.kind() == ErrorKind::PermissionDenied => format!(
            "mcp-tools build script could not write the generated traceability file \
             (or read a spec file listed in spec-files.txt): {io}\n\
             output path: {output}\n\
             This is an environment problem, not a code problem: the build script \
             regenerates {OUTPUT} inside the crate source tree, and the current \
             environment (e.g. an agent sandbox building mcp-tools as a dependency \
             from cargo's git checkout) denies that write. Re-run with that path \
             writable, or unsandboxed."
        ),
        other => format!("Failed to generate traceability code into {output}: {other}"),
    }
}
