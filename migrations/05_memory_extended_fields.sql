-- 05_memory_extended_fields.sql
-- APEX 2.0-M-FIX: Add extended memory fields for namespace isolation and semantic search
--
-- This migration adds the missing columns that the APEX 2.0-M-FIX implementation expects:
-- - namespace (for isolation)
-- - summary (for semantic search)
-- - importance (for prioritization)
-- - created_at (for lifecycle tracking)
-- - last_accessed (for access tracking)
-- - access_count (for usage metrics)

-- Add namespace column (default to 'default' for backward compatibility)
ALTER TABLE memory ADD COLUMN namespace TEXT NOT NULL DEFAULT 'default';

-- Add summary column (nullable for non-semantic entries)
ALTER TABLE memory ADD COLUMN summary TEXT;

-- Add importance column (default to 0.5 for neutral importance)
ALTER TABLE memory ADD COLUMN importance REAL NOT NULL DEFAULT 0.5;

-- Add created_at column (default to current timestamp for existing rows)
ALTER TABLE memory ADD COLUMN created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'));

-- Add last_accessed column (default to created_at)
ALTER TABLE memory ADD COLUMN last_accessed INTEGER NOT NULL DEFAULT (strftime('%s', 'now'));

-- Add access_count column (default to 0)
ALTER TABLE memory ADD COLUMN access_count INTEGER NOT NULL DEFAULT 0;

-- Add embedding_id column (nullable - links to embeddings table)
ALTER TABLE memory ADD COLUMN embedding_id INTEGER;
