# SynCore API Deprecation Policy

**Version**: 1.4.0
**Last Updated**: 2025-11-24
**Status**: ACTIVE - Tool Suite Migration in Progress

## Overview

This document outlines SynCore's deprecation policy for the transition from 65 legacy individual tools to 5 unified tool suites (memory_suite, code_suite, graph_suite, mapping_suite, debug_suite).

## Deprecation Philosophy

### Principles
1. **No Surprise Breaking Changes**: Major version bumps signal breaking changes
2. **Long Transition Period**: Minimum 2 minor versions before removal
3. **Clear Migration Path**: Document exact replacement for every deprecated tool
4. **Backward Compatibility**: v1.x maintains full compatibility
5. **Proactive Communication**: Deprecation metadata in responses (v1.5+)

### Semantic Versioning

SynCore follows strict semantic versioning (semver):

```
MAJOR.MINOR.PATCH
  1  .  4  .  0

MAJOR: Breaking changes (API removal, behavior changes)
MINOR: New features, deprecations (backward compatible)
PATCH: Bug fixes, performance improvements
```

---

## Deprecation Timeline

### ✅ v1.4.0 (Current) - "Silent Deprecation"
**Released**: 2025-11-24
**Status**: Production Ready

**Changes**:
- ✅ Created 5 unified tool suites
- ✅ Routed 27 legacy tools through suites (transparent to users)
- ✅ Added suite command routing in `executor_real.rs`
- ✅ All tests pass, zero breaking changes

**User Impact**: **NONE**
- All 65 tools work exactly as before
- No warnings, no errors
- Internal architecture changed, external API unchanged

**Migration Action**: None required, but recommended:
```rust
// Old (still works)
mcp__syncore__memory_store({"key": "x", "value": "y"})

// New (recommended)
mcp__syncore__memory_suite({"command": "store", "key": "x", "value": "y"})
```

---

### ⏳ v1.5.0 (Planned) - "Soft Deprecation"
**Target**: Q2 2025
**Status**: Planning Phase

**Changes**:
- Add deprecation metadata to legacy tool responses
- Implement 38 missing tools in suites (100% parity)
- Add suite documentation to MCP tool descriptions
- Add `X-Deprecation-Warning` header to responses

**User Impact**: **LOW**
- Deprecation warnings in responses (non-breaking)
- All tools continue to work
- Logs/console show migration recommendations

**Example Response**:
```json
{
  "ok": true,
  "data": {"key": "x", "value": "y"},
  "_deprecation": {
    "deprecated": true,
    "since": "1.5.0",
    "removal_version": "2.0.0",
    "replacement": {
      "tool": "memory_suite",
      "command": "store",
      "example": {
        "command": "store",
        "key": "x",
        "value": "y"
      }
    },
    "message": "memory_store is deprecated. Use memory_suite with command='store' instead."
  }
}
```

**Migration Action**: Update client code to use suites:
```rust
// Replace all calls to legacy tools with suite equivalents
// See API_MANIFEST.md for exact mappings
```

---

### ⏳ v1.6.0 (Planned) - "Hard Deprecation"
**Target**: Q3 2025
**Status**: Planning Phase

**Changes**:
- Increase warning severity (logged at WARN level)
- Add deprecation counters to metrics endpoint
- Client libraries emit deprecation warnings
- Documentation clearly marks legacy tools as deprecated

**User Impact**: **MEDIUM**
- Warnings become more visible
- Monitoring dashboards show deprecated tool usage
- Still fully functional, but strongly encouraged to migrate

**Metrics Example**:
```
syncore_deprecated_tool_calls_total{tool="memory_store"} 1543
syncore_suite_tool_calls_total{suite="memory_suite"} 8721
```

**Migration Action**: **Required** before v2.0
- Audit all tool calls in your codebase
- Migrate to suite-based calls
- Test thoroughly

---

### 🔴 v2.0.0 (Planned) - "Breaking Change"
**Target**: Q4 2025
**Status**: Planning Phase

**Changes**:
- **BREAKING**: Remove all 65 legacy tool endpoints
- Only 5 suites remain: `memory_suite`, `code_suite`, `graph_suite`, `mapping_suite`, `debug_suite`
- Clean codebase, improved maintainability
- Performance improvements from consolidated architecture

**User Impact**: **HIGH** - Breaking Changes
- Legacy tool calls will fail with "Tool not found" errors
- Must migrate to suite-based API

