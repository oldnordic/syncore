# SynCore

**Multi-LLM concurrent MCP server** with cognitive intelligence, RAG-Graph fusion, and persistent knowledge storage.

Built in Rust. Designed for Claude Code and other MCP clients working simultaneously on the same knowledge base.

## What This Actually Is

SynCore is an **experimental cognitive AI server** that provides 70+ tools for intelligent code understanding, task planning, and knowledge management. Multiple LLMs can connect concurrently and share the same knowledge graph.

**Core Capabilities:**
- **Multi-LLM Concurrency** - Claude, GLM, OpenCode can all connect simultaneously via STDIO/HTTP
- **RAG-Graph Fusion** - Tri-mode vector + graph reasoning (Simple, Attention, Reasoning)
- **Cognitive Intelligence** - Intent classification, continuity, pattern mining, self-consistency checks
- **Code Understanding** - Tree-sitter parsing + semantic search for 6 languages
- **Knowledge Graphs** - Neo4j integration with Cypher queries
- **Task Intelligence** - AI-powered task breakdown and planning via Ollama
- **Persistent Memory** - SQLite + Sled dual-layer with WAL guarantees

## Current State (January 2025)

### ✅ What Works (Production-Grade)

**Infrastructure:**
- ✅ **4 concurrent transports**: STDIO (Claude), HTTP-SSE, HTTP-Streaming, TCP (all share same state)
- ✅ **70+ MCP tools** via unified router with auto-generated schemas
- ✅ **DbManager architecture**: Long-lived SQLite connections prevent WAL data loss
- ✅ **Multi-LLM knowledge sharing**: All LLMs see all code graphs, patterns, episodes, memory
- ✅ **Neo4j integration**: Full Cypher support with connection pooling
- ✅ **Test acceleration**: 174x faster tests with FakeEmbeddings layer

**Cognition System (R1-R5 Complete):**
- ✅ **R1 Intent Classification**: Symbolic, Semantic, Causal query routing
- ✅ **R2 Context Fusion**: RAG + Graph + Memory combined retrieval
- ✅ **R3 Reasoning Continuity**: SQL + Graph ledger for episode history
- ✅ **R4 Pattern Mining**: Success/failure pattern recognition and recommendation
- ✅ **R5 Planning Engine**: Multi-step execution plans with tool orchestration

**RAG-Graph System:**
- ✅ **Tri-mode fusion**: Simple (vector-only), Attention (weighted), Reasoning (multi-hop)
- ✅ **Semantic search**: 26,550+ code entities indexed with vector embeddings
- ✅ **Query routing**: Automatic mode selection based on complexity heuristics
- ✅ **Graph scores working**: Neo4j integration validated with 1,243 edges (0.0-0.3 score range)
- ⚠️ **Multi-hop diffusion**: Graph traversal implemented but limited edges (only Contains relationships)

**Code Intelligence:**
- ✅ **Tree-sitter parsing**: Rust, JavaScript, Python, JSON, TOML, Bash
- ✅ **Semantic code search**: Find functions/classes by intent, not just name
- ✅ **Dependency tracking**: Import/export extraction and relationship mapping
- ✅ **Application mapping**: Track codebase evolution with change history

### ⚠️ What's Experimental

**Vector Search:**
- ⚠️ **Linear scan implementation**: O(n) search, no HNSW optimization despite dependency
- ⚠️ **Custom embeddings**: Simple TF-IDF semantic vectors (384-dim), not transformer-based
- ⚠️ **Degrades with scale**: Works well <10k vectors, slow beyond that

**AI Features:**
- ⚠️ **IntelliTask**: Ollama-based task generation (prompt quality varies by model)
- ⚠️ **Sequential reasoning**: Experimental thought chain recording with circuit breaker
- ⚠️ **Pattern confidence scores**: Statistical only, no causal inference

**Graph Relationships:**
- ⚠️ **Edge extraction incomplete**: Only `class → method` Contains relationships extracted. Missing: `calls`, `imports`, `references`, `uses`, `inherits`
- ✅ **Neo4j sync tool ready**: `code_graph_sync_neo4j` fully implemented, waiting for edge extraction to be completed
- ⚠️ **Manual graph construction**: You can manually create relationships via `graph_insert` tool with Cypher

