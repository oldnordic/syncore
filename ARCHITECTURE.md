# SynCore Architecture

**Last Updated**: December 2025
**Version**: 0.2.0
**Architecture**: MCP Server with Multi-Modal Storage

## Overview

SynCore is an AI-native Model Context Protocol (MCP) server that bridges the gap between AI agents and persistent, structured knowledge storage. It provides 65+ tools organized into 5 suites for intelligent code development workflows.

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    MCP Client (Claude/CLI)                    │
└─────────────────────┬───────────────────────────────────────┘
                      │ (stdio protocol)
┌─────────────────────▼───────────────────────────────────────┐
│                SynCore MCP Server                           │
├─────────────────────────────────────────────────────────────┤
│  Tool Router (routes MCP calls to appropriate handlers)     │
│  ├─ Memory Suite (42 commands)                             │
│  ├─ Code Suite (11 commands)                               │
│  ├─ Graph Suite (5 commands)                               │
│  ├─ Debug Suite (10 commands)                              │
│  └─ Mapping Suite (8 commands)                             │
└─────────────────────┬───────────────────────────────────────┘
                      │
┌─────────────────────▼───────────────────────────────────────┐
│                    Storage Layer                            │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐  ┌─────────────────┐  ┌──────────────┐ │
│  │   Memory        │  │    Vector       │  │    Graph     │ │
│  │   Storage       │  │    Search       │  │    Database  │ │
│  │                 │  │                 │  │              │ │
│  │ SQLite + Sled   │  │ 384-dim Embeds  │  │ SQLiteGraph  │ │
│  │ (Persistent +   │  │ Linear Scan     │  │ Neo4j Sync   │ │
│  │ Fast Cache)     │  │ (not HNSW)      │  │              │ │
│  └─────────────────┘  └─────────────────┘  └──────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

## Core Components

### 1. MCP Server Layer

**Location**: `src/mcp_server/`
**Technology**: `rmcp` library with stdio transport
**Responsibility**: MCP protocol handling, tool registration, request routing

**Key Files**:
- `server.rs`: Main MCP server implementation (2400+ LOC)
- `types.rs`: MCP protocol types and tool definitions
- `stdio_handshake.rs`: Client initialization and capability negotiation

### 2. Tool Router

**Location**: `src/router.rs` (226 LOC)
**Responsibility**: Route tool calls to appropriate handlers
**Architecture**: Centralized dispatcher with tool registry

```rust
// Tool routing flow
MCP Request → Tool Router → Suite Handler → Storage Layer → Response
```

### 3. Memory Suite

**Location**: `src/mcp_tools/memory_suite.rs` (1900+ LOC)
**Commands**: 42 (largest suite)
**Storage**: Hybrid SQLite + Sled cache

**Storage Architecture**:
```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   SQLite        │    │     Sled        │    │   Cache Layer   │
│   (Persistent)  │◄──►│    (Fast)       │◄──►│   (LRU Cache)   │
│                 │    │                 │    │                 │
│ • Key-Value     │    │ • Hot Data      │    │ • Recent Queries│
│ • Tasks         │    │ • Embeddings    │    │ • Metadata      │
│ • Steps         │    │ • Sessions      │    │                 │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

**Database Schema**:
```sql
-- Core memory storage
CREATE TABLE memory (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    k TEXT NOT NULL UNIQUE,
    v TEXT NOT NULL,
    ts INTEGER NOT NULL
);

-- Task management with relationships
CREATE TABLE tasks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    goal TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'open',
    priority INTEGER NOT NULL DEFAULT 3,
    parent_id INTEGER REFERENCES tasks(id),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

-- Task relationships (graph edges)
CREATE TABLE task_links (
    src_id INTEGER NOT NULL REFERENCES tasks(id),
    dst_id INTEGER NOT NULL REFERENCES tasks(id),
    kind TEXT NOT NULL,
    PRIMARY KEY (src_id, dst_id, kind)
);

