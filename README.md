# SynCore

A cognitive micro-kernel MCP (Model Context Protocol) server written in Rust that provides memory, task management, vector search, graph database integration, sequential reasoning, and code intelligence capabilities for LLM-assisted workflows.

## What This Is

SynCore is an experimental MCP server that connects Claude Code (or other MCP clients) to a suite of cognitive tools. It combines:

- **Key-value memory** (SQLite + Sled cache)
- **Task management** with AI-powered breakdown (IntelliTask)
- **Vector similarity search** (FastEmbed embeddings + HNSW)
- **Graph database** (Neo4j integration for knowledge graphs)
- **Code intelligence** (tree-sitter parsing + semantic search)
- **Sequential reasoning** (thought chains with Ollama LLM)
- **Application mapping** (codebase structure tracking)

## Current State (November 2025)

**This is experimental software** with real capabilities but significant limitations.

### What Works

- **50+ MCP tools** accessible via Claude Code
- **Dual transport**: STDIO (Claude Code) and HTTP/SSE (GLM-4.6, other clients)
- **Neo4j integration**: Full Cypher query support for graph operations
- **Semantic code search**: Index and search code by meaning, not just keywords
- **Sequential thinking**: Record and query reasoning chains
- **IntelliTask**: AI-powered task breakdown from PRDs
- **Document indexing**: Full-text search with embeddings
- **Application tracking**: Record code changes and file relationships

### External Dependencies

**Required:**
- **Rust 1.70+** - Build toolchain
- **SQLite3** - Core storage (usually bundled)
- **8GB+ RAM** - FastEmbed model loading

**Optional but recommended:**
- **Neo4j 5.x** - Graph database for knowledge graphs
  ```bash
  # Arch Linux
  yay -S neo4j-community
  sudo systemctl start neo4j

  # Default: bolt://127.0.0.1:7687
  # Username: neo4j, Password: testpassword123
  ```

- **Ollama** - Local LLM for sequential reasoning and IntelliTask
  ```bash
  # Install from ollama.ai
  ollama pull llama3
  ollama serve
  ```

- **Redis** (planned) - High-performance caching layer

### Limitations

- **Not production-ready**: Experimental software, use at your own risk
- **Single-node only**: No distributed mode, no replication
- **No security features**: No authentication, no encryption, no access controls
- **Memory usage**: FastEmbed + HNSW can consume significant RAM
- **Build time**: ~5-10 minutes due to ONNX runtime compilation
- **Neo4j required for graph tools**: Graph features disabled without Neo4j
- **Sparse documentation**: Most features documented through code

### Known Issues

- HNSW index rebuilds on every search after insert (performance hit)
- Sled cache can corrupt on unclean shutdown
- Some IntelliTask prompts assume specific Ollama model behavior
- Test database files accumulate and aren't cleaned automatically
- Vector search can be slow for large datasets without proper indexing

## Building

```bash
# Clone the repository
git clone https://github.com/YOUR_USERNAME/syncore.git
cd syncore

# Build release binary
cargo build --release --bin syncore_mcp_stdio

# Binary location:
# target/release/syncore_mcp_stdio
```

Build time is long (~5-10 minutes) due to fastembed and ONNX runtime compilation.

## Configuration

### Claude Code (Recommended)

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
      "NEO4J_PASS": "testpassword123"
    }
  }
}
```

### Environment Variables

**Core:**
- `DB_PATH` - SQLite database path (default: `syncore.db`)
- `HTTP_PORT` - HTTP/SSE server port (default: `3001`)
- `RUST_LOG` - Log level (`debug`, `info`, `warn`, `error`)

**Neo4j:**
- `NEO4J_URI` - Bolt connection URI (default: `bolt://127.0.0.1:7687`)
- `NEO4J_USER` - Database user (default: `neo4j`)
- `NEO4J_PASS` - Database password (default: `testpassword123`)

**Ollama:**
- `OLLAMA_HOST` - API endpoint (default: `http://localhost:11434`)

**Storage:**
- `SYNCORE_GLOBAL_DIR` - Global storage directory (default: `~/.syncore/`)
- `GRAPH_NAMESPACE` - Neo4j node namespace for isolation

### Starting the Server

```bash
# With defaults (SQLite only, Neo4j auto-connects if available)
./target/release/syncore_mcp_stdio

# With custom config
DB_PATH=./my.db NEO4J_PASS=mypassword ./target/release/syncore_mcp_stdio

# Server logs on startup:
# Starting SynCore MCP servers...
# Database path: syncore.db
# HTTP/SSE server: 127.0.0.1:3001
# Registered Tools (50+):
#   - memory.store
#   - memory.query
#   - graph.query
#   - code.search
#   - ...
```

## Available MCP Tools (50+)

### Memory (2 tools)
- `memory_store` - Store key-value pairs
- `memory_query` - Retrieve values by key

### Task Management (11 tools)
- `task_create` - Create simple tasks
- `intellitask_generate` - Generate task breakdown from PRD
- `intellitask_save` - Save task breakdown to database
- `intellitask_list` - List tasks with filtering
- `intellitask_get` - Get task details
- `intellitask_update_status` - Update task status
- `intellitask_get_subtasks` - Get subtasks for parent
- `intellitask_subtask_stats` - Subtask statistics
- `intellitask_task_statistics` - Overall task stats
- `intellitask_prd_statistics` - PRD-specific stats
- `intellitask_next_ready` - Get next ready task

### Vector Search (2 tools)
- `vector_insert` - Insert text with embeddings
- `vector_search` - Semantic similarity search