### ❌ What Doesn't Exist

**Architecture:**
- ❌ **HNSW indexing**: Listed in Cargo.toml, NOT IMPLEMENTED (uses linear scan)
- ❌ **Distributed mode**: Single-node only, no replication
- ❌ **Authentication**: No security, access controls, or encryption

**Features:**
- ❌ **Direct LLM communication**: LLMs share knowledge but don't message each other
- ❌ **Autonomous agents**: Tools for agents, not autonomous behavior
- ❌ **Production monitoring**: No metrics, alerting, or health checks
- ❌ **Documentation**: Most features documented through code and TOOLS_MANUAL.md

## Architecture

### Cognitive Stack

```
┌─────────────────────────────────────────────────────────────┐
│                    Multiple LLM Clients                      │
│  Claude (STDIO) │ GLM (HTTP-SSE) │ OpenCode (HTTP-Stream)   │
└───────────────────────────┬─────────────────────────────────┘
                            │
                ┌───────────▼────────────┐
                │   MCP Server (70+ tools) │
                │   Unified Tool Router    │
                └───────────┬─────────────┘
                            │
        ┌───────────────────┼───────────────────┐
        │                   │                   │
┌───────▼────────┐  ┌──────▼──────┐  ┌────────▼────────┐
│   Cognition    │  │  RAG-Graph  │  │  Code Analysis  │
│   ────────────  │  │  ──────────  │  │  ─────────────  │
│ • Intent        │  │ • Tri-mode  │  │ • Tree-sitter   │
│ • Continuity    │  │ • Attention │  │ • Semantic      │
│ • Patterns      │  │ • Diffusion │  │ • Dependencies  │
│ • Planning      │  │ • Fusion    │  │ • Change track  │
└───────┬────────┘  └──────┬──────┘  └────────┬────────┘
        │                   │                   │
        └───────────────────┼───────────────────┘
                            │
        ┌───────────────────▼───────────────────┐
        │         Knowledge Storage             │
        │  ─────────────────────────────────    │
        │  SQLite (WAL) │ Neo4j │ Sled Cache   │
        │  Memory • Tasks • Episodes • Patterns │
        └───────────────────────────────────────┘
```

### Multi-LLM Concurrency Model

```
Claude Code (STDIO)  ┐
GLM (HTTP-SSE)       ├──▶  Shared SynCoreState (Arc)
OpenCode (HTTP)      ┘           │
                                 ├─▶ Unified Code Graph
                                 ├─▶ Shared RAGGraph Entities
                                 ├─▶ Shared Memory (KV store)
                                 ├─▶ Shared Reasoning Episodes
                                 ├─▶ Shared Success/Failure Patterns
                                 └─▶ Shared Task Database

• All LLMs see ALL knowledge (no isolation)
• client_id field is metadata only (attribution, not filtering)
• Namespace separation for different projects
• Concurrent requests don't block each other
```

### Cognition Workflow

```
User Query
    │
    ▼
┌─────────────────────┐
│ Intent Classifier    │ ──▶ Symbolic / Semantic / Causal / Unknown
└──────┬──────────────┘
       │
       ▼
┌─────────────────────┐
│ Context Composer     │ ──▶ Fetch: RAGGraph + Memory + Graph
└──────┬──────────────┘
       │
       ▼
┌─────────────────────┐
│ Continuity Engine    │ ──▶ Load: Past episodes + patterns
└──────┬──────────────┘
       │
       ▼
┌─────────────────────┐
│ Pattern Engine       │ ──▶ Recommend: Success patterns for intent+mode
└──────┬──────────────┘
       │
       ▼
┌─────────────────────┐
│ Self-Consistency     │ ──▶ Validate: Tool order, detect loops, conflicts
└──────┬──────────────┘
       │
       ▼
┌─────────────────────┐
│ Plan Engine          │ ──▶ Generate: Multi-step execution plan
└──────┬──────────────┘
       │
       ▼
┌─────────────────────┐
│ Plan Executor        │ ──▶ Execute: Tools in sequence, handle errors
└──────┬──────────────┘
       │
       ▼
┌─────────────────────┐
│ Reasoning Ledger     │ ──▶ Store: Episode to SQL + Neo4j for next query
└─────────────────────┘
```

