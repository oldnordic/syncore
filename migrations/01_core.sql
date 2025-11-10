-- 01_core.sql
-- SynCore v1 Database Schema Migration

PRAGMA journal_mode=WAL;
PRAGMA foreign_keys=ON;

-- Tasks & planning ----------------------------------------------------------
CREATE TABLE IF NOT EXISTS tasks (
  id           INTEGER PRIMARY KEY AUTOINCREMENT,
  goal         TEXT NOT NULL,
  description  TEXT DEFAULT '',
  status       TEXT NOT NULL DEFAULT 'open',  -- open|running|blocked|done|cancelled
  priority     INTEGER NOT NULL DEFAULT 3,    -- 1=highest..5=lowest
  parent_id    INTEGER REFERENCES tasks(id) ON DELETE CASCADE,
  created_at   INTEGER NOT NULL,              -- epoch seconds
  updated_at   INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_tasks_status_prio ON tasks(status, priority, id);
CREATE INDEX IF NOT EXISTS idx_tasks_parent ON tasks(parent_id);

CREATE TABLE IF NOT EXISTS task_links (
  src_id    INTEGER NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
  dst_id    INTEGER NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
  kind      TEXT NOT NULL,   -- depends_on|relates_to
  PRIMARY KEY (src_id, dst_id, kind)
);

-- Cognitive loop ------------------------------------------------------------
CREATE TABLE IF NOT EXISTS steps (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  task_id       INTEGER REFERENCES tasks(id) ON DELETE SET NULL,
  state         TEXT NOT NULL,          -- Think|Decide|Act|Observe|Reflect
  content       TEXT NOT NULL,
  meta_json     TEXT NOT NULL DEFAULT '{}',
  created_at    INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_steps_task_state_time ON steps(task_id, state, created_at DESC);

-- Memory key/value (fast lookups) ------------------------------------------
CREATE TABLE IF NOT EXISTS memory (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  k          TEXT NOT NULL,
  v          TEXT NOT NULL,
  ts         INTEGER NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_memory_k ON memory(k);

-- Vector memory registry (text ID ↔ embedding row) -------------------------
CREATE TABLE IF NOT EXISTS embeddings (
  id         INTEGER PRIMARY KEY,       -- equals steps.id or synthetic
  task_id    INTEGER,
  kind       TEXT NOT NULL,             -- step|note|doc
  dim        INTEGER NOT NULL,
  created_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_embeddings_task ON embeddings(task_id);

-- Tool call audit (MCP) -----------------------------------------------------
CREATE TABLE IF NOT EXISTS tool_calls (
  id           INTEGER PRIMARY KEY AUTOINCREMENT,
  request_id   TEXT,                    -- optional idempotency
  tool_name    TEXT NOT NULL,
  args_json    TEXT NOT NULL,
  result_json  TEXT,
  status       TEXT NOT NULL,           -- ok|error
  created_at   INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_tool_calls_tool_time ON tool_calls(tool_name, created_at);