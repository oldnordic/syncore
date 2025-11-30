# SynCore

**MCP server for AI-native development** - Built in 2 weeks, solo, with $15 AI assistance budget.

Unified interface for memory, vectors, code analysis, and knowledge graphs. **65+ tools** accessible via single MCP connection.

Built in Rust. Production-tested with Claude Code.

## What This Actually Does

SynCore is an MCP (Model Context Protocol) server exposing **65 tools** for:
- **Persistent Memory** - Key-value storage (SQLite + Sled cache)
- **Dual-Domain Vector Search** - Separate embedding models for code vs. text
  - CODE domain: BGE-small-en-v1.5 (optimized for semantic code search)
  - GENERAL domain: all-MiniLM-L6-v2 (optimized for documents/tasks)
- **Code Intelligence** - Tree-sitter parsing for Rust, JavaScript, Python, JSON, TOML, Bash
- **Knowledge Graphs** - Neo4j integration with Cypher queries
- **Task Management** - Task tracking with parent/child relationships
- **Agent Coordination** - Message bus for multi-agent workflows
- **Application Mapping** - File dependency tracking and change history
- **Sequential Reasoning** - Multi-step thought recording
- **Project Analysis** - LLM-free codebase intelligence (hotspots, dead code, cycles, refactoring)

## Current State (November 2025)

### What Works Reliably

**Core Tools (Tested Daily):**
- `memory_store` / `memory_query` - Key-value storage, ~1ms latency
- `vector_insert` / `vector_search` - Semantic search, 10-50ms latency
- `code_index` / `code_search` - Code semantic search
- `code_index_directory` - Batch index with **incremental support** (skips unchanged files)
- `parser_analyze` - Tree-sitter AST extraction with optional persistence
- `parser_search` - Ripgrep code pattern search
- `graph_query` / `graph_insert` - Neo4j Cypher queries
- `code_graph_sync_neo4j` - Sync SQLite entities to Neo4j
- `code_graph_fusion_query` - Tri-mode (simple/attention/reasoning) search

**Graph Features:**
- 7 edge types: CONTAINS, CALLS, USES, IMPORTS, MODULE_CHILD, INHERITS, REFERENCES
- Multi-hop diffusion with PageRank
- Temporal metadata enrichment (git history + mtime)

**Project Analysis Engine (PHASE 6):**
- `project_file_report` - Detailed file analysis (entities, relationships, metrics)
- `project_hotspots` - Find complexity hotspots (fan-in/out, LOC, entity count)
- `project_dead_code` - Detect unused entities
- `project_cycles` - Find circular dependencies
- `project_unused_imports` - Find unused imports
- `project_module_map` - Module dependency map
- `project_refactor_suggestions` - Heuristic refactoring suggestions

**Meta-Tools (Aggregate PAE Data):**
- `project_architecture_overview` - Project-wide summary: entity counts, dependency stats, complexity metrics
- `project_complexity_dashboard` - Top hotspots, worst cycles, complexity rankings
- `project_improvement_roadmap` - Prioritized improvements with effort/impact matrix
- `project_refactor_action_plan` - Actionable refactor plan (high-risk hotspots, dead code, cycle breaks)

**Incremental Indexing (NEW - PHASE 5):**
- SHA256 + mtime change detection
- Skips unchanged files (0 entities returned = skipped)
- Detects new, modified, deleted files
- Idempotent (safe to run repeatedly)

### What's Experimental

- **IntelliTask AI features** - Requires Ollama running locally, quality depends on model
- **Sequential reasoning** - `sequential_cycle` requires Ollama
- **Agent message bus** - Works but not heavily tested in production
- **RAGGraph multi-hop** - Works but scoring may need tuning

### Honest Limitations

- **Single-node only** - No distributed mode
- **No authentication** - Designed for local use
- **Neo4j required for graph features** - Some tools fail silently without it
- **Ollama required for AI features** - IntelliTask, sequential_cycle need it
- **~500MB RAM after startup** - Two HuggingFace embedding models loaded (BGE + MiniLM)
- **Graph features partially tested** - Neo4j sync works but entity population needs validation

## Quick Start

### Build

```bash
cd syncore
cargo build --release
```

Produces two binaries:
- `target/release/syncore_mcp_stdio` (~55 MB) - MCP server for Claude Code
- `target/release/syncore_graph_cli` (~33 MB) - CLI for graph operations

### Configure Claude Code

Add to `~/.config/claude/mcp_settings.json`:

