#![allow(missing_docs)]

use anyhow::Result;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Help,
    ShowAll,
    Unknown(String),
    Empty,
}

impl Command {
    pub fn parse(input: &str) -> Self {
        input
            .parse()
            .unwrap_or_else(|_| Command::Unknown(input.to_string()))
    }

    pub fn help_text() -> &'static str {
        "Commands:\n\
help\n\
show all\n"
    }
}

impl FromStr for Command {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Ok(Command::Empty);
        }

        let normalized = trimmed
            .split_whitespace()
            .map(|w| w.to_ascii_lowercase())
            .collect::<Vec<_>>();

        match normalized.as_slice() {
            [cmd] if cmd == "help" => Ok(Command::Help),
            [a, b] if a == "show" && b == "all" => Ok(Command::ShowAll),
            _ => Ok(Command::Unknown(trimmed.to_string())),
        }
    }
}
