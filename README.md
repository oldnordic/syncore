# SynCore

An MCP (Model Context Protocol) server written in Rust that provides memory, task management, vector search, and code analysis capabilities.

## What This Is

SynCore is an experimental MCP server that I built to learn Rust and explore AI-assisted development workflows. It connects to Claude Desktop (or other MCP clients) and provides tools for:

- Key-value memory storage (SQLite + optional Sled cache)
- Task and subtask management
- Vector similarity search (basic embeddings)
- Tree-sitter code parsing (Rust, Python, JavaScript, JSON, TOML, Bash)
- Sequential reasoning loops (requires external Ollama instance)
- AI-powered task generation from PRDs (requires Ollama)

## Current State

**This is experimental software.** It works for my personal use case but has significant limitations:

### What Works
- MCP protocol via stdio transport
- Basic CRUD operations for memory and tasks
- Code parsing with tree-sitter
- Vector search with cosine similarity
- Integration with Claude Desktop

### Limitations
- **Vector search is slow**: Uses linear scan by default (O(n)). HNSW support added but largely untested.
- **Basic embeddings**: Uses fastembed with a small model, not production-grade semantic understanding.
- **Requires Ollama**: Sequential reasoning and IntelliTask features need a running Ollama instance with llama3 or similar.
- **Single-node only**: No distributed mode, no replication.
- **No security features**: No authentication, no encryption, no access controls.
- **Limited error handling**: Some edge cases not handled gracefully.
- **Sparse documentation**: Most of this was built iteratively with AI assistance.
- **Not battle-tested**: Used only by me for personal projects.

