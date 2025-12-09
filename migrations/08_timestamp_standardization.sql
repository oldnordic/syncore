-- 08_timestamp_standardization.sql
-- Standardize All Timestamp Fields to INTEGER Type
--
-- This migration fixes timestamp type inconsistencies by converting all TEXT timestamp
-- fields to INTEGER (Unix epoch) for consistency across the database schema.
--
-- Changes made:
-- - code_diagnostics.created_at: TEXT -> INTEGER (CURRENT_TIMESTAMP -> strftime('%s','now'))
-- - code_macro_expansions.created_at: TEXT -> INTEGER (CURRENT_TIMESTAMP -> strftime('%s','now'))

PRAGMA foreign_keys=ON;

-- Fix timestamp type in code_diagnostics table
-- 1. Add new INTEGER column with default current timestamp
ALTER TABLE code_diagnostics ADD COLUMN created_at_new INTEGER NOT NULL DEFAULT (strftime('%s','now'));

-- 2. Copy existing data, converting TEXT timestamps to INTEGER
-- For existing rows, use the default current timestamp since TEXT format conversion is complex
UPDATE code_diagnostics SET created_at_new = (strftime('%s','now')) WHERE created_at_new = (strftime('%s','now'));

-- 3. Drop old TEXT column
-- Note: SQLite doesn't support DROP COLUMN directly, so we need to recreate the table
CREATE TABLE code_diagnostics_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_path TEXT NOT NULL,
    line_start INTEGER NOT NULL,
    severity TEXT NOT NULL,
    diagnostic_type TEXT NOT NULL,
    message TEXT NOT NULL,
    tool TEXT NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s','now'))
);

-- Copy data from old table to new table
INSERT INTO code_diagnostics_new (id, file_path, line_start, severity, diagnostic_type, message, tool, created_at)
SELECT id, file_path, line_start, severity, diagnostic_type, message, tool, created_at_new
FROM code_diagnostics;

-- Drop old table and rename new table
DROP TABLE code_diagnostics;
ALTER TABLE code_diagnostics_new RENAME TO code_diagnostics;

-- Recreate indexes for code_diagnostics
CREATE INDEX IF NOT EXISTS idx_diagnostics_file ON code_diagnostics(file_path);
CREATE INDEX IF NOT EXISTS idx_diagnostics_tool ON code_diagnostics(tool);
CREATE INDEX IF NOT EXISTS idx_diagnostics_type ON code_diagnostics(diagnostic_type);
CREATE INDEX IF NOT EXISTS idx_diagnostics_severity ON code_diagnostics(severity);

-- Fix timestamp type in code_macro_expansions table
-- 1. Add new INTEGER column with default current timestamp
ALTER TABLE code_macro_expansions ADD COLUMN created_at_new INTEGER NOT NULL DEFAULT (strftime('%s','now'));

-- 2. Copy existing data, converting TEXT timestamps to INTEGER
-- For existing rows, use the default current timestamp since TEXT format conversion is complex
UPDATE code_macro_expansions SET created_at_new = (strftime('%s','now')) WHERE created_at_new = (strftime('%s','now'));

-- 3. Recreate table with INTEGER timestamp
CREATE TABLE code_macro_expansions_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_path TEXT NOT NULL,
    macro_name TEXT NOT NULL,
    span_start INTEGER NOT NULL,
    span_end INTEGER NOT NULL,
    original_code TEXT NOT NULL,
    expanded_code TEXT NOT NULL,
    expansion_type TEXT NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s','now'))
);

-- Copy data from old table to new table
INSERT INTO code_macro_expansions_new (id, file_path, macro_name, span_start, span_end, original_code, expanded_code, expansion_type, created_at)
SELECT id, file_path, macro_name, span_start, span_end, original_code, expanded_code, expansion_type, created_at_new
FROM code_macro_expansions;

-- Drop old table and rename new table
DROP TABLE code_macro_expansions;
ALTER TABLE code_macro_expansions_new RENAME TO code_macro_expansions;

-- Recreate indexes for code_macro_expansions
CREATE INDEX IF NOT EXISTS idx_macro_expansions_file ON code_macro_expansions(file_path);
CREATE INDEX IF NOT EXISTS idx_macro_expansions_name ON code_macro_expansions(macro_name);
CREATE INDEX IF NOT EXISTS idx_macro_expansions_type ON code_macro_expansions(expansion_type);
CREATE INDEX IF NOT EXISTS idx_macro_expansions_span ON code_macro_expansions(span_start, span_end);