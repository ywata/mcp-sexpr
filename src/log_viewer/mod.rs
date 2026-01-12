#![allow(missing_docs)]

pub mod cli;
pub mod command;

pub use command::Command;

use anyhow::Result;
use std::path::{Path, PathBuf};

pub fn run(db_path: PathBuf) -> Result<()> {
    cli::run(Path::new(&db_path))
}