```json
{
  "syncore": {
    "command": "/path/to/syncore/target/release/syncore_mcp_stdio",
    "args": [],
    "env": {
      "DB_PATH": "/path/to/syncore.db",
      "NEO4J_URI": "bolt://127.0.0.1:7687",
      "NEO4J_USER": "neo4j",
      "NEO4J_PASS": "your_password"
    }
  }
}
```

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `DB_PATH` | `syncore.db` | SQLite database path |
| `HTTP_PORT` | `3001` | HTTP streaming server port |
| `NEO4J_URI` | `bolt://127.0.0.1:7687` | Neo4j connection |
| `NEO4J_USER` | `neo4j` | Neo4j username |
| `NEO4J_PASS` | (required for graph) | Neo4j password |
| `RUST_LOG` | `info` | Log level |

## All Tools (42 in memory_suite + 23 standalone)

### Memory Suite (42 commands via `memory_suite`)

**Basic Memory (5 commands):**
| Command | Description | Cost |
|---------|-------------|------|
| `store` | Store key-value pair with metadata | Low |
| `query` | Retrieve value by key | Low |
| `delete` | Delete memory entry | Low |
| `list_keys` | List all memory keys | Low |
| `memory_stats` | Get count and namespace statistics | Low |

**Semantic Search (8 commands - APEX 1.9-M):**
| Command | Description | Cost |
|---------|-------------|------|
| `search_semantic` | Vector-based semantic search | Medium |
| `search_hybrid` | Combined semantic + keyword search | Medium |
| `query_by_tags` | Filter memories by tags | Low |
| `query_by_importance` | Filter by importance threshold | Low |
| `query_recent` | Get N most recent memories | Low |
| `query_since` | Get memories since timestamp | Low |
| `consolidate_similar` | Find and merge similar memories | High |
| `get_related_memories` | Get related memories (semantic + graph) | Medium |

**Vector Operations (2 commands):**
| Command | Description | Cost |
|---------|-------------|------|
| `vector_insert` | Add text with embeddings | Medium |
| `vector_search` | Semantic similarity search | Medium |

**Task Management (15 commands):**
| Command | Description | Cost |
|---------|-------------|------|
| `task_create` | Create a task | Low |
| `intellitask_list` | List all tasks | Low |
| `intellitask_get` | Get task by ID | Low |
| `intellitask_update_status` | Update task status | Low |
| `intellitask_next_ready` | Get next ready task | Low |
| `intellitask_get_subtasks` | Get subtasks | Low |
| `intellitask_subtask_stats` | Subtask statistics | Low |
| `intellitask_task_statistics` | Overall task statistics | Low |
| `intellitask_prd_statistics` | PRD-specific statistics | Low |
| `intellitask_save` | Save task breakdown to DB | Low |
| `intellitask_generate` | **Requires Ollama** - AI task breakdown from PRD | High |
| `intellitask_subtasks` | **Requires Ollama** - Generate subtasks | High |
| `intellitask_prioritize` | **Requires Ollama** - AI task prioritization | High |
| `intellitask_next` | **Requires Ollama** - AI next task suggestion | High |

**Sequential Reasoning (4 commands):**
| Command | Description | Cost |
|---------|-------------|------|
| `sequential_record` | Record thought step in reasoning chain | Low |
| `sequential_get` | Get thought steps for task | Low |
| `sequential_search` | Search thought steps semantically | Medium |
| `sequential_cycle` | **Requires Ollama** - Run reasoning cycle | Very High |

**Agent Coordination (8 commands):**
| Command | Description | Cost |
|---------|-------------|------|
| `agent_send` | Send message to agent via message bus | Low |
| `agent_recv` | Receive pending messages | Low |
| `agent_poll` | Wait for next message (blocking) | Low |
| `agent_register` | Register agent with capabilities | Low |
| `agent_list` | List registered agents | Low |
| `agent_status` | Update agent status | Low |
| `agent_task` | Send structured task envelope | Low |
| `agent_result` | Submit completed task result | Low |

### Standalone MCP Tools

**Code Suite (11 commands):**
| Command | Description | Cost |
|---------|-------------|------|
| `index` | Index a source file | High |
| `search` | Semantic code search | Medium |
| `index_directory` | Batch index directory (incremental) | Very High |
| `parse` | Tree-sitter AST extraction | Medium |
| `grep` | Ripgrep pattern search | Medium |
| `explain` | Explain function with metrics | Medium |
| `doc_index` | Index documents from directory | Very High |
| `doc_search` | Semantic document search | Medium |
| `fusion_query` | Tri-mode RAG query (simple/attention/reasoning) | High |
| `enrich_temporal` | Add git history + mtime metadata | Very High |
| `sync_neo4j` | Sync SQLite entities to Neo4j | Very High |

