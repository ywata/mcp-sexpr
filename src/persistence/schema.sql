CREATE TABLE IF NOT EXISTS tool_call_events (
  id INTEGER PRIMARY KEY,
  created_at TEXT NOT NULL,
  transport TEXT NOT NULL,
  client_name TEXT,
  tool_name TEXT NOT NULL,
  canonical_tool_name TEXT NOT NULL,
  request_sexpr TEXT NOT NULL,
  response_sexpr TEXT NOT NULL,
  is_error INTEGER NOT NULL,
  internal_id TEXT
);

CREATE INDEX IF NOT EXISTS tool_call_events_internal_id_created_at
  ON tool_call_events (internal_id, created_at);

CREATE INDEX IF NOT EXISTS tool_call_events_created_at
  ON tool_call_events (created_at);

CREATE TABLE IF NOT EXISTS progress_snapshots (
  internal_id TEXT PRIMARY KEY,
  updated_at TEXT NOT NULL,
  event TEXT NOT NULL,
  snapshot_text TEXT NOT NULL
);
