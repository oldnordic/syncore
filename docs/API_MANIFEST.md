# SynCore MCP API Manifest v1.4

**Last Updated**: 2025-11-24
**API Version**: 1.4.0
**Migration Status**: APEX v1.4 SPEC - Tool Suite Consolidation

## Overview

SynCore provides 65 MCP tools organized into 5 unified suites. This manifest documents all available tools, their suite mappings, deprecation status, and migration path.

## Suite Architecture

### Design Principles
1. **Single Source of Truth**: Suites implement all functionality, legacy endpoints are thin routing wrappers
2. **Unified Interface**: All suites use `SuiteResult` response format with success/error handling
3. **Command-Based**: Suites use string commands for routing (e.g., "store", "query", "search")
4. **Zero Breaking Changes**: Legacy tools route through suites, maintaining backward compatibility

### The 5 Core Suites

1. **memory_suite** - Key-value storage, vector search, sequential reasoning, tasks, agents
2. **code_suite** - Code indexing, semantic search, parsing, analysis
3. **graph_suite** - Neo4j graph operations, RAG graph queries
4. **mapping_suite** - Application structure mapping, dependency analysis, change tracking
5. **debug_suite** - Project analysis, diagnostics, logs, refactoring suggestions

---

## 1. Memory Suite (22 tools)

**Suite Command**: `memory_suite`
**Purpose**: Persistent storage, vector embeddings, reasoning, task management, agent communication

### Exact Parity Tools (4) ✅

| Legacy Tool | Suite Command | Status | Description |
|------------|---------------|---------|-------------|
| `memory_store` | `store` | ✅ DEPRECATED | Store key-value pair in persistent memory |
| `memory_query` | `query` | ✅ DEPRECATED | Query value by key from memory |
| `vector_insert` | `vector_insert` | ✅ DEPRECATED | Insert text into vector index for semantic search |
| `vector_search` | `vector_search` | ✅ DEPRECATED | Semantic vector search with cosine similarity |

### Missing Tools (18) ⚠️

| Legacy Tool | Suite Command | Status | Description |
|------------|---------------|---------|-------------|
| `sequential_record` | `sequential_record` | ⚠️ NOT MIGRATED | Record reasoning step in sequential thinking chain |
| `sequential_get` | `sequential_get` | ⚠️ NOT MIGRATED | Get all reasoning steps for a task |
| `sequential_search` | `sequential_search` | ⚠️ NOT MIGRATED | Search reasoning steps by semantic content |
| `sequential_cycle` | `sequential_cycle` | ⚠️ NOT MIGRATED | Run sequential thinking cycles with LLM |
| `task_create` | `task_create` | ⚠️ NOT MIGRATED | Create new task with goal and priority |
| `intellitask_generate` | `intellitask_generate` | ⚠️ NOT MIGRATED | AI-powered PRD → task breakdown |
| `intellitask_subtasks` | `intellitask_subtasks` | ⚠️ NOT MIGRATED | Generate subtasks for parent task |
| `intellitask_prioritize` | `intellitask_prioritize` | ⚠️ NOT MIGRATED | AI-powered task prioritization |
| `intellitask_next` | `intellitask_next` | ⚠️ NOT MIGRATED | Suggest next task to work on |
| `intellitask_save` | `intellitask_save` | ⚠️ NOT MIGRATED | Save task breakdown to database |
| `intellitask_get` | `intellitask_get` | ⚠️ NOT MIGRATED | Get task by ID |
| `intellitask_list` | `intellitask_list` | ⚠️ NOT MIGRATED | List tasks with filtering |
| `intellitask_update_status` | `intellitask_update_status` | ⚠️ NOT MIGRATED | Update task status (open/in_progress/done) |
| `intellitask_next_ready` | `intellitask_next_ready` | ⚠️ NOT MIGRATED | Get next ready task (dependencies satisfied) |
| `intellitask_get_subtasks` | `intellitask_get_subtasks` | ⚠️ NOT MIGRATED | Get subtasks for parent |
| `intellitask_subtask_stats` | `intellitask_subtask_stats` | ⚠️ NOT MIGRATED | Get subtask statistics |
| `intellitask_task_statistics` | `intellitask_task_statistics` | ⚠️ NOT MIGRATED | Overall task statistics |
| `intellitask_prd_statistics` | `intellitask_prd_statistics` | ⚠️ NOT MIGRATED | Statistics for specific PRD |

### Agent Communication Tools (8) ⚠️

