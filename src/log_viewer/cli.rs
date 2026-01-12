#![allow(missing_docs)]

use crate::interactive::{
    default_history_path, run_line_loop, HistoryKind, LineLoopConfig, LoopControl,
};
use crate::log_viewer::command::Command;
use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::Path;

pub fn run(db_path: &Path) -> Result<()> {
    let conn = Connection::open(db_path)
        .with_context(|| format!("Failed to open sqlite db: {}", db_path.display()))?;

    let cfg = LineLoopConfig::new(
        || "log-viewer> ".to_string(),
        true,
        || LoopControl::Continue,
        || LoopControl::Break,
    )
    .with_history_file(default_history_path(HistoryKind::LogViewer));

    run_line_loop(cfg, |line| {
        let cmd = Command::parse(line);
        match cmd {
            Command::Empty => {}
            Command::Help => {
                println!("{}", Command::help_text());
            }
            Command::ShowAll => {
                show_all(&conn)?;
            }
            Command::Unknown(s) => {
                println!("Unknown command: {}", s);
                println!("{}", Command::help_text());
            }
        }

        Ok(LoopControl::Continue)
    })
}

fn show_all(conn: &Connection) -> Result<()> {
    let out = render_show_all(conn)?;
    print!("{}", out);
    Ok(())
}

pub fn render_show_all(conn: &Connection) -> Result<String> {
    let mut stmt = conn
        .prepare(
            "SELECT internal_id, updated_at, event, snapshot_text \
             FROM progress_snapshots \
             ORDER BY updated_at DESC",
        )
        .context("Failed to prepare progress snapshot query")?;

    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .context("Failed to query progress snapshots")?;

    let mut out = String::new();

    for row in rows {
        let (internal_id, updated_at, event, snapshot_text) =
            row.context("Failed to read progress snapshot row")?;
        out.push_str(&format!("== {} {} {} ==\n", internal_id, updated_at, event));
        out.push_str(&snapshot_text);
        if !snapshot_text.ends_with('\n') {
            out.push('\n');
        }
    }

    Ok(out)
}