## Building

```bash
# Clone repository
git clone https://github.com/YOUR_USERNAME/syncore.git
cd syncore

# Build release binaries (5-10 minutes)
cargo build --release

# Two binaries created:
# - target/release/syncore_mcp_stdio  (STDIO transport for Claude Code)
# - target/release/syncore            (All transports: STDIO + HTTP-SSE + HTTP-Streaming + TCP)
```

**Requirements:**
- Rust 1.70+ with cargo
- C compiler (gcc/clang) for SQLite and ONNX
- ~4GB disk space for build artifacts
- ~2GB RAM during compilation
- ~8GB RAM at runtime (embedding model loading)

## Configuration

### Claude Code Setup (Recommended)

Add to `~/.config/claude/mcp_settings.json`:

```json
{
  "syncore": {
    "command": "/absolute/path/to/syncore/target/release/syncore_mcp_stdio",
    "args": [],
    "env": {
      "DB_PATH": "/absolute/path/to/syncore.db",
      "NEO4J_URI": "bolt://127.0.0.1:7687",
      "NEO4J_USER": "neo4j",
      "NEO4J_PASS": "testpassword123",
      "OLLAMA_HOST": "http://localhost:11434",
      "RUST_LOG": "info"
    }
  }
}
```

### Environment Variables

**Core:**
- `DB_PATH` - Main SQLite database (default: `syncore.db`)
- `RUST_LOG` - Logging level: `debug`, `info`, `warn`, `error`

**Neo4j (Optional but Recommended):**
- `NEO4J_URI` - Bolt connection (default: `bolt://127.0.0.1:7687`)
- `NEO4J_USER` - Database user (default: `neo4j`)
- `NEO4J_PASS` - Database password (required)
- `GRAPH_NAMESPACE` - Node namespace for project isolation

**Ollama (Optional):**
- `OLLAMA_HOST` - API endpoint (default: `http://localhost:11434`)

**Multi-Transport (for main binary):**
- `TRANSPORT` - Which transports to run: `all`, `stdio`, `http-sse`, `http-streaming`, `http` (default: `all`)
- `SSE_ADDR` - HTTP-SSE bind address (default: `127.0.0.1:8081`)
- `STREAM_ADDR` - HTTP-Streaming bind address (default: `127.0.0.1:8082`)
- `SOCKET_PATH` - TCP bind address (default: `127.0.0.1:8080`)

## Running the Server

### STDIO Mode (Claude Code)

```bash
# Standard mode (single LLM via STDIO)
./target/release/syncore_mcp_stdio

# With custom config
DB_PATH=./my.db \
NEO4J_PASS=mypassword \
./target/release/syncore_mcp_stdio
```

### Multi-LLM Mode (All Transports)

```bash
# Run all 4 transports concurrently (default)
./target/release/syncore

# Now connect:
# - Claude Code via STDIO (auto-connected via mcp_settings.json)
# - GLM/OpenCode via HTTP-SSE on http://127.0.0.1:8081
# - Other clients via HTTP-Streaming on http://127.0.0.1:8082
# - Legacy clients via TCP on 127.0.0.1:8080

# All clients share the same knowledge base!
```

### HTTP Endpoints

When running HTTP transports:
- `GET /` - Server info (name, version, tools)
- `GET /mcp/v1/info` - MCP protocol metadata
- `GET /mcp/v1/tools` - List all 70+ tools with schemas

## Available Tools

See [`TOOLS_MANUAL.md`](TOOLS_MANUAL.md) for complete documentation of all 70+ tools.

