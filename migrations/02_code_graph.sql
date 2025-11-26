-- 02_code_graph.sql
-- Code Graph Mapping Schema Migration
-- Enables semantic + structural code search

PRAGMA foreign_keys=ON;

-- Code entities extracted from source files --------------------------------
CREATE TABLE IF NOT EXISTS code_entities (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_path TEXT NOT NULL,
    entity_type TEXT NOT NULL,  -- function|class|method|import|struct|enum|trait
    name TEXT NOT NULL,
    signature TEXT,              -- Full signature/declaration
    line_start INTEGER NOT NULL,
    line_end INTEGER NOT NULL,
    docstring TEXT,              -- Documentation/comments
    language TEXT NOT NULL,      -- rust|javascript|python|json|toml|bash
    indexed_at INTEGER NOT NULL, -- Epoch seconds
    body_snippet TEXT,           -- APEX v1.7: First N lines of function body for semantic search
    UNIQUE(file_path, entity_type, name, line_start)
);

CREATE INDEX IF NOT EXISTS idx_entities_name ON code_entities(name);
CREATE INDEX IF NOT EXISTS idx_entities_file ON code_entities(file_path);
CREATE INDEX IF NOT EXISTS idx_entities_type ON code_entities(entity_type);
CREATE INDEX IF NOT EXISTS idx_entities_lang ON code_entities(language);

-- Relationships between code entities --------------------------------------
CREATE TABLE IF NOT EXISTS code_edges (
    src_entity_id INTEGER NOT NULL REFERENCES code_entities(id) ON DELETE CASCADE,
    dst_entity_id INTEGER NOT NULL REFERENCES code_entities(id) ON DELETE CASCADE,
    edge_type TEXT NOT NULL,  -- calls|imports|inherits|references|uses|contains
    PRIMARY KEY (src_entity_id, dst_entity_id, edge_type)
);

CREATE INDEX IF NOT EXISTS idx_edges_src ON code_edges(src_entity_id);
CREATE INDEX IF NOT EXISTS idx_edges_dst ON code_edges(dst_entity_id);
CREATE INDEX IF NOT EXISTS idx_edges_type ON code_edges(edge_type);

-- Embeddings for code entities (linked to vector store) --------------------
CREATE TABLE IF NOT EXISTS code_embeddings (
    entity_id INTEGER PRIMARY KEY REFERENCES code_entities(id) ON DELETE CASCADE,
    vector_id INTEGER NOT NULL,  -- References embeddings table
    model_version TEXT NOT NULL, -- all-MiniLM-L6-v2
    created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_code_embeddings_vector ON code_embeddings(vector_id);
