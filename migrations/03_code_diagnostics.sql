-- 03_code_diagnostics.sql
-- Code Diagnostics Storage Schema Migration
-- Enables storage of static analysis diagnostics from tools like Clippy

PRAGMA foreign_keys=ON;

-- Static analysis diagnostics from external tools
CREATE TABLE IF NOT EXISTS code_diagnostics (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_path TEXT NOT NULL,
    line_start INTEGER NOT NULL,
    severity TEXT NOT NULL,           -- e.g. "warning", "error", "note"
    diagnostic_type TEXT NOT NULL,    -- e.g. "clippy::dead_code"
    message TEXT NOT NULL,            -- human-readable summary
    tool TEXT NOT NULL,               -- e.g. "clippy"
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_diagnostics_file ON code_diagnostics(file_path);
CREATE INDEX IF NOT EXISTS idx_diagnostics_tool ON code_diagnostics(tool);
CREATE INDEX IF NOT EXISTS idx_diagnostics_type ON code_diagnostics(diagnostic_type);
CREATE INDEX IF NOT EXISTS idx_diagnostics_severity ON code_diagnostics(severity);