| Legacy Tool | Suite Command | Status | Description |
|------------|---------------|---------|-------------|
| `agent_send` | `agent_send` | ⚠️ NOT MIGRATED | Send message to another agent |
| `agent_recv` | `agent_recv` | ⚠️ NOT MIGRATED | Receive pending messages |
| `agent_poll` | `agent_poll` | ⚠️ NOT MIGRATED | Wait for next message with timeout |
| `agent_register` | `agent_register` | ⚠️ NOT MIGRATED | Register agent with capabilities |
| `agent_list` | `agent_list` | ⚠️ NOT MIGRATED | List all registered agents |
| `agent_status` | `agent_status` | ⚠️ NOT MIGRATED | Update agent status |
| `agent_task` | `agent_task` | ⚠️ NOT MIGRATED | Send structured task envelope to agent |
| `agent_result` | `agent_result` | ⚠️ NOT MIGRATED | Submit task completion result |

---

## 2. Code Suite (8 tools)

**Suite Command**: `code_suite`
**Purpose**: Code indexing, semantic search, parsing, and analysis

### Exact Parity Tools (5) ✅

| Legacy Tool | Suite Command | Status | Description |
|------------|---------------|---------|-------------|
| `code_index` | `index` | ✅ DEPRECATED | Index single code file with tree-sitter |
| `code_search` | `search` | ✅ DEPRECATED | Semantic code search (384-dim embeddings) |
| `code_index_directory` | `index_directory` | ✅ DEPRECATED | Index directory with glob pattern |
| `parser_analyze` | `parse` | ✅ DEPRECATED | Parse code structure (functions, classes, imports) |
| `parser_search` | `grep` | ✅ DEPRECATED | Ripgrep-based pattern search with context |

### Missing Tools (3) ⚠️

| Legacy Tool | Suite Command | Status | Description |
|------------|---------------|---------|-------------|
| `document_index` | `doc_index` | ⚠️ NOT MIGRATED | Index documents from directory |
| `document_search` | `doc_search` | ⚠️ NOT MIGRATED | Semantic document search |
| `explain_function` | `explain` | ⚠️ NOT MIGRATED | Explain function with callers/callees/complexity |
| `code_graph_sync_neo4j` | `sync_neo4j` | ⚠️ NOT MIGRATED | Sync code entities to Neo4j |
| `code_graph_enrich_temporal` | `enrich_temporal` | ⚠️ NOT MIGRATED | Enrich entities with git history metadata |
| `code_graph_fusion_query` | `fusion_query` | ⚠️ NOT MIGRATED | Tri-mode fusion query (Simple/Attention/Reasoning) |

---

## 3. Graph Suite (5 tools)

**Suite Command**: `graph_suite`
**Purpose**: Neo4j graph operations, RAG graph traversal

### Exact Parity Tools (3) ✅

| Legacy Tool | Suite Command | Status | Description |
|------------|---------------|---------|-------------|
| `graph_query` | `query` | ✅ DEPRECATED | Execute Cypher read query |
| `graph_insert` | `insert` | ✅ DEPRECATED | Execute Cypher write query |
| `graph_relate` | `relate` | ✅ DEPRECATED | Create relationship between nodes |

### Missing Tools (2) ⚠️

| Legacy Tool | Suite Command | Status | Description |
|------------|---------------|---------|-------------|
| `raggraph_query` | `rag_query` | ⚠️ NOT MIGRATED | RAG query with multi-hop graph reasoning |
| `raggraph_multihop` | `rag_multihop` | ⚠️ NOT MIGRATED | Execute multi-hop graph diffusion from seeds |

---

## 4. Mapping Suite (8 tools)

**Suite Command**: `mapping_suite`
**Purpose**: Application structure mapping, dependency analysis, change tracking

### Exact Parity Tools (4) ✅

| Legacy Tool | Suite Command | Status | Description |
|------------|---------------|---------|-------------|
| `mapping_record` | `record` | ✅ DEPRECATED | Record file node in application structure |
| `mapping_get` | `get` | ✅ DEPRECATED | Get file node from structure map |
| `mapping_search` | `search` | ✅ DEPRECATED | Search files by semantic query |
| `mapping_deps` | `deps` | ✅ DEPRECATED | Get transitive dependencies for file |

### Missing Tools (4) ⚠️