-- Sequential reasoning steps
CREATE TABLE steps (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id INTEGER REFERENCES tasks(id),
    state TEXT NOT NULL,  -- Think, Decide, Act, Observe, Reflect
    content TEXT NOT NULL,
    created_at INTEGER NOT NULL
);
```

### 4. Vector Search System

**Location**: `src/vector.rs` (1800+ LOC)
**Embedding Dimensions**: 384
**Search Algorithm**: Linear scan O(n) (not HNSW despite dependency)

**Embedding Architecture**:
```
Input Text
     │
     ▼
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│  HuggingFace    │    │   RealEmbed     │    │  FastEmbed     │
│  Embeddings     │    │   dings         │    │  (Legacy)      │
│                 │    │                 │    │                 │
│ • BGE-small     │    │ • Hash-based    │    │ • TF-IDF       │
│ • MiniLM        │    │ • Deterministic │    │ • Cache         │
│ • Production    │    │ • Development   │    │ • Fallback      │
└─────────────────┘    └─────────────────┘    └─────────────────┘
     │                       │                       │
     ▼                       ▼                       ▼
┌─────────────────────────────────────────────────────────────┐
│                   Vector Store                            │
│  • Linear Search (O(n))                                     │
│  • Cosine Similarity                                       │
│  • Metadata Storage                                         │
│  • Snapshot Persistence                                    │
└─────────────────────────────────────────────────────────────┘
```

**Known Issues**:
- Linear search scales poorly with large datasets
- HNSW dependency exists but not implemented
- No approximate nearest neighbor optimization

### 5. Code Intelligence System

**Location**: `src/parser.rs` (1000+ LOC)
**Languages**: Rust, JavaScript, Python, JSON, TOML, Bash
**Technology**: Tree-sitter parsing with optional persistence

**Parsing Pipeline**:
```
Source File
     │
     ▼
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│  Tree-sitter    │    │   Entity        │    │   Storage       │
│  Parser         │───▶│   Extraction    │───▶│   Layer         │
│                 │    │                 │    │                 │
│ • AST Build     │    │ • Functions     │    │ • SQLite        │
│ • Language Spec │    │ • Structs       │    │ • HNSW (planned)│
│ • Error Recovery│    │ • Imports       │    │ • Neo4j         │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

**Entity Types**:
- Functions, Methods, Constructors
- Structs, Classes, Interfaces
- Constants, Variables, Fields
- Imports, Modules, Files

### 6. Graph Database System

**Location**: `src/graph/`
**Primary Backend**: SQLiteGraph (embedded)
**Optional Backend**: Neo4j (external)
**Configuration**: Precedence: config file → env vars → defaults

**Graph Storage Architecture**:
```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   SQLiteGraph   │    │     Neo4j       │    │   Sync Layer    │
│   (Default)     │    │   (Optional)    │    │                 │
│                 │    │                 │    │                 │
│ • Embedded      │◄──►│ • Professional  │◄──►│ • Bidirectional │
│ • Zero Config   │    │ • Cypher Queries │    │ • Conflict Res. │
│ • Fast Local    │    │ • Visualization │    │ • Validation    │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

**Entity Schema**:
```sql
CREATE TABLE entities (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_path TEXT NOT NULL,
    entity_type TEXT NOT NULL,  -- Function, Struct, etc.
    name TEXT NOT NULL,
    line_start INTEGER,
    line_end INTEGER,
    char_start INTEGER,
    char_end INTEGER,
    visibility TEXT,
    signature TEXT,
    docstring TEXT,
    created_at INTEGER DEFAULT CURRENT_TIMESTAMP
);
```

**Relationship Schema**:
```sql
CREATE TABLE relationships (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    src_entity_id INTEGER NOT NULL,
    dst_entity_id INTEGER NOT NULL,
    relationship_type TEXT NOT NULL,  -- CALLS, USES, IMPORTS, etc.
    line_number INTEGER,
    metadata_json TEXT DEFAULT '{}',
    FOREIGN KEY (src_entity_id) REFERENCES entities(id),
    FOREIGN KEY (dst_entity_id) REFERENCES entities(id)
);
```

### 7. Project Analysis Engine

**Location**: `src/databases/`
**Analysis Type**: Static, deterministic (no LLM required)
**Complexity Metrics**: Fan-in/out, LOC, entity coupling

**Analysis Pipeline**:
```
Codebase
   │
   ▼
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│  Graph Query    │    │   Metric        │    │   Report        │
│  Engine         │───▶│   Calculation   │───▶│   Generation    │
│                 │    │                 │    │                 │
│ • Entity Graph  │    │ • Hotspot Score │    │ • File Reports  │
│ • Dependency    │    │ • Complexity    │    │ • Refactor Sugg.│
│ • Traversal     │    │ • Coupling      │    │ • Health Score  │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

