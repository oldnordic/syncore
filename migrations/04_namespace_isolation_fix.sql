-- FIX: Namespace isolation
--
-- The original unique index on memory(k) prevents storing the same key
-- in different namespaces. This migration fixes that.

-- Drop the old unique index
DROP INDEX IF EXISTS idx_memory_k;

-- Create new composite unique index for (k, namespace)
CREATE UNIQUE INDEX IF NOT EXISTS idx_memory_k_namespace ON memory(k, namespace);