| Legacy Tool | Suite Command | Status | Description |
|------------|---------------|---------|-------------|
| `application_record` | `app_record` | ⚠️ NOT MIGRATED | Record code change in application |
| `application_get` | `app_get` | ⚠️ NOT MIGRATED | Get all changes for task |
| `application_history` | `app_history` | ⚠️ NOT MIGRATED | Get change history for file |
| `application_search` | `app_search` | ⚠️ NOT MIGRATED | Search changes by semantic content |

---

## 5. Debug Suite (11 tools)

**Suite Command**: `debug_suite`
**Purpose**: Project analysis, diagnostics, logs, code quality

### Exact Parity Tools (11) ✅

| Legacy Tool | Suite Command | Status | Description |
|------------|---------------|---------|-------------|
| `logs_tail` | `logs_tail` | ✅ DEPRECATED | Get recent log entries |
| `tool_metadata_list` | `tool_metadata_list` | ✅ DEPRECATED | List MCP tool metadata (category, cost, side effects) |
| `project_file_report` | `project_file_report` | ✅ DEPRECATED | Detailed report for single source file |
| `project_module_map` | `project_module_map` | ✅ DEPRECATED | Module-level dependency map |
| `project_hotspots` | `project_hotspots` | ✅ DEPRECATED | Identify complexity hotspots |
| `project_cycles` | `project_cycles` | ✅ DEPRECATED | Detect circular dependencies |
| `project_dead_code` | `project_dead_code` | ✅ DEPRECATED | Identify potentially dead code |
| `project_unused_imports` | `project_unused_imports` | ✅ DEPRECATED | Find unused imports |
| `project_refactor_suggestions` | `project_refactor_suggestions` | ✅ DEPRECATED | Heuristic refactoring suggestions |
| `project_code_smells` | `project_code_smells` | ✅ DEPRECATED | Detect code smells and anti-patterns |
| `project_cleanup_excluded` | `project_cleanup_excluded` | ✅ DEPRECATED | Clean indexed data for excluded directories |

### Missing Tools (0) ✅
All debug suite tools have exact parity!

---

## Migration Statistics

```
Total Legacy Tools:    65
Exact Parity:          27 (41.5%) ✅
Missing:               38 (58.5%) ⚠️
Suites:                 5

Breakdown by Suite:
- memory_suite:     4/22 (18.2%)
- code_suite:       5/8  (62.5%)
- graph_suite:      3/5  (60.0%)
- mapping_suite:    4/8  (50.0%)
- debug_suite:     11/11 (100%) ✅
```

---

## Using the API

### Direct Suite Access (Recommended)

```rust
// Memory operations
mcp__syncore__memory_suite({
  "command": "store",
  "key": "project_context",
  "value": "SynCore is an MCP server..."
})

// Code operations
mcp__syncore__code_suite({
  "command": "search",
  "query": "function that handles errors",
  "limit": 10
})

// Graph operations
mcp__syncore__graph_suite({
  "command": "query",
  "cypher": "MATCH (n:CodeEntity) RETURN n LIMIT 10"
})

// Mapping operations
mcp__syncore__mapping_suite({
  "command": "deps",
  "path": "src/main.rs"
})

// Debug operations
mcp__syncore__debug_suite({
  "command": "project_hotspots",
  "limit": 20,
  "min_loc": 100
})
```

### Legacy Tool Access (Backward Compatible)

```rust
// Still works, but internally routes through suite
mcp__syncore__memory_store({
  "key": "context",
  "value": "data"
})

// Equivalent to memory_suite + store command
```

---

## Deprecation Policy

### Timeline
- **v1.4.0** (Current): Legacy tools marked DEPRECATED, route through suites
- **v1.5.0** (Future): Deprecation warnings in responses
- **v2.0.0** (Future): Legacy tools removed, suites only

### Migration Path
1. ✅ **Phase 1** (v1.4): Identify exact_parity tools, extend suites
2. ✅ **Phase 2** (v1.4): Route legacy tools through suites (zero breaking changes)
3. ⏳ **Phase 3** (v1.5): Add deprecation metadata, implement missing tools in suites
4. ⏳ **Phase 4** (v2.0): Remove legacy endpoints, suites become sole API

### Compatibility Guarantee
- **v1.x**: Full backward compatibility maintained
- **v2.0**: Breaking change - legacy tools removed
- **Semantic Versioning**: Following strict semver for API changes

---

## Tool Categories

### By Cost/Side Effects

**Read-Only (Safe)**:
- All `*_query`, `*_search`, `*_get`, `*_list` tools
- `project_*` analysis tools (file_report, hotspots, cycles, etc.)
- `logs_tail`, `tool_metadata_list`

