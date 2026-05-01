use spec_trace::builder::Builder;
use std::fs;

fn read_spec_files() -> Vec<String> {
    let content = fs::read_to_string("spec-files.txt")
        .expect("Failed to read spec-files.txt");
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

    let mut builder = Builder::new()
        .require_prefix("mcp-tools");
    for doc in &docs {
        builder = builder.add_doc(doc);
    }
    builder
        .output("src/traceability_gen.rs")
        .generate()
        .expect("Failed to generate traceability code");
}
