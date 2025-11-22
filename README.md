# SynCore

**MCP server for AI-assisted development** with persistent memory, vector search, code intelligence, and Neo4j graph integration.

Built in Rust. Designed for Claude Code and other MCP clients.

## What This Actually Does

SynCore is an MCP (Model Context Protocol) server exposing **62 tools** for:
- **Persistent Memory** - Key-value storage (SQLite + Sled cache)
- **Vector Search** - Semantic search with HNSW indexing + brute-force fallback
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

**Project Analysis Engine (NEW - PHASE 6):**
- `project_file_report` - Detailed file analysis (entities, relationships, metrics)
- `project_hotspots` - Find complexity hotspots (fan-in/out, LOC, entity count)
- `project_dead_code` - Detect unused entities
- `project_cycles` - Find circular dependencies
- `project_unused_imports` - Find unused imports
- `project_module_map` - Module dependency map
- `project_refactor_suggestions` - Heuristic refactoring suggestions

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
- **TF-IDF embeddings** - Not transformer-based (fastembed gives ~384 dim vectors)
- **Neo4j required for graph features** - Some tools fail silently without it
- **Ollama required for AI features** - IntelliTask, sequential_cycle need it
- **~500MB RAM after startup** - Embedding model needs to load

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

## All 62 Tools

### Memory (2 tools)
| Tool | Description | Cost |
|------|-------------|------|
| `memory_store` | Store key-value pair | Low |
| `memory_query` | Retrieve value by key | Low |

### Vector Search (2 tools)
| Tool | Description | Cost |
|------|-------------|------|
| `vector_insert` | Add text with embeddings | Medium |
| `vector_search` | Semantic similarity search | Medium |

### Code Intelligence (5 tools)
| Tool | Description | Cost |
|------|-------------|------|
| `code_index` | Index a source file | High |
| `code_search` | Semantic code search | Medium |
| `code_index_directory` | Batch index directory (incremental) | Very High |
| `parser_analyze` | Tree-sitter AST extraction, `persist=true` writes to DB/HNSW/Neo4j | Medium |
| `parser_search` | Ripgrep pattern search | Medium |

### Documents (2 tools)
| Tool | Description | Cost |
|------|-------------|------|
| `document_index` | Index documents from directory | Very High |
| `document_search` | Semantic document search | Medium |

### Graph Database (3 tools)
| Tool | Description | Cost |
|------|-------------|------|
| `graph_query` | Execute Cypher read query | High |
| `graph_insert` | Execute Cypher write query | High |
| `graph_relate` | Create relationship between nodes | High |

### Code Graph (4 tools)
| Tool | Description | Cost |
|------|-------------|------|
| `code_graph_fusion_query` | Tri-mode (simple/attention/reasoning) search | High |
| `code_graph_sync_neo4j` | Sync SQLite entities to Neo4j | Very High |
| `code_graph_enrich_temporal` | Add git history + mtime metadata | Very High |
| `raggraph_multihop` | Multi-hop graph diffusion from seed nodes | High |

### RAG (1 tool)
| Tool | Description | Cost |
|------|-------------|------|
| `raggraph_query` | RAG query with multi-hop graph reasoning | High |

### Task Management (11 tools)
| Tool | Description | Cost |
|------|-------------|------|
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

### Agent Coordination (8 tools)
| Tool | Description | Cost |
|------|-------------|------|
| `agent_send` | Send message to agent via message bus | Low |
| `agent_recv` | Receive pending messages | Low |
| `agent_poll` | Wait for next message (blocking) | Low |
| `agent_register` | Register agent with capabilities | Low |
| `agent_list` | List registered agents | Low |
| `agent_status` | Update agent status | Low |
| `agent_task` | Send structured task envelope | Low |
| `agent_result` | Submit completed task result | Low |

### Application Mapping (4 tools)
| Tool | Description | Cost |
|------|-------------|------|
| `mapping_record` | Record file node with imports/exports/deps | Low |
| `mapping_get` | Get file node | Low |
| `mapping_search` | Semantic file search | Medium |
| `mapping_deps` | Get transitive dependencies | Medium |

### Sequential Reasoning (4 tools)
| Tool | Description | Cost |
|------|-------------|------|
| `sequential_record` | Record thought step in reasoning chain | Low |
| `sequential_get` | Get thought steps for task | Low |
| `sequential_search` | Search thought steps semantically | Medium |
| `sequential_cycle` | **Requires Ollama** - Run reasoning cycle | Very High |

### Application Change Tracking (4 tools)
| Tool | Description | Cost |
|------|-------------|------|
| `application_record` | Record code change | Low |
| `application_get` | Get changes for task | Low |
| `application_history` | Get file change history | Low |
| `application_search` | Search changes semantically | Medium |

### System (2 tools)
| Tool | Description | Cost |
|------|-------------|------|
| `logs_tail` | Get recent log entries | Low |
| `tool_metadata_list` | List tool metadata (only shows 14 core tools) | Low |

### Project Analysis (7 tools)
| Tool | Description | Cost |
|------|-------------|------|
| `project_file_report` | Detailed analysis of a single file (entities, relationships, imports, metrics) | Medium |
| `project_hotspots` | Find code complexity hotspots by fan-in, fan-out, LOC, entity count | Medium |
| `project_dead_code` | Identify potentially unused entities (no incoming references) | Medium |
| `project_cycles` | Detect circular dependencies between files | Medium |
| `project_unused_imports` | Find imports that aren't used | Medium |
| `project_module_map` | Generate module-level dependency map | Medium |
| `project_refactor_suggestions` | Generate heuristic-based refactoring suggestions | Medium |

## Performance

**Tested on Ryzen 7, 32GB RAM:**

| Operation | Latency |
|-----------|---------|
| Memory store/query | <1ms |
| Vector search (cold) | 10-50ms |
| Vector search (hot HNSW) | 1-10ms |
| Code graph fusion | 10-50ms |
| Neo4j query | 1-10ms |
| Incremental index (unchanged file) | <1ms |

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

## Acknowledgments

Built with rmcp, neo4rs, rusqlite, tree-sitter, fastembed, tokio, sled, hnsw_rs.
