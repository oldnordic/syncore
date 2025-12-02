-- 06_code_graph_nullable_lines.sql
-- Allow NULL values for line_start and line_end to match Neo4j behavior
-- File entities should have NULL line numbers, not 0

PRAGMA foreign_keys=ON;

-- Recreate the table with nullable line columns
CREATE TABLE IF NOT EXISTS code_entities_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_path TEXT NOT NULL,
    entity_type TEXT NOT NULL,  -- function|class|method|import|struct|enum|trait|file
    name TEXT NOT NULL,
    signature TEXT,              -- Full signature/declaration
    line_start INTEGER,         -- Allow NULL for File entities
    line_end INTEGER,           -- Allow NULL for File entities
    docstring TEXT,              -- Documentation/comments
    language TEXT NOT NULL,      -- rust|javascript|python|json|toml|bash
    indexed_at INTEGER NOT NULL, -- Epoch seconds
    body_snippet TEXT,           -- APEX v1.7: First N lines of function body for semantic search
    UNIQUE(file_path, entity_type, name, line_start)
);

-- Copy data from old table
INSERT INTO code_entities_new 
SELECT id, file_path, entity_type, name, signature, 
       CASE WHEN line_start = 0 THEN NULL ELSE line_start END as line_start,
       CASE WHEN line_end = 0 THEN NULL ELSE line_end END as line_end,
       docstring, language, indexed_at, body_snippet
FROM code_entities;

-- Drop old table and rename new one
DROP TABLE code_entities;
ALTER TABLE code_entities_new RENAME TO code_entities;

-- Recreate indexes
CREATE INDEX IF NOT EXISTS idx_entities_name ON code_entities(name);
CREATE INDEX IF NOT EXISTS idx_entities_file ON code_entities(file_path);
CREATE INDEX IF NOT EXISTS idx_entities_type ON code_entities(entity_type);
CREATE INDEX IF NOT EXISTS idx_entities_lang ON code_entities(language);