**Graph Suite (5 commands):**
| Command | Description | Cost |
|---------|-------------|------|
| `query` | Execute Cypher read query | High |
| `insert` | Execute Cypher write query | High |
| `relate` | Create relationship between nodes | High |
| `rag_query` | RAG query with multi-hop graph reasoning | High |
| `rag_multihop` | Multi-hop graph diffusion from seed nodes | High |

**Mapping Suite (8 commands):**
| Command | Description | Cost |
|---------|-------------|------|
| `record` | Record file node with imports/exports/deps | Low |
| `get` | Get file node | Low |
| `search` | Semantic file search | Medium |
| `deps` | Get transitive dependencies | Medium |
| `app_record` | Record code change | Low |
| `app_get` | Get changes for task | Low |
| `app_history` | Get file change history | Low |
| `app_search` | Search changes semantically | Medium |

**Debug Suite (10 commands):**
| Command | Description | Cost |
|---------|-------------|------|
| `logs_tail` | Get recent log entries | Low |
| `tool_metadata_list` | List tool metadata | Low |
| `project_file_report` | Detailed file analysis | Medium |
| `project_hotspots` | Find complexity hotspots | Medium |
| `project_dead_code` | Identify unused entities | Medium |
| `project_cycles` | Detect circular dependencies | Medium |
| `project_unused_imports` | Find unused imports | Medium |
| `project_module_map` | Module dependency map | Medium |
| `project_refactor_suggestions` | Refactoring suggestions | Medium |
| `project_code_smells` | Detect code smells | Medium |

**REFRAG Suite (3 commands - APEX 1.8):**
| Command | Description | Cost |
|---------|-------------|------|
| `query` | Selective expansion with token budget | High |
| `configure` | Update expansion policy | Low |
| `help` | Show REFRAG usage | Low |

## Architecture Notes

### Three-Tier Embedding Architecture

SynCore implements **three distinct embedding systems** with specialized purposes:

#### 1. HuggingFaceEmbeddings - Production Code Embeddings
**File**: `src/vector.rs:78`  
**Trait**: `impl Embeddings for HuggingFaceEmbeddings`  
**Model**: BGE-small-en-v1.5 (384 dimensions)  
**Domain**: CODE domain  
**Purpose**: Production-ready semantic code search

```rust
// src/vector.rs:78
impl Embeddings for HuggingFaceEmbeddings {
    fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        // Uses fastembed-rs with BGE-small-en-v1.5
    }
}
```

**Characteristics:**
- **Latency**: 10-50ms per query
- **Use Case**: Code entities, functions, classes
- **Storage**: `syncore_code.index` (HNSW)
- **Routing**: Automatic for code-related content

#### 2. RealEmbeddings - Development/Testing Embeddings  
**File**: `src/vector.rs:294`  
**Trait**: `impl Embeddings for RealEmbeddings`  
**Model**: Hash-based deterministic (384 dimensions)  
**Domain**: Both CODE and GENERAL domains  
**Purpose**: Development, testing, CI/CD pipelines

```rust
// src/vector.rs:294
impl Embeddings for RealEmbeddings {
    fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        // Deterministic hash-based embeddings for testing
    }
}
```

**Characteristics:**
- **Latency**: <1ms (no model loading)
- **Use Case**: Development environments, automated testing
- **Storage**: In-memory only
- **Routing**: Fallback for development

#### 3. GraphBertModel - Graph-Aware Specialized Embeddings
**File**: `src/code_graph/graph_bert.rs:158`  
**Trait**: `impl GraphEmbeddingStrategy for GraphBertModel`  
**Model**: Feature engineering (future: ONNX Graph-BERT)  
**Domain**: GRAPH domain  
**Purpose**: Graph-aware embeddings combining code + structural features

```rust
// src/code_graph/graph_bert.rs:158
impl GraphEmbeddingStrategy for GraphBertModel {
    fn embed_with_graph(&self, code_embedding: &[f32], graph_features: &GraphFeatures) -> Vec<f32> {
        self.transform(code_embedding, graph_features)
    }
}
```

**Characteristics:**
- **Input**: CODE embedding + GraphFeatures (degree, edge types)
- **Output**: 384-dimensional graph-aware embedding
- **Use Case**: Code graph fusion, multi-hop reasoning
- **Future**: ONNX Runtime integration for actual Graph-BERT inference

### Embedding Workflow Architecture