**Write Operations (Side Effects)**:
- `memory_store`, `vector_insert`
- `graph_insert`, `graph_relate`
- `mapping_record`, `application_record`
- `code_index`, `code_index_directory`, `document_index`
- `task_create`, `intellitask_*` (creates/updates tasks)

**Expensive (LLM/Compute)**:
- `sequential_cycle` (runs LLM reasoning loops)
- `intellitask_generate`, `intellitask_prioritize` (Ollama LLM calls)
- `code_graph_fusion_query` (tri-mode reasoning)
- `raggraph_*` (graph traversal with embeddings)

---

## Future Enhancements

### Planned for v1.5
1. **Complete Suite Migration**: Implement 38 missing tools in suites
2. **Enhanced Documentation**: OpenAPI/JSON Schema for all suites
3. **Performance Metrics**: Add timing/cost metadata to SuiteResult
4. **Streaming Support**: Long-running operations (indexing, reasoning)

### Planned for v2.0
1. **Remove Legacy Endpoints**: Clean break, suites-only API
2. **GraphQL Alternative**: Explore GraphQL over MCP for complex queries
3. **Batching**: Batch multiple suite commands in single call
4. **Versioned Suites**: `/v2/memory_suite` for future-proof evolution

---

## LangGraph/LangChain Equivalence

You mentioned building a Rust-based LangGraph equivalent - here's the mapping:

### LangGraph Components → SynCore Tools

```
LangGraph StateGraph     → memory_suite (store/query state)
LangChain VectorStore    → memory_suite (vector_insert/search)
LangChain Memory         → memory_suite (store/query + sequential_*)
LangGraph Checkpointing  → application_record/get (change tracking)
LangChain Tools          → All 65 MCP tools!
LangGraph Nodes          → intellitask_* (task graph)
LangGraph Edges          → task_links table (dependencies)
LangChain Chains         → sequential_cycle (reasoning loop)
LangGraph Router         → agent_* (message bus routing)
LangSmith Tracing        → application_* (change history)
```

### Building a Rust LangGraph

```rust
// 1. Define agent state (memory_suite)
store("agent_state", json!({"step": 1, "context": "..."}));

// 2. Create task graph (intellitask)
let tasks = intellitask_generate(prd_content);
let subtasks = intellitask_subtasks(parent_task);

// 3. Execute reasoning (sequential)
let result = sequential_cycle(max_cycles: 10);

// 4. Vector search for context (memory_suite)
let docs = vector_search("relevant context", limit: 5);

// 5. Track execution (application_record)
application_record({
  file_path: "src/agent.rs",
  change_type: "state_transition",
  description: "Agent moved from think → act"
});

// 6. Multi-agent orchestration (agent_*)
agent_register("planner", ["planning", "decomposition"]);
agent_send("executor", "task_123");
let messages = agent_recv("planner");

// 7. Graph traversal (graph_suite)
graph_query("MATCH (a:Agent)-[:DEPENDS_ON]->(b:Agent) RETURN a, b");
```

You have all the primitives for state management, memory, task graphs, reasoning loops, and multi-agent systems!

---

## API Contract

### SuiteResult Format

All suite commands return this unified format:

```json
{
  "success": true,
  "command": "store",
  "data": { /* command-specific result */ },
  "error": null  // populated on failure
}
```

### Error Handling

```json
{
  "success": false,
  "command": "query",
  "data": {},
  "error": "Key 'missing_key' not found in memory"
}
```

### Legacy Tool Envelope

Legacy tools wrap SuiteResult in MCP format:

```json
{
  "ok": true,
  "data": { /* SuiteResult.data */ }
}
```

---

## Conclusion

SynCore v1.4 represents a major architectural shift to unified tool suites while maintaining 100% backward compatibility. The 5 suites provide a clean, composable API for building sophisticated AI systems - from simple key-value storage to complex multi-agent reasoning graphs.

**Migration Progress**: 41.5% complete (27/65 tools)
**Breaking Changes**: None (v1.x maintains full compatibility)
**Next Steps**: Phase D documentation, implement remaining 38 tools in suites

---

*For implementation details, see:*
- `src/mcp_tools/memory_suite.rs`
- `src/mcp_tools/code_suite.rs`
- `src/mcp_tools/graph_suite.rs`
- `src/mcp_tools/mapping_suite.rs`
- `src/mcp_tools/debug_suite.rs`
- `tests/tool_suite_mapping.json`