### Graph Database (3 tools)
- `graph_query` - Execute Cypher read queries
- `graph_insert` - Execute Cypher write queries
- `graph_relate` - Create relationships between nodes

### Code Intelligence (6 tools)
- `parser_analyze` - Analyze code structure (functions, classes, imports)
- `parser_search` - Ripgrep pattern search
- `code_index` - Index file for semantic search
- `code_search` - Semantic code search
- `code_index_directory` - Batch index files

### Application Mapping (6 tools)
- `mapping_record` - Record file in structure map
- `mapping_get` - Get file node
- `mapping_search` - Search for related files
- `mapping_deps` - Get transitive dependencies
- `application_record` - Record code changes
- `application_search` - Search code change history
- `application_history` - Get file change history
- `application_get` - Get task code changes

### Sequential Reasoning (4 tools)
- `sequential_record` - Record thought step
- `sequential_get` - Get all steps for task
- `sequential_search` - Search thoughts semantically
- `sequential_cycle` - Run reasoning cycles (requires Ollama)

### Document Management (2 tools)
- `document_index` - Index documents into global store
- `document_search` - Semantic document search

### Agent Communication (8 tools)
- `agent_register` - Register agent with capabilities
- `agent_send` - Send message to agent
- `agent_recv` - Receive pending messages
- `agent_poll` - Wait for next message
- `agent_task` - Send structured task
- `agent_result` - Submit task result
- `agent_status` - Update agent status
- `agent_list` - List all agents

### Other (2 tools)
- `logs_tail` - Get recent log entries

## Architecture

```
syncore/
├── src/
│   ├── lib.rs                    # Library entry point
│   ├── mcp_stdio_main.rs         # MCP stdio server
│   ├── mcp_server.rs             # MCP tool handlers
│   ├── mcp/
│   │   └── protocol.rs           # Tool definitions (50+ tools)
│   ├── memory.rs                 # Key-value storage
│   ├── tasks.rs                  # Task management
│   ├── vector.rs                 # Vector search + embeddings
│   ├── graph/
│   │   └── neo4j_client.rs       # Neo4j integration
│   ├── intellitask.rs            # AI task generation
│   ├── parser.rs                 # Tree-sitter code analysis
│   ├── sequential.rs             # Reasoning loops
│   ├── ollama.rs                 # Ollama API client
│   ├── message_bus.rs            # Agent communication
│   ├── global_store.rs           # Global document storage
│   ├── document_indexer.rs       # Document indexing
│   └── tools_cli.rs              # Tool management CLI
├── tests/                        # Integration tests
├── examples/                     # Usage examples
├── schemas/                      # JSON schemas for all tools
├── mcp_tools.json                # Runtime tool manifest
└── Cargo.toml                    # Dependencies
```

## Testing

```bash
# Run all tests (slow, ~10+ minutes)
cargo test

# Run library tests only (faster)
cargo test --lib

# Run specific test
cargo test test_name

# Clean test artifacts
rm -f test_*.db
```

**Note**: Tests create temporary database files. Neo4j tests require a running Neo4j instance.

## Performance Expectations

On a Ryzen 7, 32GB RAM system:

- Memory store/query: <1ms
- Vector search (1000 vectors, HNSW): ~500µs after index build
- Code parsing: 10-100ms depending on file size
- Neo4j queries: 1-10ms for simple queries
- IntelliTask generation: 5-30 seconds (Ollama response time)
- Code semantic search: ~10ms for indexed codebase

These are rough estimates. Your mileage will vary significantly based on:
- Data volume
- Query complexity
- Available RAM
- Neo4j configuration
- Ollama model and hardware

## Why GPL-3.0?

1. This is experimental software built on open source tools
2. Improvements should be shared back with the community
3. Not intended for commercial use in its current state

If you need a different license for a specific use case, open an issue.

## Contributing

This is a personal learning project. Contributions are welcome but don't expect quick responses.

1. Fork the repository
2. Create a branch for your changes
3. Submit a pull request with clear description

## What This Is Not

- **Not production-ready**: Experimental software, no guarantees
- **Not a full knowledge graph**: Basic graph operations, not a reasoning engine
- **Not a vector database**: HNSW support exists but isn't optimized
- **Not an AI agent**: It's tools that an AI can use, not autonomous
- **Not feature-complete**: Many features are work-in-progress
- **Not battle-tested**: Used primarily for personal projects

## Future Directions

No formal roadmap. Development is driven by personal needs:

- Better embeddings (larger models, fine-tuning)
- Redis caching layer
- Improved HNSW performance
- More sophisticated graph operations
- Better error handling and recovery
- Performance optimizations
- Comprehensive documentation

This project may be abandoned, rewritten, or evolved unpredictably.

## License

GPL-3.0. See LICENSE file for details.

## Acknowledgments

This project uses:
- [rmcp](https://github.com/anthropics/rmcp) - MCP protocol implementation
- [neo4rs](https://github.com/neo4j-labs/neo4rs) - Neo4j Rust driver
- [rusqlite](https://github.com/rusqlite/rusqlite) - SQLite bindings
- [hnsw_rs](https://github.com/jean-pierreBoth/hnswlib-rs) - HNSW implementation
- [tree-sitter](https://tree-sitter.github.io/) - Code parsing
- [fastembed](https://github.com/Anush008/fastembed-rs) - Text embeddings
- [tokio](https://tokio.rs/) - Async runtime
- [serde](https://serde.rs/) - Serialization
- And many other excellent Rust crates

Built with significant assistance from Claude Code (Anthropic's AI coding assistant). The code quality reflects iterative AI-assisted development.

---

*Last updated: November 2025 (PHASE 8: LLM Readiness)*