**Migration Action**: **MANDATORY**
```rust
// ❌ This will fail in v2.0
mcp__syncore__memory_store({"key": "x", "value": "y"})
// Error: Tool 'memory_store' not found

// ✅ This is the only supported API
mcp__syncore__memory_suite({"command": "store", "key": "x", "value": "y"})
```

---

## Deprecated Tools Registry

### Currently Deprecated (v1.4.0)

**Total**: 27 tools have exact parity with suites

#### Memory Suite (4)
- `memory_store` → `memory_suite` + `command: "store"`
- `memory_query` → `memory_suite` + `command: "query"`
- `vector_insert` → `memory_suite` + `command: "vector_insert"`
- `vector_search` → `memory_suite` + `command: "vector_search"`

#### Code Suite (5)
- `code_index` → `code_suite` + `command: "index"`
- `code_search` → `code_suite` + `command: "search"`
- `code_index_directory` → `code_suite` + `command: "index_directory"`
- `parser_analyze` → `code_suite` + `command: "parse"`
- `parser_search` → `code_suite` + `command: "grep"`

#### Graph Suite (3)
- `graph_query` → `graph_suite` + `command: "query"`
- `graph_insert` → `graph_suite` + `command: "insert"`
- `graph_relate` → `graph_suite` + `command: "relate"`

#### Mapping Suite (4)
- `mapping_record` → `mapping_suite` + `command: "record"`
- `mapping_get` → `mapping_suite` + `command: "get"`
- `mapping_search` → `mapping_suite` + `command: "search"`
- `mapping_deps` → `mapping_suite` + `command: "deps"`

#### Debug Suite (11)
- `logs_tail` → `debug_suite` + `command: "logs_tail"`
- `tool_metadata_list` → `debug_suite` + `command: "tool_metadata_list"`
- `project_file_report` → `debug_suite` + `command: "project_file_report"`
- `project_module_map` → `debug_suite` + `command: "project_module_map"`
- `project_hotspots` → `debug_suite` + `command: "project_hotspots"`
- `project_cycles` → `debug_suite` + `command: "project_cycles"`
- `project_dead_code` → `debug_suite` + `command: "project_dead_code"`
- `project_unused_imports` → `debug_suite` + `command: "project_unused_imports"`
- `project_refactor_suggestions` → `debug_suite` + `command: "project_refactor_suggestions"`
- `project_code_smells` → `debug_suite` + `command: "project_code_smells"`
- `project_cleanup_excluded` → `debug_suite` + `command: "project_cleanup_excluded"`

### Not Yet Deprecated (38 tools)

These tools will be deprecated in v1.5.0 once suite implementations are complete:

- 18 IntelliTask/Sequential/Agent tools (memory_suite)
- 6 Code/Document/Explain tools (code_suite)
- 2 RAGGraph tools (graph_suite)
- 4 Application mapping tools (mapping_suite)

---

## Migration Guide

### Step 1: Inventory (v1.4 → v1.5)

Scan your codebase for deprecated tool usage:

```bash
# Search for legacy tool calls
rg "mcp__syncore__(memory_store|memory_query|vector_insert|vector_search)" .
rg "mcp__syncore__(code_index|code_search|code_index_directory)" .
rg "mcp__syncore__(graph_query|graph_insert|graph_relate)" .
rg "mcp__syncore__(mapping_record|mapping_get|mapping_search|mapping_deps)" .
rg "mcp__syncore__(logs_tail|tool_metadata_list|project_.*)" .
```

### Step 2: Replace (v1.5 → v1.6)

Use suite-based API:

```rust
// Before
let result = mcp__syncore__memory_store({
    "key": "project_context",
    "value": "SynCore is awesome"
});

// After
let result = mcp__syncore__memory_suite({
    "command": "store",
    "key": "project_context",
    "value": "SynCore is awesome"
});
```

### Step 3: Test (v1.6)

Run your test suite to ensure suite-based calls work:

```bash
cargo test
# All tests should pass
```

### Step 4: Monitor (v1.6)

Check metrics for deprecated tool usage:

```bash
curl http://localhost:9090/metrics | grep syncore_deprecated_tool_calls
```

### Step 5: Upgrade (v2.0)

Once all deprecated calls are migrated, upgrade to v2.0:

```toml
# Cargo.toml
[dependencies]
syncore = "2.0"  # Clean architecture, suites only
```

---

## Automated Migration Tool

We provide a migration script for common patterns:

```bash
# Install syncore-migrate CLI
cargo install syncore-migrate

# Dry run (preview changes)
syncore-migrate --dry-run ./src

# Apply migrations
syncore-migrate ./src

# Output:
# ✓ Migrated 127 memory_store calls → memory_suite
# ✓ Migrated 43 code_index calls → code_suite
# ✓ Migrated 89 graph_query calls → graph_suite
# Total: 259 migrations applied
```

---

## Exceptions & Special Cases

### Long-Running Operations

Some tools may have different behavior in suites:

```rust
// Legacy: Synchronous, blocks until complete
code_index_directory(directory: "/large/codebase")

// Suite: May support streaming/chunking in v2.0
code_suite({
  command: "index_directory",
  directory: "/large/codebase",
  stream: true  // Future feature
})
```

### Batch Operations

v2.0 will support batching:

```rust
// v2.0: Batch multiple suite commands
memory_suite({
  batch: true,
  commands: [
    {command: "store", key: "a", value: "1"},
    {command: "store", key: "b", value: "2"},
    {command: "query", key: "a"}
  ]
})
// Returns array of results
```

---

## Support & Communication

### Staying Informed

- **Release Notes**: Check GitHub releases for deprecation announcements
- **Migration Docs**: Updated in `docs/MIGRATION_GUIDE.md` (v1.5+)
- **API Manifest**: `docs/API_MANIFEST.md` always current

### Getting Help

- **GitHub Issues**: Report migration issues or questions
- **Documentation**: See `docs/` directory for guides
- **Examples**: `examples/suite_migration/` (v1.5+)

### Enterprise Support

Contact for custom migration assistance:
- Extended deprecation timeline for enterprise deployments
- Custom tooling for large codebases
- Migration consulting

---

## Rollback Plan

If issues arise during migration:

### v1.5 → v1.4 Rollback
```toml
[dependencies]
syncore = "=1.4.0"  # Pin to stable version
```

### v2.0 → v1.6 Downgrade
Not recommended, but possible:
1. Restore legacy tool calls from git history
2. Downgrade to v1.6 (last version with legacy support)
3. Test thoroughly before production deployment

---

## Metrics & Tracking

### Deprecation Dashboard (v1.5+)

Prometheus metrics available at `http://localhost:9090/metrics`:

```
# Total calls to deprecated tools
syncore_deprecated_tool_calls_total{tool="memory_store"} 1543

# Calls to suite-based API
syncore_suite_tool_calls_total{suite="memory_suite",command="store"} 8721

# Migration percentage (auto-calculated)
syncore_migration_percentage{suite="memory_suite"} 85.0
```

### Migration Progress

Track your progress:

```bash
# Check migration status
curl http://localhost:9090/api/migration-status

# Response:
{
  "version": "1.5.0",
  "total_tools": 65,
  "deprecated": 27,
  "migrated_calls_7d": 8721,
  "legacy_calls_7d": 1543,
  "migration_percentage": 85.0,
  "estimated_completion": "2025-09-15"
}
```

---

## FAQ

### Q: Why deprecate individual tools?
**A**: Suite-based architecture is more maintainable, composable, and extensible. It reduces code duplication and provides a cleaner API surface.

### Q: Can I keep using legacy tools forever?
**A**: Only until v2.0. Legacy tools will be removed in v2.0 (Q4 2025). You have 3 quarters to migrate.

### Q: Will there be breaking changes before v2.0?
**A**: No. All v1.x versions maintain backward compatibility. Only v2.0 is a breaking change.

### Q: What if my tool isn't in a suite yet?
**A**: 38 tools don't have suite equivalents yet. They'll be implemented in v1.5 before any deprecation.

### Q: How do I report migration issues?
**A**: Open a GitHub issue with the `migration` label. We'll help!

### Q: Can I use both APIs in v1.x?
**A**: Yes! Mix and match freely. Legacy tools and suites coexist until v2.0.

### Q: What about client libraries?
**A**: Client libraries will be updated to use suites by default in v1.5, with legacy wrappers for compatibility.

---

## Commitment

SynCore maintainers commit to:

1. **No surprises**: Breaking changes only in major versions
2. **Long runway**: Minimum 6 months notice before removal
3. **Clear docs**: Every deprecated tool has documented replacement
4. **Smooth migration**: Automated tools and guidance provided
5. **Backward compat**: v1.x remains stable and supported

**Thank you for using SynCore!** We believe this migration will result in a better, more maintainable API that serves you well for years to come.

---

*Last Updated*: 2025-11-24
*Version*: 1.4.0
*Status*: Active
