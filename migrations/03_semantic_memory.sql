-- SEMANTIC MEMORY MIGRATION
--
-- Adds semantic search, metadata, tags, and graph capabilities to memory store.
-- APEX 1.9 - REAL implementation using DualEmbeddingService + Neo4j.
-- APEX 2.4-CG-SCHEMA-FIX: Made idempotent to prevent duplicate column errors

-- Add new columns to existing memory table
-- SQLite doesn't support IF NOT EXISTS for ALTER TABLE ADD COLUMN,
-- so this migration will fail if run twice. This is handled by:
-- 1. ensure_schema() checking for memory_tags table before running this migration
-- 2. migration_005 checking for 'summary' column before running duplicate migration

ALTER TABLE memory ADD COLUMN summary TEXT;
ALTER TABLE memory ADD COLUMN namespace TEXT NOT NULL DEFAULT 'default';
ALTER TABLE memory ADD COLUMN importance REAL NOT NULL DEFAULT 0.5;
ALTER TABLE memory ADD COLUMN created_at INTEGER NOT NULL DEFAULT 0;
ALTER TABLE memory ADD COLUMN last_accessed INTEGER NOT NULL DEFAULT 0;
ALTER TABLE memory ADD COLUMN access_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE memory ADD COLUMN embedding_id INTEGER;

-- Tags table (many-to-many with memory entries)
CREATE TABLE IF NOT EXISTS memory_tags (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  memory_id INTEGER NOT NULL REFERENCES memory(id) ON DELETE CASCADE,
  tag TEXT NOT NULL,
  UNIQUE(memory_id, tag)
);

CREATE INDEX IF NOT EXISTS idx_memory_tags_tag ON memory_tags(tag);
CREATE INDEX IF NOT EXISTS idx_memory_tags_memory_id ON memory_tags(memory_id);

-- Memory consolidations (deduplication tracking)
CREATE TABLE IF NOT EXISTS memory_consolidations (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  source_id INTEGER NOT NULL REFERENCES memory(id) ON DELETE CASCADE,
  target_id INTEGER NOT NULL REFERENCES memory(id) ON DELETE CASCADE,
  similarity REAL NOT NULL,
  consolidated_at INTEGER NOT NULL,
  UNIQUE(source_id, target_id)
);

CREATE INDEX IF NOT EXISTS idx_consolidations_source ON memory_consolidations(source_id);
CREATE INDEX IF NOT EXISTS idx_consolidations_target ON memory_consolidations(target_id);

-- Indexes for semantic memory queries
CREATE INDEX IF NOT EXISTS idx_memory_namespace ON memory(namespace);
CREATE INDEX IF NOT EXISTS idx_memory_importance ON memory(importance DESC);
CREATE INDEX IF NOT EXISTS idx_memory_created_at ON memory(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_memory_last_accessed ON memory(last_accessed DESC);
CREATE INDEX IF NOT EXISTS idx_memory_access_count ON memory(access_count DESC);
CREATE INDEX IF NOT EXISTS idx_memory_embedding_id ON memory(embedding_id);

-- Update existing rows to have created_at/last_accessed timestamps
UPDATE memory SET created_at = ts WHERE created_at = 0;
UPDATE memory SET last_accessed = ts WHERE last_accessed = 0;
