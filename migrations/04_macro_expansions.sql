-- 04_macro_expansions.sql
-- Macro Expansions Storage Schema Migration
-- Enables storage of static macro expansion results for PAE analysis

PRAGMA foreign_keys=ON;

-- Static macro expansion results from SMEL (Static Macro Expansion Layer)
CREATE TABLE IF NOT EXISTS code_macro_expansions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_path TEXT NOT NULL,
    macro_name TEXT NOT NULL,         -- e.g. "vec!", "format!", "my_macro!"
    span_start INTEGER NOT NULL,     -- Start position in source code
    span_end INTEGER NOT NULL,       -- End position in source code
    original_code TEXT NOT NULL,      -- Original macro invocation
    expanded_code TEXT NOT NULL,      -- Expanded code after processing
    expansion_type TEXT NOT NULL,      -- e.g. "vec", "format", "log", "assert", "declarative"
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_macro_expansions_file ON code_macro_expansions(file_path);
CREATE INDEX IF NOT EXISTS idx_macro_expansions_name ON code_macro_expansions(macro_name);
CREATE INDEX IF NOT EXISTS idx_macro_expansions_type ON code_macro_expansions(expansion_type);
CREATE INDEX IF NOT EXISTS idx_macro_expansions_span ON code_macro_expansions(span_start, span_end);