# SynCore Documentation

This directory contains comprehensive documentation for the SynCore MCP server.

## Core Documentation

### API & Migration Docs (v1.4)
- **[API_MANIFEST.md](./API_MANIFEST.md)** - Complete catalog of all 65 MCP tools, 5 unified suites, usage examples, and LangGraph/LangChain equivalence mapping
- **[DEPRECATION_POLICY.md](./DEPRECATION_POLICY.md)** - Deprecation timeline, migration guide, and backward compatibility guarantees
- **[APEX_V1.4_COMPLETION_SUMMARY.md](./APEX_V1.4_COMPLETION_SUMMARY.md)** - Implementation details for APEX v1.4 tool suite migration

## Quick Links

### For Users
- **Getting Started**: See `API_MANIFEST.md` → "Using the API" section
- **Tool Reference**: See `API_MANIFEST.md` → Suite-specific sections (Memory, Code, Graph, Mapping, Debug)
- **Migration Help**: See `DEPRECATION_POLICY.md` → "Migration Guide" section

### For Developers
- **Architecture**: See `APEX_V1.4_COMPLETION_SUMMARY.md` → "Architecture Impact" section
- **Implementation Details**: See `APEX_V1.4_COMPLETION_SUMMARY.md` → Phase C section
- **Testing**: See `tests/tool_suite_mapping_tests.rs` for validation suite

### For Maintainers
- **Migration Status**: See `tests/tool_suite_mapping.json` for current progress (41.5%)
- **Deprecation Timeline**: See `DEPRECATION_POLICY.md` → "Deprecation Timeline" section
- **Future Work**: See `APEX_V1.4_COMPLETION_SUMMARY.md` → "Future Work (v1.5)" section

## Document Overview

### API_MANIFEST.md (16KB, 550 lines)
Comprehensive API reference documenting:
- All 65 MCP tools organized by suite
- Complete command reference for 5 suites
- Migration statistics and progress tracking
- LangGraph/LangChain component equivalence
- Tool categories (read-only, write, expensive)
- Future enhancement roadmap
- Usage examples for all suites

**Key Sections**:
1. Suite Architecture (design principles)
2. Memory Suite (22 tools)
3. Code Suite (8 tools)
4. Graph Suite (5 tools)
5. Mapping Suite (8 tools)
6. Debug Suite (11 tools)
7. Migration Statistics
8. Using the API
9. LangGraph/LangChain Equivalence

### DEPRECATION_POLICY.md (13KB, 450 lines)
Official deprecation policy including:
- 4-stage deprecation timeline (v1.4 → v2.0)
- Semantic versioning commitment
- Complete migration guide
- Automated migration tooling
- Metrics & progress tracking
- FAQ section
- Rollback procedures

**Key Sections**:
1. Deprecation Philosophy
2. Timeline (v1.4 Silent → v1.5 Soft → v1.6 Hard → v2.0 Breaking)
3. Deprecated Tools Registry (27 tools)
4. Migration Guide (5 steps)
5. Automated Migration Tool
6. Support & Communication
7. FAQ

### APEX_V1.4_COMPLETION_SUMMARY.md (16KB, 600 lines)
Implementation completion report with:
- Phase-by-phase implementation details
- Test results (11/11 passing)
- Architecture impact analysis
- Migration progress dashboard
- Code changes summary
- Lessons learned
- Future work roadmap

**Key Sections**:
1. Executive Summary
2. Phase A: Inventory + Parity Map
3. Phase B: Extend Suites
4. Phase C: Route Legacy Tools
5. Phase D: Documentation & Policy
6. Test Results & Metrics
7. LangGraph/LangChain Equivalence
8. Migration Progress Dashboard

## Migration Progress

**Current Status (v1.4.0)**:
- ✅ 27/65 tools (41.5%) migrated to suites
- ✅ 11/11 validation tests passing
- ✅ Zero breaking changes
- ✅ Complete documentation

**Suite Breakdown**:
```
debug_suite:    11/11 (100%) ✅ COMPLETE
code_suite:      5/8  (62.5%)
graph_suite:     3/5  (60.0%)
mapping_suite:   4/8  (50.0%)
memory_suite:    4/22 (18.2%)
```

**Next Milestone (v1.5.0)**:
- Implement 38 remaining tools in suites
- Add deprecation metadata to responses
- Create automated migration CLI tool

## Building a Rust LangGraph

As noted in `API_MANIFEST.md`, SynCore provides all the primitives for building a Rust-based LangGraph/LangChain equivalent:

**Available Components**:
- State management (memory_suite)
- Vector search (memory_suite)
- Task graphs (intellitask_*)
- Reasoning loops (sequential_cycle)
- Multi-agent messaging (agent_*)
- Graph traversal (graph_suite)
- Change tracking (application_*)

See `API_MANIFEST.md` → "LangGraph/LangChain Equivalence" section for detailed mapping.

## Version History

- **v1.4.0** (2025-11-24): APEX tool suite migration, 27 tools routed, comprehensive docs
- **v1.3.x**: Legacy individual tool architecture
- **v2.0.0** (Planned Q4 2025): Breaking change - suite-only API

## Contributing

When adding new tools or suites:
1. Implement in appropriate suite first
2. Add mapping to `tests/tool_suite_mapping.json`
3. Add test case to `tests/tool_suite_mapping_tests.rs`
4. Update `docs/API_MANIFEST.md` with tool documentation
5. Update deprecation timeline if applicable

## Support

- **GitHub Issues**: Report bugs or request features
- **Documentation**: Check these docs first
- **Examples**: See `examples/` directory (coming in v1.5)

---

*Last Updated: 2025-11-24*
*Documentation Version: 1.4.0*
