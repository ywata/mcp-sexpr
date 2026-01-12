#![allow(missing_docs)]

use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct ToolCallEvent {
    pub transport: String,
    pub client_name: Option<String>,
    pub tool_name: String,
    pub canonical_tool_name: String,
    pub request_sexpr: String,
    pub response_sexpr: String,
    pub is_error: bool,
    pub internal_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ProgressSnapshot {
    pub internal_id: String,
    pub event: String,
    pub snapshot_text: String,
}

#[derive(Clone)]
pub struct SqlitePersistence {
    conn: Arc<Mutex<Connection>>,
}

impl SqlitePersistence {
    pub fn open(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path)
            .with_context(|| format!("Failed to open sqlite db: {}", db_path.display()))?;

        let schema_sql = include_str!("schema.sql");
        conn.execute_batch(schema_sql)
            .context("Failed to initialize sqlite schema")?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn insert_tool_call_event(&self, event: &ToolCallEvent) -> Result<()> {
        let created_at = unix_epoch_seconds_string()?;
        let is_error = if event.is_error { 1 } else { 0 };

        let conn = self.conn.lock().expect("sqlite connection mutex poisoned");
        conn.execute(
            "INSERT INTO tool_call_events (created_at, transport, client_name, tool_name, canonical_tool_name, request_sexpr, response_sexpr, is_error, internal_id)\
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                created_at,
                event.transport,
                event.client_name,
                event.tool_name,
                event.canonical_tool_name,
                event.request_sexpr,
                event.response_sexpr,
                is_error,
                event.internal_id,
            ],
        )
        .context("Failed to insert tool call event")?;

        Ok(())
    }

    pub fn upsert_progress_snapshot(&self, snapshot: &ProgressSnapshot) -> Result<()> {
        let updated_at = unix_epoch_seconds_string()?;

        let conn = self.conn.lock().expect("sqlite connection mutex poisoned");
        conn.execute(
            "INSERT INTO progress_snapshots (internal_id, updated_at, event, snapshot_text)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(internal_id) DO UPDATE SET
               updated_at = excluded.updated_at,
               event = excluded.event,
               snapshot_text = excluded.snapshot_text",
            params![
                snapshot.internal_id,
                updated_at,
                snapshot.event,
                snapshot.snapshot_text,
            ],
        )
        .context("Failed to upsert progress snapshot")?;

        Ok(())
    }
}

fn unix_epoch_seconds_string() -> Result<String> {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("System time is before UNIX_EPOCH")?
        .as_secs();
    Ok(secs.to_string())
}
