# SynCore Changelog

## [Unreleased] - December 2025

### Added
- **Project Cleanup**: Comprehensive project structure cleanup
  - Moved internal documentation to `research/internal_docs/`
  - Consolidated test files in `tests/` directory
  - Updated `.gitignore` for proper exclusion rules
- **Honest Documentation**: Rewrote README.md with accurate, transparent information
- **Tool Assessment**: Comprehensive testing and evaluation of all MCP tools
- **Updated GitIgnore**: Enhanced to properly exclude binaries, databases, and internal docs

### Changed
- **README.md**: Complete rewrite with honest assessment of capabilities
- **Project Structure**: Cleaner organization with proper separation of concerns
- **Documentation**: Moved internal status reports to research folder

### Fixed
- **File Organization**: Proper placement of test files and documentation
- **Repository Hygiene**: Better exclusion patterns for generated files

---

## [v0.2.0] - November 2025

### Added
- **Debug Suite**: Project analysis tools
  - Hotspot detection for complex files
  - Dead code identification
  - Circular dependency analysis
  - Refactoring suggestions
  - Module dependency mapping
- **Code Suite Enhancements**:
  - Incremental indexing with SHA256 change detection
  - Tri-mode RAG queries (simple/attention/reasoning)
  - Function explanation with complexity metrics
  - Temporal metadata enrichment
- **Graph Suite**: Neo4j integration and synchronization
- **Memory Suite**: Expanded to 42 commands including AI task management

### Changed
- **Vector Search**: Switched from HNSW to linear scan (more reliable)
- **Graph Backend**: SQLiteGraph as default, Neo4j as optional
- **Performance**: Optimized memory usage and query latency

### Fixed
- **Configuration**: Precedence system (file > env > defaults)
- **Indexing**: Idempotent batch operations
- **Memory Storage**: Dual-layer architecture (SQLite + Sled)

---

## [v0.1.0] - October 2025

### Added
- **Initial Release**: Core MCP server functionality
- **Memory Suite**: Basic key-value storage with semantic search
- **Code Suite**: Tree-sitter parsing for 6 languages
- **Vector Search**: 384-dimension embeddings with HNSW indexing
- **Graph Storage**: SQLite-based knowledge graph
- **Task Management**: Parent-child relationships
- **Agent Coordination**: Message bus for multi-agent workflows

### Features
- **65+ MCP Tools**: Organized into 5 suites
- **Multi-Modal Storage**: SQLite, Sled, Neo4j, HNSW
- **AI Integration**: IntelliTask for automated planning
- **Sequential Reasoning**: Multi-step thought recording

### Known Limitations
- Single-node architecture only
- No authentication or security features
- Experimental AI features requiring external LLM
- Linear vector search scalability concerns

---

## Technical Debt & Future Plans

### Immediate Priorities
- [ ] Replace linear vector search with proper HNSW implementation
- [ ] Complete Neo4j entity population validation
- [ ] Add streaming support for large result sets
- [ ] Implement authentication and security features

### Architecture Improvements
- [ ] Distributed mode support
- [ ] Real Graph-BERT integration (replace placeholder)
- [ ] Enhanced caching strategies
- [ ] Performance monitoring and metrics

### Integration Enhancements
- [ ] External LLM backend abstraction
- [ ] Plugin system for custom tools
- [ ] REST API alongside MCP protocol
- [ ] CLI tool improvements

---

## Version Philosophy

- **Major versions (x.0.0)**: Breaking changes or major architecture shifts
- **Minor versions (0.x.0)**: New features, significant improvements
- **Patch versions (0.0.x)**: Bug fixes, documentation updates

**Development Branch**: `development` - All active development happens here
**Main Branch**: `main` - Stable releases only

---

## Testing Status

All changes are tested with:
- Unit tests (`cargo test`)
- Integration tests (MCP protocol compliance)
- Real-world usage with Claude Code
- Performance benchmarking on target hardware

## Support

- **Issues**: Report via GitHub Issues
- **Discussions**: Use GitHub Discussions for questions
- **Documentation**: See `MANUAL.md` for detailed usage
- **Architecture**: See `ARCHITECTURE.md` for technical details