**Categories:**
- **Memory** (2): Key-value storage with SQLite + Sled cache
- **Tasks** (1): Task creation with parent/dependency links
- **Vector** (2): Semantic embedding and similarity search
- **Code** (5): Parsing, indexing, semantic search, dependency tracking
- **Graph** (3): Neo4j Cypher queries and relationship creation
- **RAGGraph** (2): Tri-mode fusion query and multi-hop diffusion
- **Application Mapping** (8): File tracking, change history, dependency analysis
- **Sequential Reasoning** (4): Thought chain recording and search
- **IntelliTask** (13): AI-powered task breakdown and prioritization
- **Agent Communication** (8): Message bus and task routing
- **Document** (2): Document indexing and semantic search
- **System** (2): Logs and tool metadata

## External Dependencies

**Required:**
- Rust 1.70+ toolchain
- SQLite3 (bundled with rusqlite)
- 8GB+ RAM for embedding models

**Optional but Recommended:**
- **Neo4j 5.x** - Graph database for knowledge graphs
  ```bash
  # Arch Linux
  yay -S neo4j-community
  sudo systemctl start neo4j
  sudo neo4j-admin dbms set-initial-password testpassword123
  ```

- **Ollama** - Local LLM for IntelliTask and sequential reasoning
  ```bash
  # Install from ollama.ai
  ollama pull llama3
  ollama serve
  ```

## Performance Expectations

**Tested on Ryzen 7, 32GB RAM:**

| Operation | Latency | Notes |
|-----------|---------|-------|
| Memory store/query | <1ms | SQLite + Sled |
| Code graph fusion query | 10-50ms | Depends on mode (simple/attention/reasoning) |
| Vector search (1k) | ~10ms | Linear scan O(n) |
| Vector search (10k) | ~100ms | Degrades linearly |
| Tree-sitter parse | 10-200ms | File size dependent |
| Neo4j simple query | 1-10ms | Network + query |
| IntelliTask generation | 5-30s | Ollama LLM latency |

**Memory Usage:**
- Base: ~50MB runtime
- After embeddings: ~500MB (model in RAM)
- With Neo4j: ~1GB (Java heap)
- Peak indexing: ~1.5GB

## Honest Limitations

**Architecture:**
- Single-node only (no replication, no distributed mode)
- No authentication, encryption, or access controls
- No rate limiting (except sequential reasoning circuit breaker)
- Linear vector search (O(n), no HNSW optimization)

**Performance:**
- Vector search degrades linearly with dataset size
- Edge extraction partially implemented (only class → method `Contains` relationships)
- Ollama dependency adds 5-30s latency for AI features

**Stability:**
- Sled cache can corrupt on unclean shutdown (auto-recovery implemented)
- Test database files accumulate in /tmp (manual cleanup)
- Some Ollama prompts assume llama3 model behavior

**Known Issues:**
- **Edge extraction incomplete**: Indexing extracts 26,550+ code entities but limited edges. Only `class → method` Contains relationships are extracted. Missing: `calls`, `imports`, `references`, `uses`, `inherits` extraction.
- **Neo4j sync working**: `code_graph_sync_neo4j` fully operational with 1,243 edges synced. Graph scores: 0.0-0.3 range.
- Test artifacts need manual cleanup: `rm -f /tmp/syncore_test_*.db*`

**Recent Fixes (Jan 2025):**
- ✅ Fixed Neo4j graph_score=0 bug (vector snapshot ID mismatch, :memory: database, silent failures)
- ✅ Added snapshot validation: Vector IDs now validated against SQLite on load
- ✅ Added debug logging: Failed lookups now logged with actionable error messages
- ✅ All fixes TDD-tested: 8 new tests covering edge cases

## Testing

```bash
# Fast tests with FakeEmbeddings (8 seconds)
cargo test

# With output
cargo test -- --nocapture

# Specific test
cargo test test_name

# Neo4j integration tests (requires Neo4j running)
NEO4J_URI=bolt://127.0.0.1:7687 \
NEO4J_USER=neo4j \
NEO4J_PASS=testpassword123 \
cargo test --test real_executor_graph_tests
```

## Development