### Known Issues
- Sled cache can corrupt on unclean shutdown (auto-recovery attempts exist but aren't reliable)
- HNSW index rebuilds on every search after insert (performance hit for dynamic workloads)
- Some IntelliTask prompts assume specific Ollama model behavior
- Test database files accumulate and aren't cleaned automatically

## Building

Requirements:
- Rust 1.70+ (may work with earlier versions, not tested)
- SQLite development headers (usually bundled)
- 8GB+ RAM recommended (fastembed model loading)

```bash
# Clone the repository
git clone https://github.com/YOUR_USERNAME/syncore.git
cd syncore

# Build release binary
cargo build --release --bin syncore_mcp_stdio

# The binary will be at:
# target/release/syncore_mcp_stdio
```

Build time is long (~2-5 minutes) due to fastembed and ONNX runtime compilation.

## Configuration

### Claude Desktop

Add to your Claude Desktop config (`~/.config/claude-desktop/claude_desktop_config.json` on Linux):

```json
{
  "mcpServers": {
    "syncore": {
      "command": "/path/to/syncore/target/release/syncore_mcp_stdio",
      "args": [],
      "env": {}
    }
  }
}
```

Restart Claude Desktop after configuration.

### Environment Variables

- `DB_PATH`: SQLite database path (default: `syncore.db`)
- `SYNCORE_GLOBAL_DIR`: Global storage directory (default: `~/.syncore/`)
- `OLLAMA_HOST`: Ollama API endpoint (default: `http://localhost:11434`)
- `RUST_LOG`: Log level (`debug`, `info`, `warn`, `error`)

### For Sequential Reasoning

Install and run Ollama:
```bash
# Install Ollama (see ollama.ai for your platform)
ollama pull llama3
ollama serve
```

## Available MCP Tools

After connecting to Claude Desktop, these tools become available:

**Memory**
- `memory_store` - Store key-value pairs
- `memory_query` - Retrieve values by key

**Tasks**
- `intellitask_generate` - Generate task breakdown from PRD text
- `intellitask_save` - Save task breakdown to database
- `intellitask_list` - List tasks with optional filtering
- `intellitask_get` - Get task details by ID
- `intellitask_update_status` - Update task status
- `intellitask_subtasks` - Generate subtasks for a parent task
- `intellitask_prioritize` - Prioritize tasks using AI
- `intellitask_next` - Suggest next task to work on
- `task_create` - Create a simple task

**Vector Search**
- `vector_insert` - Insert text with embeddings
- `vector_search` - Search for similar text

**Code Analysis**
- `parser_analyze` - Analyze code structure (functions, classes, imports)
- `parser_search` - Search code patterns with ripgrep
- `code_index` - Index a file for semantic search
- `code_search` - Semantic code search
- `code_index_directory` - Index all matching files in a directory

**Other**
- `sequential_cycle` - Run reasoning cycles (requires Ollama)
- `document_index` - Index documents into global store
- `document_search` - Search indexed documents
- `logs_tail` - Get recent log entries

## Project Structure

```
syncore/
├── src/
│   ├── lib.rs              # Library entry point
│   ├── main.rs             # TCP server (legacy)
│   ├── bin/                # Binary entrypoints
│   │   └── syncore_mcp_stdio.rs  # MCP stdio server
│   ├── mcp_server.rs       # MCP tool handlers
│   ├── memory.rs           # Key-value storage
│   ├── tasks.rs            # Task management
│   ├── vector.rs           # Vector search
│   ├── intellitask.rs      # AI task generation
│   ├── parser.rs           # Tree-sitter code analysis
│   ├── sequential.rs       # Reasoning loops
│   ├── ollama.rs           # Ollama API client
│   └── ...                 # Other modules
├── tests/                  # Integration tests
├── examples/               # Usage examples
├── benches/                # Benchmarks
├── migrations/             # Database migrations
├── schemas/                # JSON schemas
├── Cargo.toml              # Dependencies
└── mcp.toml               # MCP server metadata
```

## Testing

```bash
# Run all tests (slow, ~5-10 minutes)
cargo test

# Run specific test
cargo test test_name

# Run tests with output
cargo test -- --nocapture

# Run only library tests (faster)
cargo test --lib
```

Note: Tests create temporary database files that may accumulate. Clean with:
```bash
rm -f test_*.db
```

## Performance Expectations

On my machine (Ryzen 7, 32GB RAM):

- Memory store/query: <1ms
- Vector search (1000 vectors, linear): ~10ms
- Vector search (1000 vectors, HNSW): ~500µs after index build
- Code parsing: 10-100ms depending on file size
- IntelliTask generation: 5-30 seconds (depends on Ollama response time)

These are rough estimates. Your mileage will vary.

## Why GPL-3.0?

I chose GPL-3.0 because:
1. This is experimental software built on top of open source tools
2. If someone improves it, those improvements should be shared back
3. It's not intended for commercial use in its current state

If you need a different license for a specific use case, open an issue.

## Contributing

I'm not actively seeking contributions since this is a personal learning project, but if you want to:

1. Fork the repository
2. Create a branch for your changes
3. Submit a pull request with clear description

Please don't expect quick responses. I work on this sporadically.

## What This Is Not

- **Not production-ready**: Don't use this for anything important without thorough testing
- **Not a vector database**: It's just linear scan with optional HNSW
- **Not a knowledge graph**: Task relationships are simple parent-child, not a proper graph
- **Not an AI agent**: It's tools that an AI can use, not an autonomous agent
- **Not feature-complete**: Many planned features are stubs or incomplete

## Future Plans

Honestly, I don't have a roadmap. I add features when I need them for my workflow. Some things I might work on:

- Better embeddings (actual transformer models)
- Proper HNSW integration (batch operations, incremental updates)
- Graph relationships for tasks
- Better error handling
- More documentation
- Performance optimizations

Or I might abandon this entirely for something else. That's the nature of experimental projects.

## License

GPL-3.0. See LICENSE file for details.

## Acknowledgments

This project uses:
- [rmcp](https://github.com/anthropics/rmcp) - MCP protocol implementation
- [rusqlite](https://github.com/rusqlite/rusqlite) - SQLite bindings
- [instant-distance](https://github.com/djc/instant-distance) - HNSW implementation
- [tree-sitter](https://tree-sitter.github.io/) - Code parsing
- [fastembed](https://github.com/Anush008/fastembed-rs) - Text embeddings
- [tokio](https://tokio.rs/) - Async runtime
- And many other Rust crates

Built with significant assistance from Claude Code (Anthropic's AI coding assistant). The code quality reflects iterative AI-assisted development, for better or worse.