**Analysis Tools**:
- `project_hotspots`: Files with high coupling/complexity
- `project_dead_code`: Unused entities detection
- `project_cycles`: Circular dependency detection
- `project_refactor_suggestions`: Automated improvement suggestions

## Data Flow Architecture

### MCP Request Flow
```
1. MCP Client → stdio → SynCore Server
2. Tool Router → Parse Request
3. Suite Handler → Business Logic
4. Storage Layer → Database Operations
5. Response → MCP Client
```

### Code Indexing Flow
```
1. code_index_directory(file_pattern)
2. For each file:
   a. Check SHA256+mtime (incremental)
   b. Tree-sitter parse if changed
   c. Extract entities (functions, structs, etc.)
   d. Store in SQLite entities table
   e. Create relationships (CALLS, USES, IMPORTS)
   f. Generate embeddings for semantic search
3. Sync to Neo4j if enabled
```

### Vector Search Flow
```
1. User Query → Embedding Generation
2. Linear Scan through all embeddings
3. Cosine Similarity Calculation
4. Top-K Selection
5. Metadata Enrichment
6. Ranked Results
```

### Graph Query Flow
```
1. Cypher Query Parse
2. SQLiteGraph Execution (or Neo4j)
3. Result Aggregation
4. Relationship Traversal
5. JSON Response Formatting
```

## Configuration Architecture

### Configuration Precedence
```
1. config/syncore.toml (highest priority)
2. Environment Variables
3. Built-in Defaults (fallback)
```

### Configuration File Structure
```toml
[graph]
backend = "sqlite"  # or "neo4j"
sqlite_db_path = "syncore_code_graph.db"

[neo4j]  # Only if backend = "neo4j"
uri = "bolt://127.0.0.1:7687"
user = "neo4j"
password = ""

[vector]
dimensions = 384
model = "BGE-small-en-v1.5"
cache_dir = ".fastembed_cache"
```

## Performance Characteristics

### Tested on Ryzen 7, 32GB RAM

| Operation | Latency | Notes |
|-----------|---------|-------|
| Memory store/query | <1ms | SQLite + Sled cache |
| Vector search (1k items) | 10-50ms | Linear scan O(n) |
| Vector search (10k items) | 100-500ms | Linear scale |
| Code parsing (1000 LOC) | 50-200ms | Tree-sitter |
| Graph query (SQLite) | 1-10ms | Depends on complexity |
| Graph query (Neo4j) | 5-50ms | Network latency |
| Incremental index (unchanged) | <1ms | SHA256 check |
| Full project index | 1-5min | Depends on size |

### Memory Usage

- **Base**: ~100MB (Rust runtime + SQLite)
- **Embedding Models**: ~400MB (BGE + MiniLM via fastembed)
- **Working Set**: 100-500MB depending on indexed content
- **Total Typical**: 500-1000MB

### Storage Requirements

- **SQLite Database**: ~10MB per 10k entities
- **Vector Embeddings**: ~4MB per 10k embeddings (384-dim float32)
- **Code Graph**: ~20MB per 10k entities + relationships
- **Snapshots**: Additional ~50% for versioning