```
Input Content
       │
       ▼
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   Domain Check  │───▶│  Trait Selection │───▶│  Model Router  │
│ (Code/Text/Graph)│    │ (Embeddings vs   │    │ (BGE/Hash/GB) │
│                 │    │  GraphStrategy) │    │                 │
└─────────────────┘    └──────────────────┘    └─────────────────┘
       │                       │                       │
       ▼                       ▼                       ▼
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│ HuggingFace     │    │   RealEmbed     │    │   GraphBert    │
│ Embeddings      │    │   dings         │    │    Model        │
│ (CODE domain)   │    │ (Development)   │    │ (GRAPH domain)  │
└─────────────────┘    └──────────────────┘    └─────────────────┘
       │                       │                       │
       ▼                       ▼                       ▼
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│  Vector Store   │    │  Memory Store   │    │  Graph Store   │
│ (HNSW Index)   │    │ (SQLite+Sled)   │    │  (Neo4j)       │
└─────────────────┘    └──────────────────┘    └─────────────────┘
```

### Domain Separation Benefits

1. **No Domain Pollution** - Code embeddings don't contaminate document search
2. **Optimized Models** - Each model trained for specific data type
3. **Specialized Features** - GraphBert adds structural awareness
4. **Development Efficiency** - Fast hash embeddings for testing
5. **Future Extensibility** - Trait-based design allows new embedding types

### Storage Architecture

**CODE Domain** (`syncore_code.index`):
- **Model**: BGE-small-en-v1.5 via fastembed-rs
- **Index**: HNSW with M=16, ef_construction=200
- **Content**: Functions, classes, variables, imports
- **Size**: ~200MB for 10k code entities

**GENERAL Domain** (`syncore_general.index`):
- **Model**: all-MiniLM-L6-v2 via fastembed-rs  
- **Index**: HNSW with M=16, ef_construction=200
- **Content**: Documents, tasks, memory entries, reasoning steps
- **Size**: ~150MB for 10k general entries

**GRAPH Domain** (Neo4j):
- **Model**: GraphBert feature transformation
- **Storage**: Neo4j graph database
- **Content**: Entities + relationships + graph-aware embeddings
- **Size**: Variable based on codebase complexity

## Performance

**Tested on Ryzen 7, 32GB RAM:**

| Operation | Latency |
|-----------|---------|
| Memory store/query | <1ms |
| Vector search (CODE domain) | 10-50ms |
| Vector search (GENERAL domain) | 10-50ms |
| Code graph fusion | 10-50ms |
| Neo4j query | 1-10ms |
| Incremental index (unchanged file) | <1ms |

Note: HNSW indexing reduces search to 1-10ms after warmup, but initial queries use linear scan.

## Requirements

**Required:**
- Rust 1.70+
- ~4GB disk for build artifacts
- ~500MB RAM after embedding model loads

**Optional:**
- Neo4j 5.x (for graph features - `graph_*`, `code_graph_*`, `raggraph_*`)
- Ollama (for AI features - `intellitask_generate/subtasks/prioritize/next`, `sequential_cycle`)

## CLI Tool

`syncore_graph_cli` provides command-line graph operations:

```bash
# Sync SQLite entities to Neo4j
NEO4J_URI="bolt://127.0.0.1:7687" NEO4J_USER="neo4j" NEO4J_PASS="password" \
  ./target/release/syncore_graph_cli sync

# Validate graph integrity
./target/release/syncore_graph_cli validate

# Show graph statistics
./target/release/syncore_graph_cli stats
```

## License

GPL-3.0

## Technical Highlights (Engineer-Grounded)

**What makes this interesting:**
1. **Dual-domain embeddings** - Separate BGE/MiniLM models with automatic routing (rare in MCP servers)
2. **Multi-modal storage** - SQLite + Sled + Neo4j + HNSW in single coherent API
3. **Incremental indexing** - SHA256+mtime change detection, idempotent re-runs
4. **Code graph fusion** - Tri-mode search (simple/attention/reasoning) with temporal scoring
5. **Graph-BERT seam** - Extensibility point for GNN integration via trait abstraction
6. **Zero-dependency project analysis** - Complexity metrics without LLM calls

**What's not production-ready:**
- Neo4j entity queries return empty (sync works but population logic needs debugging)
- Graph-BERT is architectural placeholder (uses deterministic feature injection)
- No streaming for large result sets
- Limited integration test coverage
- Single-node architecture (no replication/sharding)

**Development context:**
- 2 weeks, solo developer, $15 budget
- 100% AI-assisted code generation (Claude Code + free tier models)
- 7866 lines of Rust across 250 files
- Daily production use with Claude Code CLI

## Acknowledgments

Built with rmcp, neo4rs, rusqlite, tree-sitter, fastembed, tokio, sled, hnsw_rs.

Core embedding models: BGE-small-en-v1.5 (BAAI), all-MiniLM-L6-v2 (sentence-transformers).
