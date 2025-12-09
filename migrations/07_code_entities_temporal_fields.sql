-- 07_code_entities_temporal_fields.sql
-- Fix Code Entities Schema Drift - Migration-Driven Temporal Fields
--
-- This migration eliminates runtime schema drift by moving temporal field creation
-- from dynamic ALTER TABLE statements in code_graph/graph.rs to a proper migration.
--
-- Temporal fields added:
-- - created_at: Unix timestamp when entity was first created (default: 0)
-- - last_modified_at: Unix timestamp of last modification (default: 0)
-- - change_count: Number of times entity has been modified (default: 0)
-- - author_count: Number of unique authors (default: 0)

PRAGMA foreign_keys=ON;

-- Add temporal fields to code_entities table
ALTER TABLE code_entities ADD COLUMN created_at INTEGER NOT NULL DEFAULT 0;
ALTER TABLE code_entities ADD COLUMN last_modified_at INTEGER NOT NULL DEFAULT 0;
ALTER TABLE code_entities ADD COLUMN change_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE code_entities ADD COLUMN author_count INTEGER NOT NULL DEFAULT 0;

-- Create indexes for temporal fields to support temporal queries
CREATE INDEX IF NOT EXISTS idx_code_entities_created_at ON code_entities(created_at);
CREATE INDEX IF NOT EXISTS idx_code_entities_last_modified_at ON code_entities(last_modified_at);