## Security Architecture

### Current State: Local Development Focus
- **No Authentication**: Designed for local use only
- **No Encryption**: All data stored in plaintext
- **No Network Services**: stdio only, no HTTP endpoints
- **File System Access**: Full read/write to configured paths

### Security Considerations
- Database files should be protected by OS permissions
- No input validation on file paths (assumes trusted client)
- No rate limiting or resource quotas
- No audit logging or access controls

## Integration Points

### External Dependencies

**Required**:
- `rmcp`: MCP protocol implementation
- `rusqlite`: SQLite database access
- `tree-sitter`: Code parsing
- `fastembed`: Embedding models
- `tokio`: Async runtime

**Optional**:
- `neo4rs`: Neo4j connectivity
- External LLM backend (Ollama, OpenAI API)
- `hnsw_rs`: HNSW indexing (not implemented)

### LLM Integration
```rust
// Sequential reasoning requires external LLM
pub trait LLMBackend {
    async fn generate(&self, prompt: &str) -> Result<String>;
    async fn chat(&self, messages: &[ChatMessage]) -> Result<String>;
}

// Current implementations:
// - GGUFEngine (local models)
// - OllamaEngine (Ollama API)
// - TestEngine (deterministic responses)
```

## Known Architectural Limitations

### Technical Debt
1. **Linear Vector Search**: O(n) complexity, no HNSW implementation
2. **Graph-BERT Placeholder**: Feature injection instead of real model inference
3. **Single-Threaded Storage**: SQLite connection pool not optimized
4. **Memory Management**: No memory pooling for large embeddings

### Scalability Issues
1. **Single-Node**: No distributed architecture
2. **No Streaming**: Large result sets loaded into memory
3. **No Caching Layer**: Repeated expensive computations
4. **No Rate Limiting**: Potential for resource exhaustion

### Design Trade-offs
1. **Simplicity vs Performance**: Chose simple linear search over complex HNSW
2. **Local vs Distributed**: Prioritized ease of use over scalability
3. **Embedded vs External**: SQLiteGraph for zero-config vs Neo4j for features
4. **Static vs Dynamic**: Deterministic analysis vs LLM-powered insights

## Future Architecture Plans

### Phase 1: Performance Optimization
- Implement real HNSW vector indexing
- Add connection pooling for databases
- Optimize memory usage for large embeddings
- Add result streaming for large queries

### Phase 2: Distributed Features
- Multi-node synchronization
- Distributed vector search
- Horizontal scaling capabilities
- Load balancing and failover

### Phase 3: Advanced Features
- Real Graph-BERT integration
- Streaming updates and real-time indexing
- Advanced caching strategies
- Performance monitoring and metrics

### Phase 4: Security & Enterprise
- Authentication and authorization
- Encryption at rest and in transit
- Audit logging and compliance
- Multi-tenant isolation

## Development Guidelines

### Code Organization
```
src/
├── mcp_server/          # MCP protocol handling
├── mcp_tools/           # Tool implementations
│   ├── memory_suite.rs  # 42 commands
│   ├── graph_suite.rs   # 5 commands
│   ├── debug_suite.rs   # 10 commands
│   └── ...
├── databases/           # Storage backends
├── graph/              # Graph database logic
├── vector/             # Vector search implementation
├── parser/             # Code parsing logic
└── lib.rs              # Public API
```

### Adding New Tools
1. Define request/response structs in `src/mcp_tools/`
2. Implement handler with `#[tool_handler]` macro
3. Register in `create_tool_router()` function
4. Add documentation to `MANUAL.md`
5. Update test coverage

### Testing Strategy
- **Unit Tests**: Per-module logic testing
- **Integration Tests**: Cross-module workflow testing
- **E2E Tests**: Full MCP protocol testing
- **Performance Tests**: Latency and scalability validation

---

**Architecture Maintainers**: Development Team
**Last Review**: December 2025
**Next Review**: As needed for major changes