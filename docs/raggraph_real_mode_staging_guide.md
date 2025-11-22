# RAGGraph Real Mode Staging Guide

## Overview

This guide covers deploying SynCore with RAGGraph in **REAL mode**, which uses actual Neo4j graph database and HNSW vector search instead of mock data.

## Prerequisites

### Required Infrastructure

1. **Neo4j Database** (v4.0+)
   - Bolt protocol enabled (default port 7687)
   - Authentication configured (username + password)
   - Sufficient memory for graph storage (recommend 4GB+ for production)

2. **Vector Embeddings**
   - Populated HNSW index with embeddings
   - Correct dimensionality (default: 384-dim)
   - Non-empty index (validation will fail otherwise)

### Required Environment Variables

```bash
# Neo4j Connection
export NEO4J_URI="bolt://127.0.0.1:7687"
export NEO4J_USER="neo4j"
export NEO4J_PASS="your_secure_password"

# RAGGraph Backend Mode
export SYNCORE_RAGGRAPH_BACKEND="real"  # or "mock" for testing
```

## Deployment Steps

### 1. Build Release Binary

```bash
cd syncore
cargo build --release
```

Binary location: `target/release/syncore_mcp_stdio`

### 2. Verify Neo4j Connectivity

Test Neo4j connection before starting SynCore:

```bash
# Using cypher-shell (if installed)
cypher-shell -a "$NEO4J_URI" -u "$NEO4J_USER" -p "$NEO4J_PASS" "RETURN 1 as health_check"

# Expected output: health_check: 1
```

### 3. Populate Vector Index

Ensure HNSW vector index has embeddings before starting:

```bash
# Check index stats (example - adjust to your setup)
# Index should report non-zero count and correct dimension
```

**CRITICAL**: Real mode validation will fail if vector index is empty or has dimension mismatch.

### 4. Start SynCore MCP Server

```bash
# Set environment variables
export NEO4J_URI="bolt://127.0.0.1:7687"
export NEO4J_USER="neo4j"
export NEO4J_PASS="your_password"
export SYNCORE_RAGGRAPH_BACKEND="real"

# Start server
./target/release/syncore_mcp_stdio
```

### 5. Monitor Logs

RAGGraph operations log to stderr with `[RAGGraph]` prefix:

```
[RAGGraph] query: backend=Real, query_len=42, num_hops=3, top_k=50
[RAGGraph] query completed: top_nodes=10, reasoning_steps=7
```

Filter RAGGraph logs:

```bash
./target/release/syncore_mcp_stdio 2>&1 | grep "\[RAGGraph\]"
```

## Validation

RAGGraph includes runtime validation that checks:

1. **Neo4j Connectivity**: Executes health check query
2. **Vector Index Population**: Verifies non-empty index
3. **Dimension Matching**: Confirms embedding dimensions match config

**Validation failures return clear error messages** (no silent fallback to mocks):

```json
{
  "error": "RAGGraph real mode validation failed: Vector index is empty. Real mode requires a populated HNSW index with embeddings."
}
```

## Configuration

### Backend Mode

Control RAGGraph mode via environment variable:

```bash
# Real mode (Neo4j + HNSW)
export SYNCORE_RAGGRAPH_BACKEND="real"

# Mock mode (in-memory test data)
export SYNCORE_RAGGRAPH_BACKEND="mock"  # or unset (default)
```

### RAGGraph Parameters

Configure via environment (defaults shown):

```bash
export SYNCORE_RAGGRAPH_NUM_HOPS="3"      # Number of graph hops
export SYNCORE_RAGGRAPH_ALPHA="0.85"      # Diffusion damping factor
export SYNCORE_RAGGRAPH_TOP_K="50"        # Top K results to return
export SYNCORE_RAGGRAPH_EMBEDDING_DIM="384"  # Embedding dimensions
```

## Troubleshooting

### Error: "Neo4j connection unavailable"

**Cause**: Cannot connect to Neo4j database.

**Solution**:
1. Verify Neo4j is running: `systemctl status neo4j` (Linux) or check process
2. Check `NEO4J_URI` format: must be `bolt://host:port`
3. Verify credentials: `NEO4J_USER` and `NEO4J_PASS` are correct
4. Check firewall/network: ensure port 7687 is accessible

### Error: "Vector index is empty"

**Cause**: HNSW index has no embeddings.

**Solution**:
1. Index embeddings before starting SynCore
2. Verify indexing completed successfully
3. Check index statistics to confirm non-zero count

### Error: "Vector dimension mismatch: expected 384, got 128"

**Cause**: Embedding dimensions don't match configuration.

**Solution**:
1. Re-index with correct embedding model (384-dim)
2. Or update `SYNCORE_RAGGRAPH_EMBEDDING_DIM` to match existing embeddings
3. Ensure consistency across vector index and config

### Silent Fallback to Mock Mode

**This should NOT happen in real mode**. If you see mock data when `SYNCORE_RAGGRAPH_BACKEND=real`:

1. Check environment variable is set correctly
2. Verify no typos: must be exactly `"real"` (lowercase)
3. Check Neo4j client initialization in logs

## Testing Real Mode

### Integration Test Suite

Run real-mode tests against Neo4j:

```bash
# Set Neo4j credentials
export NEO4J_URI="bolt://127.0.0.1:7687"
export NEO4J_USER="neo4j"
export NEO4J_PASS="testpassword123"

# Run real-mode tests
cargo test --test raggraph_real_mode_tests
```

Expected: All 4 real-mode tests pass.

### Manual MCP Tool Test

Test via MCP client (e.g., Claude Desktop):

```json
{
  "tool": "raggraph_query",
  "params": {
    "query_text": "test query"
  }
}
```

Check response includes:
- `top_nodes`: List of node IDs (not mock data)
- `context_embedding_dim`: 384 (or your configured dimension)
- `reasoning_path`: Non-empty array of reasoning steps

## Production Checklist

- [ ] Neo4j running and accessible
- [ ] Neo4j credentials set in environment
- [ ] Vector index populated with embeddings
- [ ] Embedding dimensions match config (default 384)
- [ ] `SYNCORE_RAGGRAPH_BACKEND="real"` set
- [ ] Release binary built (`cargo build --release`)
- [ ] Integration tests pass (`cargo test --test raggraph_real_mode_tests`)
- [ ] Manual MCP tool test returns real data (not mock)
- [ ] Logs show `[RAGGraph]` entries with backend=Real
- [ ] Validation errors surfaced clearly (no silent failures)

## Architecture Notes

### Real Mode Components

1. **RealStorageAdapter** (`src/raggraph/storage.rs`)
   - Interfaces with Neo4j for graph traversal
   - Uses HNSW for vector similarity search
   - Resolves embeddings from graph nodes

2. **HopGraphTransformer** (`src/raggraph/hopgraph.rs`)
   - Multi-hop reasoning over real graph topology
   - PageRank-style diffusion algorithm
   - Combines vector similarity + graph structure

3. **Validation** (`src/raggraph/validation.rs`)
   - Runtime checks for Neo4j + vector index
   - Fails fast with clear error messages
   - No silent fallback to mocks

### MCP Tools

- `raggraph_query`: Text query → semantic search → graph reasoning → results
- `raggraph_multihop`: Direct seed nodes → graph diffusion → results

Both tools:
- Validate backend before execution
- Log params and results to stderr
- Return structured JSON errors on failure

## Support

For issues or questions:
- Check logs for `[RAGGraph]` entries
- Verify all environment variables set correctly
- Run integration tests to isolate problem
- Review validation error messages (they're designed to be actionable)