### Adding New Tools

1. Define request struct with `#[derive(Deserialize, JsonSchema)]`
2. Add `#[tool]` handler in `src/mcp_server.rs`
3. RMCP auto-generates schemas and routing
4. Test with `cargo test`

Example:
```rust
#[derive(Deserialize, JsonSchema)]
struct MyToolRequest {
    query: String,
}

#[tool]
async fn my_tool(state: ToolState, req: MyToolRequest) -> Result<String> {
    Ok(format!("Result: {}", req.query))
}
```

### Understanding the Cognition Stack

Key files for reasoning intelligence:
- `src/cognition/orchestrator.rs` - Main cognitive entry point
- `src/cognition/intent_classifier.rs` - Query intent detection
- `src/cognition/continuity_engine.rs` - Reasoning history
- `src/cognition/pattern_engine.rs` - Success/failure patterns
- `src/cognition/plan_engine.rs` - Multi-step planning
- `src/cognition/self_consistency.rs` - Validation and anomaly detection

### Understanding RAG-Graph Fusion

Key files for tri-mode retrieval:
- `src/raggraph/fusion.rs` - Mode selection and fusion logic
- `src/raggraph/attention.rs` - Attention-weighted combination
- `src/raggraph/diffusion.rs` - Multi-hop graph traversal
- `src/raggraph/hopgraph.rs` - Graph structure and operations

## Why GPL-3.0?

1. Built on open source (Neo4j Community, Ollama, RMCP, Tree-sitter)
2. Experimental software should be shared openly
3. Improvements benefit the community
4. Not intended for proprietary commercial use

**Need different license?** Open an issue with use case.

## Contributing

Personal learning project. Contributions welcome but no promises on response time.

1. Fork repository
2. Create feature branch
3. Write tests (use `cargo test` for fast iteration)
4. Submit PR with clear rationale
5. Follow existing code style

## What This Is NOT

- ❌ **Not production-ready**: No security, no guarantees, experimental features
- ❌ **Not a vector database**: Linear search, no HNSW optimization
- ❌ **Not an AGI system**: Tools for AI agents, not autonomous intelligence
- ❌ **Not feature-complete**: Many features experimental or work-in-progress
- ❌ **Not well-documented**: Code and TOOLS_MANUAL are primary docs
- ❌ **Not battle-tested**: Personal projects only, limited real-world use

## Future Directions

No formal roadmap. Development driven by personal needs:

**Short-term (maybe):**
- [ ] Complete Neo4j relationship extraction (edges pending)
- [ ] Real HNSW implementation (currently linear scan)
- [ ] Redis caching for agent communication
- [ ] Better error messages and recovery

**Long-term (aspirational):**
- [ ] Distributed mode with replication
- [ ] Authentication and access controls
- [ ] Transformer-based embeddings
- [ ] Comprehensive documentation site
- [ ] Performance profiling and optimization

**Never (out of scope):**
- Commercial support or SLAs
- Cloud-hosted service
- Multi-tenancy
- Enterprise features

## License

GNU General Public License v3.0 (GPL-3.0)

See LICENSE file for full text.

## Acknowledgments

Built with:
- [rmcp](https://github.com/anthropics/rmcp) - MCP protocol
- [neo4rs](https://github.com/neo4j-labs/neo4rs) - Neo4j driver
- [rusqlite](https://github.com/rusqlite/rusqlite) - SQLite with WAL
- [tree-sitter](https://tree-sitter.github.io/) - Multi-language parsing
- [fastembed](https://github.com/Anush008/fastembed-rs) - Text embeddings
- [tokio](https://tokio.rs/) - Async runtime
- [sled](https://github.com/spacejam/sled) - Key-value store

Special thanks to Claude Code (Anthropic) for AI-assisted development.

---

**Status**: Active development (January 2025)
**Phase**: Post-R5 Cognition, Multi-LLM Concurrency, RAG-Graph Fusion
**Next**: Complete Neo4j relationship extraction, Real HNSW implementation

*"Honest software: Tell users what works, what doesn't, and why."*
