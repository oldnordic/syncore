# SynCore MCP Tools - Complete Usage Manual

**Version**: R5.0
**Target Audience**: Humans and LLMs
**Last Updated**: 2025-11-21

---

## Table of Contents

1. [Overview](#overview)
2. [Memory Tools](#memory-tools)
3. [Vector Search Tools](#vector-search-tools)
4. [Task Management Tools](#task-management-tools)
5. [Code Analysis Tools](#code-analysis-tools)
6. [Code Graph Tools](#code-graph-tools)
7. [Sequential Reasoning Tools](#sequential-reasoning-tools)
8. [IntelliTask Tools](#intellitask-tools)
9. [Application Tracking Tools](#application-tracking-tools)
10. [Document Management Tools](#document-management-tools)
11. [Graph Database Tools](#graph-database-tools)
12. [Agent Communication Tools](#agent-communication-tools)
13. [Logging Tools](#logging-tools)
14. [Tool Metadata](#tool-metadata)
15. [Best Practices](#best-practices)
16. [Common Workflows](#common-workflows)
17. [Error Handling](#error-handling)

---

## Overview

SynCore is an AI-native Model Context Protocol (MCP) server providing intelligent memory, task management, vector search, and reasoning capabilities. All tools are accessible via MCP protocol over stdio transport.

**Key Features:**
- Hybrid storage: SQLite (persistent) + Sled cache (fast)
- Vector search with HNSW indexing (384-dimensional embeddings)
- Multi-modal code analysis (Rust, JS, Python, JSON, TOML, Bash)
- Sequential reasoning with LLM integration
- RAGGraph: Tri-modal fusion (Vector + Graph + Symbolic)
- Neo4j graph database integration
- Real-time logging and metrics

**Connection**: stdio transport (designed for Claude Desktop, Cline, etc.)

---

## Memory Tools

### `memory_store`

**Purpose**: Store key-value pairs in hybrid SQLite + Sled cache storage.

**Parameters**:
```json
{
  "key": "string (required)",
  "value": "string (required)",
  "dry_run": "boolean (optional, default: false)"
}
```

**Usage Examples**:

**Human Usage**:
```json
// Store project context
{
  "key": "project_context",
  "value": "SynCore is a 7866 LOC Rust MCP server with RAGGraph integration"
}

// Store current task
{
  "key": "current_task",
  "value": "Implementing Phase R5.0 Planning Engine"
}

// Store implementation notes
{
  "key": "phase_5_notes",
  "value": "Plan engine complete. Executor uses real tools. All tests passing."
}
```

**LLM Usage**:
```javascript
// At session start - load context
mcp__syncore__memory_query("project_context")
mcp__syncore__memory_query("current_task")

// During work - store progress
mcp__syncore__memory_store("task_status", "Phase R5.0 complete - 8/8 tests passing")
mcp__syncore__memory_store("files_modified", "plan_engine.rs, plan_executor.rs, orchestrator.rs")

// At session end - store next steps
mcp__syncore__memory_store("next_steps", "Index entire src/ directory with code_index_directory")
```

**Returns**:
```json
{
  "status": "success",
  "key": "project_context",
  "message": "Stored successfully"
}
```

**Storage Details**:
- Primary: SQLite database (persistent across restarts)
- Cache: Sled B-tree (fast in-memory access)
- Conflict: Last write wins
- Index: Unique on `k` column

---

### `memory_query`

**Purpose**: Retrieve value by key from memory storage.

**Parameters**:
```json
{
  "key": "string (required)"
}
```

**Usage Examples**:

**Human Usage**:
```json
// Query stored context
{"key": "project_context"}

// Check task status
{"key": "current_task"}

// Retrieve configuration
{"key": "embedding_model"}
```

**LLM Usage**:
```javascript
// Session initialization pattern
const context = mcp__syncore__memory_query("project_context")
const task = mcp__syncore__memory_query("current_task")
const blockers = mcp__syncore__memory_query("blockers")

// Check if key exists before using
const config = mcp__syncore__memory_query("user_config")
if (config.value !== null) {
  // Use configuration
}
```

**Returns**:
```json
{
  "key": "project_context",
  "value": "SynCore is a 7866 LOC Rust MCP server...",
  "found": true
}

// Or if not found:
{
  "key": "nonexistent_key",
  "value": null,
  "found": false
}
```

**Best Practices**:
- Always check `found` field before using `value`
- Use descriptive keys (e.g., `phase_5_status` not `p5s`)
- Store timestamps for time-sensitive data

---

## Vector Search Tools

### `vector_insert`

**Purpose**: Insert text into vector store with semantic embeddings (384-dimensional).

**Parameters**:
```json
{
  "text": "string (required)",
  "metadata": "object (optional)",
  "dry_run": "boolean (optional, default: false)"
}
```

**Usage Examples**:

**Human Usage**:
```json
// Index documentation
{
  "text": "The Planning Engine generates minimal execution plans (3-8 steps) based on query intent, recommended patterns, and self-consistency evaluation.",
  "metadata": {
    "source": "plan_engine.rs",
    "type": "documentation",
    "phase": "R5.0"
  }
}

// Index code snippet
{
  "text": "pub fn generate_plan(query: &str, intent: &QueryIntent, selected_mode: &str, recommended_patterns: &[ReasoningPattern], consistency: &SelfConsistencyResult, bundle: &ContextBundle) -> Result<Plan>",
  "metadata": {
    "source": "plan_engine.rs",
    "type": "function_signature",
    "line": 70
  }
}
```

**LLM Usage**:
```javascript
// Index knowledge after learning
mcp__syncore__vector_insert(
  "Phase R5.0 introduced Planning Engine (plan_engine.rs) and Plan Executor (plan_executor.rs). Both modules < 300 LOC. Tests: test_generate_plan_respects_intent_and_patterns, test_execute_plan_runs_real_tools.",
  { phase: "R5.0", type: "summary" }
)

// Index error solutions
mcp__syncore__vector_insert(
  "When execute_plan shows 0 steps executed with allow_write=false, check if tool is in is_write_operation() list. Write operations: memory_store, vector_insert, code_index, file_update, task_create.",
  { type: "troubleshooting", category: "execution" }
)
```

**Returns**:
```json
{
  "status": "success",
  "id": 12345,
  "dimension": 384,
  "indexed": true
}
```

**Embedding Details**:
- Model: HuggingFace semantic embeddings
- Dimensions: 384
- Method: TF-IDF weighted averaging
- Index: HNSW (m=32, ef_construction=200)

---

### `vector_search`

**Purpose**: Semantic search across indexed vectors using cosine similarity.

**Parameters**:
```json
{
  "query": "string (required)",
  "limit": "integer (optional, default: 10, max: 100)"
}
```

**Usage Examples**:

**Human Usage**:
```json
// Search for relevant code
{
  "query": "how does plan execution handle errors",
  "limit": 5
}

// Find similar implementations
{
  "query": "pattern matching with success rate threshold",
  "limit": 10
}
```

**LLM Usage**:
```javascript
// Before implementing new feature - search for similar patterns
const similar = mcp__syncore__vector_search(
  "context bundle composition with raggraph entities",
  10
)

// Retrieve relevant documentation
const docs = mcp__syncore__vector_search(
  "self-consistency evaluation algorithm",
  5
)

// Find error handling examples
const errors = mcp__syncore__vector_search(
  "anyhow Result error propagation in async functions",
  8
)
```

**Returns**:
```json
{
  "results": [
    {
      "id": 12345,
      "text": "The Planning Engine generates minimal execution plans...",
      "score": 0.8732,
      "metadata": {
        "source": "plan_engine.rs",
        "type": "documentation"
      }
    },
    {
      "id": 12346,
      "text": "pub fn generate_plan(...) -> Result<Plan>",
      "score": 0.8456,
      "metadata": {
        "source": "plan_engine.rs",
        "type": "function_signature"
      }
    }
  ],
  "query_embedding_dim": 384,
  "total_results": 2
}
```

**Performance Notes**:
- Search time: O(n) linear scan with SIMD optimizations
- Parallel search available for large datasets
- Typical latency: <10ms for 10k vectors

---

## Task Management Tools

### `task_create`

**Purpose**: Create a new task with optional parent-child hierarchy.

**Parameters**:
```json
{
  "goal": "string (required)",
  "priority": "integer (optional, 1-5, default: 3)"
}
```

**Usage Examples**:

**Human Usage**:
```json
// Create top-level task
{
  "goal": "Implement Phase R6.0 - Evidence Provenance System",
  "priority": 1
}

// Create subtask
{
  "goal": "Design evidence chain data structure",
  "priority": 2
}
```

**LLM Usage**:
```javascript
// Create task hierarchy
const parent = mcp__syncore__task_create(
  "Optimize vector search performance",
  1
)

// Note: Subtask creation requires separate API (see intellitask_subtasks)
```

**Returns**:
```json
{
  "task_id": 42,
  "goal": "Implement Phase R6.0 - Evidence Provenance System",
  "status": "open",
  "priority": 1,
  "created_at": 1732179123
}
```

---

### `task_list`

**Purpose**: List tasks with optional filtering.

**Parameters**:
```json
{
  "status": "string (optional: 'open', 'in_progress', 'done', 'cancelled')",
  "parent_id": "integer (optional)",
  "prd_title": "string (optional)"
}
```

**Usage Examples**:

**Human Usage**:
```json
// List all open tasks
{"status": "open"}

// List completed tasks
{"status": "done"}

// List subtasks of task 42
{"parent_id": 42}
```

**LLM Usage**:
```javascript
// Get current work queue
const open_tasks = mcp__syncore__intellitask_list({ status: "open" })

// Check completed work
const done_tasks = mcp__syncore__intellitask_list({ status: "done" })

// Review specific project tasks
const project_tasks = mcp__syncore__intellitask_list({
  prd_title: "Phase R5.0 Implementation"
})
```

**Returns**:
```json
{
  "tasks": [
    {
      "id": 42,
      "goal": "Implement Phase R6.0",
      "status": "open",
      "priority": 1,
      "parent_id": null,
      "created_at": 1732179123,
      "updated_at": 1732179123
    }
  ],
  "total": 1
}
```

---

### `intellitask_next_ready`

**Purpose**: Get next task ready to work on (all dependencies satisfied).

**Parameters**: None

**Usage Examples**:

**LLM Usage**:
```javascript
// At session start - determine what to work on
const next_task = mcp__syncore__intellitask_next_ready()

if (next_task.task_id) {
  console.log(`Working on: ${next_task.goal}`)
  // Begin implementation
}
```

**Returns**:
```json
{
  "task_id": 43,
  "goal": "Design evidence chain data structure",
  "status": "open",
  "priority": 2,
  "dependencies_satisfied": true,
  "blocking_tasks": []
}

// Or if no tasks ready:
{
  "task_id": null,
  "message": "No tasks ready. 2 tasks blocked by dependencies."
}
```

---

### `intellitask_update_status`

**Purpose**: Update task status.

**Parameters**:
```json
{
  "task_id": "integer (required)",
  "status": "string (required: 'open', 'in_progress', 'done', 'cancelled')"
}
```

**Usage Examples**:

**Human Usage**:
```json
// Mark task as in progress
{
  "task_id": 43,
  "status": "in_progress"
}

// Complete task
{
  "task_id": 43,
  "status": "done"
}
```

**LLM Usage**:
```javascript
// Start working on task
mcp__syncore__intellitask_update_status(43, "in_progress")

// Complete after implementation
mcp__syncore__intellitask_update_status(43, "done")

// Cancel if blocked
mcp__syncore__intellitask_update_status(43, "cancelled")
```

**Returns**:
```json
{
  "task_id": 43,
  "status": "done",
  "updated_at": 1732179456
}
```

---

## Code Analysis Tools

### `parser_analyze`

**Purpose**: Parse source code using tree-sitter and extract structure (functions, classes, imports).

**Supported Languages**: Rust, JavaScript, TypeScript, Python, JSON, TOML, Bash

**Parameters**:
```json
{
  "file_path": "string (required, absolute path)"
}
```

**Usage Examples**:

**Human Usage**:
```json
{
  "file_path": "/home/user/project/src/main.rs"
}
```

**LLM Usage**:
```javascript
// Analyze file structure before editing
const structure = mcp__syncore__parser_analyze(
  "/home/feanor/Projects/SynCore/syncore/src/cognition/plan_engine.rs"
)

console.log(`Functions: ${structure.functions.length}`)
console.log(`Classes: ${structure.classes.length}`)

// Find specific function
const target_fn = structure.functions.find(f => f.name === "generate_plan")
if (target_fn) {
  console.log(`Line range: ${target_fn.line_start}-${target_fn.line_end}`)
}
```

**Returns**:
```json
{
  "file_path": "/home/feanor/Projects/SynCore/syncore/src/cognition/plan_engine.rs",
  "language": "rust",
  "functions": [
    {
      "name": "generate_plan",
      "signature": "generate_plan(query, intent, selected_mode, recommended_patterns, consistency, bundle)",
      "line_start": 70,
      "line_end": 122,
      "docstring": "/// Generate an execution plan based on cognitive context"
    },
    {
      "name": "determine_strategy",
      "signature": "determine_strategy(intent, selected_mode, bundle)",
      "line_start": 134,
      "line_end": 155,
      "docstring": "/// Determine best strategy based on intent, mode, and context"
    }
  ],
  "classes": [
    {
      "name": "Plan",
      "line_start": 29,
      "line_end": 55
    }
  ],
  "imports": [
    "use super::context_bundle::ContextBundle",
    "use super::intent_classifier::QueryIntent"
  ],
  "total_lines": 359
}
```

**Performance**: ~1-5ms for typical files (<1000 LOC)

---

### `parser_search`

**Purpose**: Search code patterns using ripgrep with context lines.

**Parameters**:
```json
{
  "pattern": "string (required, regex pattern)",
  "path": "string (optional, default: current directory)",
  "context_lines": "integer (optional, lines before/after match)"
}
```

**Usage Examples**:

**Human Usage**:
```json
// Find all TODO comments
{
  "pattern": "TODO|FIXME",
  "context_lines": 2
}

// Find function definitions
{
  "pattern": "fn\\s+\\w+",
  "path": "src/cognition"
}

// Find error handling
{
  "pattern": "\\.ok\\(\\)|\\.unwrap\\(\\)",
  "context_lines": 3
}
```

**LLM Usage**:
```javascript
// Before implementing - check existing patterns
const existing = mcp__syncore__parser_search(
  "pub fn generate_plan",
  "/home/feanor/Projects/SynCore/syncore/src"
)

// Find similar error handling
const error_patterns = mcp__syncore__parser_search(
  "anyhow::Result|Result<",
  "/home/feanor/Projects/SynCore/syncore/src/cognition",
  5
)

// Verify no regressions
const test_refs = mcp__syncore__parser_search(
  "test_generate_plan",
  "/home/feanor/Projects/SynCore/syncore/tests"
)
```

**Returns**:
```json
{
  "pattern": "pub fn generate_plan",
  "matches": [
    {
      "file": "src/cognition/plan_engine.rs",
      "line_number": 70,
      "line": "pub fn generate_plan(",
      "context_before": [
        "/// # Returns",
        "/// A minimal execution plan (3-8 steps) using real SynCore tools"
      ],
      "context_after": [
        "    query: &str,",
        "    intent: &QueryIntent,"
      ]
    }
  ],
  "total_matches": 1,
  "files_searched": 156
}
```

---

## Code Graph Tools

### `code_index`

**Purpose**: Index a source code file into the CodeGraph database (entities, embeddings, relationships).

**Parameters**:
```json
{
  "file_path": "string (required, absolute path)"
}
```

**Usage Examples**:

**Human Usage**:
```json
{
  "file_path": "/home/user/project/src/main.rs"
}
```

**LLM Usage**:
```javascript
// Index newly created files
mcp__syncore__code_index(
  "/home/feanor/Projects/SynCore/syncore/src/cognition/plan_engine.rs"
)

// Re-index after modifications
mcp__syncore__code_index(
  "/home/feanor/Projects/SynCore/syncore/src/cognition/orchestrator.rs"
)
```

**Returns**:
```json
{
  "file_path": "/home/feanor/Projects/SynCore/syncore/src/cognition/plan_engine.rs",
  "entities_indexed": 16,
  "functions": 8,
  "classes": 2,
  "total_lines": 358,
  "language": "rust",
  "database": "syncore_code_graph.db"
}
```

**Storage**:
- SQLite: Entity metadata (name, signature, line ranges)
- Vector Store: Semantic embeddings (384-dim)
- Neo4j Ready: Relationships prepared for graph sync

---

### `code_index_directory`

**Purpose**: Recursively index all matching files in a directory.

**Parameters**:
```json
{
  "directory": "string (required, absolute path)",
  "pattern": "string (required, glob pattern like '*.rs')"
}
```

**Usage Examples**:

**Human Usage**:
```json
// Index entire module
{
  "directory": "/home/user/project/src/cognition",
  "pattern": "*.rs"
}

// Index TypeScript files
{
  "directory": "/home/user/frontend/src",
  "pattern": "*.{ts,tsx}"
}
```

**LLM Usage**:
```javascript
// Index all cognition modules
mcp__syncore__code_index_directory(
  "/home/feanor/Projects/SynCore/syncore/src/cognition",
  "*.rs"
)

// Index test files
mcp__syncore__code_index_directory(
  "/home/feanor/Projects/SynCore/syncore/tests",
  "*_tests.rs"
)
```

**Returns**:
```json
{
  "directory": "/home/feanor/Projects/SynCore/syncore/src/cognition",
  "pattern": "*.rs",
  "indexed_files": 12,
  "total_entities": 152,
  "languages": {
    "rust": 12
  }
}
```

---

### `code_search`

**Purpose**: Semantic search across indexed code entities.

**Parameters**:
```json
{
  "query": "string (required)",
  "limit": "integer (optional, default: 10)"
}
```

**Usage Examples**:

**Human Usage**:
```json
{
  "query": "pattern matching with success rate",
  "limit": 5
}
```

**LLM Usage**:
```javascript
// Find relevant code before implementation
const relevant = mcp__syncore__code_search(
  "execute plan with error handling",
  10
)

// Find similar implementations
const similar = mcp__syncore__code_search(
  "context bundle composition",
  5
)
```

**Returns**:
```json
{
  "query": "pattern matching with success rate",
  "entities": [
    {
      "id": 11296,
      "file_path": "src/cognition/pattern_engine.rs",
      "entity_type": "Function",
      "name": "recommend_patterns_for_query",
      "signature": "recommend_patterns_for_query(intent, mode, memory, namespace, limit)",
      "score": 0.8932,
      "line_start": 98,
      "line_end": 145
    }
  ],
  "total": 1
}
```

---

### `code_graph_fusion_query`

**Purpose**: RAGGraph tri-modal fusion query (Vector + Graph + Symbolic).

**Modes**:
- **simple**: Vector-only search (α=0.6)
- **attention**: Context-aware fusion (α=0.4-0.6, adaptive)
- **reasoning**: Full tri-modal with higher-order relationships (α=0.4, γ term)

**Parameters**:
```json
{
  "query": "string (required)",
  "mode_hint": "string (optional: 'simple', 'attention', 'reasoning')",
  "top_k": "integer (optional, default: 10)",
  "namespace": "string (optional)"
}
```

**Usage Examples**:

**Human Usage**:
```json
// Simple semantic search
{
  "query": "plan engine execution",
  "mode_hint": "simple",
  "top_k": 5
}

// Context-aware search
{
  "query": "how does orchestrator integrate planning",
  "mode_hint": "attention",
  "top_k": 8
}

// Complex reasoning query
{
  "query": "reasoning continuity and pattern engine integration",
  "mode_hint": "reasoning",
  "top_k": 10
}
```

**LLM Usage**:
```javascript
// Simple lookup
const entities = mcp__syncore__code_graph_fusion_query(
  "plan executor error handling",
  "simple",
  5
)

// Complex architectural query
const architecture = mcp__syncore__code_graph_fusion_query(
  "how do self-consistency checks influence plan generation across continuity engine and pattern recommendations",
  "reasoning",
  15
)
```

**Returns**:
```json
{
  "entities": [
    {
      "entity": {
        "id": 11296,
        "file_path": "src/cognition/plan_engine.rs",
        "entity_type": "Function",
        "name": "generate_plan",
        "signature": "generate_plan(query, intent, selected_mode, recommended_patterns, consistency, bundle)",
        "line_start": 70,
        "line_end": 122,
        "docstring": "/// Generate an execution plan based on cognitive context",
        "language": "rust"
      },
      "combined_score": 0.8456,
      "vector_score": 0.7892,
      "graph_score": 0.9234
    }
  ],
  "selected_mode": "attention",
  "debug_info": {
    "alpha": 0.512,
    "query_length": 58,
    "total_matches": 16,
    "context_complexity": 1.06,
    "attention_alpha": 0.512
  }
}
```

**Fusion Algorithm Details**:

**Simple Mode** (α=0.6):
```
combined_score = α * vector_score + (1-α) * graph_score
```

**Attention Mode** (adaptive α):
```
context_complexity = log(num_entities + 1) * entity_diversity
α = 0.4 + 0.2 / (1 + context_complexity)
combined_score = α * vector_score + (1-α) * graph_score
```

**Reasoning Mode** (γ term):
```
higher_order_score = γ * Σ(neighbor_scores) * relationship_weights
combined_score = α * vector_score + (1-α) * graph_score + γ * higher_order_score
```

---

### `code_graph_sync_neo4j`

**Purpose**: Sync indexed code entities and relationships to Neo4j graph database.

**Parameters**:
```json
{
  "namespace": "string (optional)",
  "limit": "integer (optional)"
}
```

**Usage Examples**:

**Human Usage**:
```json
// Sync all entities
{}

// Sync specific namespace
{
  "namespace": "syncore_cognition",
  "limit": 100
}
```

**LLM Usage**:
```javascript
// After indexing - sync to graph
mcp__syncore__code_graph_sync_neo4j("syncore_cognition", 500)
```

**Returns**:
```json
{
  "edges_processed": 48,
  "edges_created": 45,
  "edges_skipped": 3,
  "namespaces": ["syncore_cognition"]
}
```

**Troubleshooting graph_score=0 Issues**:

If you see `graph_score=0.0` for all entities in fusion queries, check these common issues:

1. **Entities not synced to Neo4j**:
   - Solution: Run `code_graph_sync_neo4j` with high limit (e.g., 50000)
   - Verify: Check `nodes_synced` count in response

2. **Stale vector snapshot IDs**:
   - Symptom: `[WARN] Vector→Entity mapping failed: vector_id=XXXX not found`
   - Cause: Vector snapshot contains IDs from old database session
   - Fix: Delete snapshot files and reindex:
     ```bash
     rm -f syncore_code_graph.vectors syncore_code_graph.meta
     # Then re-run code_index_directory
     ```
   - Auto-fix: Starting Jan 2025, snapshots auto-validate and rebuild if IDs mismatch

3. **Accidental :memory: database**:
   - Symptom: Data disappears after MCP reconnect
   - Cause: CodeGraph initialized with `:memory:` instead of file path
   - Fix: Use persistent path: `DB_PATH=./syncore.db` (not `:memory:`)
   - Protection: CodeGraph now rejects `:memory:` database at initialization

4. **Neo4j not running or wrong credentials**:
   - Symptom: `graph_score=0.0` with no warning logs
   - Solution: Verify Neo4j connection and credentials
   - Test: Run `graph_query` tool with simple query

All these issues are now logged with actionable warnings (Jan 2025 fixes).

---

## Sequential Reasoning Tools

### `sequential_cycle`

**Purpose**: Run sequential reasoning loop (Think → Decide → Act → Observe → Reflect).

**Requires**: Ollama instance running

**Parameters**:
```json
{
  "max_cycles": "integer (optional, default: 5)"
}
```

**Usage Examples**:

**Human Usage**:
```json
{
  "max_cycles": 3
}
```

**LLM Usage**:
```javascript
// Run reasoning loop for complex problem
mcp__syncore__sequential_cycle(5)
```

**Returns**:
```json
{
  "cycles_completed": 3,
  "final_state": "Reflect",
  "thoughts": [
    {
      "step": 1,
      "state": "Think",
      "content": "Analyzing problem space...",
      "timestamp": 1732179123
    },
    {
      "step": 2,
      "state": "Decide",
      "content": "Will execute code_search to find relevant patterns",
      "timestamp": 1732179124
    }
  ],
  "outcome": "success"
}
```

---

### `sequential_record`

**Purpose**: Record a thought step in reasoning chain.

**Parameters**:
```json
{
  "step_number": "integer (required)",
  "thought": "string (required)",
  "reasoning": "string (required)",
  "action": "string (optional)",
  "observation": "string (optional)",
  "task_id": "integer (optional)"
}
```

**Usage Examples**:

**LLM Usage**:
```javascript
// Record reasoning step
mcp__syncore__sequential_record(
  1,
  "Need to understand how plan generation works",
  "Before implementing new feature, should analyze existing code",
  "code_graph_fusion_query",
  "Found generate_plan function in plan_engine.rs"
)
```

**Returns**:
```json
{
  "step_id": 1234,
  "step_number": 1,
  "stored": true
}
```

---

### `sequential_get`

**Purpose**: Retrieve all thought steps for a task.

**Parameters**:
```json
{
  "task_id": "integer (required)"
}
```

**Returns**:
```json
{
  "task_id": 42,
  "steps": [
    {
      "step_number": 1,
      "thought": "Need to understand how plan generation works",
      "reasoning": "Before implementing new feature...",
      "action": "code_graph_fusion_query",
      "observation": "Found generate_plan function"
    }
  ],
  "total_steps": 1
}
```

---

### `sequential_search`

**Purpose**: Semantic search across thought steps.

**Parameters**:
```json
{
  "query": "string (required)"
}
```

**Returns**:
```json
{
  "query": "error handling patterns",
  "steps": [
    {
      "step_id": 1234,
      "thought": "Investigating error propagation...",
      "score": 0.89
    }
  ]
}
```

---

## IntelliTask Tools

### `intellitask_generate`

**Purpose**: AI-powered PRD parsing to generate task breakdown.

**Requires**: Ollama instance running

**Parameters**:
```json
{
  "prd_content": "string (required, markdown or plain text)"
}
```

**Usage Examples**:

**Human Usage**:
```json
{
  "prd_content": "# Phase R6.0 - Evidence Provenance\n\n## Goals\n1. Design evidence chain data structure\n2. Implement evidence tracking\n3. Add provenance queries\n\n## Success Criteria\n- All evidence has source attribution\n- Chain integrity validated\n- Query API functional"
}
```

**LLM Usage**:
```javascript
// Generate tasks from PRD
const prd = `
# Feature: Multi-modal Search Enhancement

## Objectives
- Improve vector search accuracy by 20%
- Add graph traversal optimization
- Implement caching layer

## Deliverables
- Enhanced embeddings model
- Graph query optimizer
- Redis cache integration
`

const breakdown = mcp__syncore__intellitask_generate(prd)
```

**Returns**:
```json
{
  "prd_title": "Phase R6.0 - Evidence Provenance",
  "tasks": [
    {
      "id": 1,
      "title": "Design evidence chain data structure",
      "description": "Create schema for evidence nodes with source attribution",
      "priority": 1,
      "estimated_complexity": "medium",
      "dependencies": []
    },
    {
      "id": 2,
      "title": "Implement evidence tracking",
      "description": "Add tracking logic to capture evidence sources",
      "priority": 2,
      "estimated_complexity": "high",
      "dependencies": [1]
    }
  ],
  "total_tasks": 2
}
```

---

### `intellitask_prioritize`

**Purpose**: AI-powered task prioritization with dependency awareness.

**Parameters**:
```json
{
  "tasks_json": "string (required, JSON array)",
  "business_context": "string (optional)"
}
```

**Usage Examples**:

**Human Usage**:
```json
{
  "tasks_json": "[{\"id\": 1, \"title\": \"Design API\"}, {\"id\": 2, \"title\": \"Implement tests\"}]",
  "business_context": "Launch deadline is Q1 2025. Testing is critical."
}
```

**Returns**:
```json
{
  "prioritized_tasks": [
    {
      "id": 1,
      "priority": 1,
      "reasoning": "Foundation for all other work"
    },
    {
      "id": 2,
      "priority": 2,
      "reasoning": "Critical for quality assurance before launch"
    }
  ]
}
```

---

### `intellitask_subtasks`

**Purpose**: Generate subtasks for a parent task.

**Parameters**:
```json
{
  "parent_task_id": "string (required)",
  "parent_task_json": "string (required, JSON object)",
  "codebase_context": "string (optional)"
}
```

**Returns**:
```json
{
  "parent_task_id": "1",
  "subtasks": [
    {
      "id": "1.1",
      "title": "Define evidence struct",
      "estimated_hours": 2
    },
    {
      "id": "1.2",
      "title": "Create database schema",
      "estimated_hours": 3
    }
  ]
}
```

---

### `intellitask_next`

**Purpose**: AI-suggested next task based on completion history.

**Parameters**:
```json
{
  "completed_tasks": "array of strings (required)",
  "remaining_tasks_json": "string (required, JSON array)"
}
```

**Returns**:
```json
{
  "suggested_task_id": "2",
  "reasoning": "Dependencies satisfied. High priority. On critical path."
}
```

---

### `intellitask_get`

**Purpose**: Get task details by ID.

**Parameters**:
```json
{
  "task_id": "integer (required)"
}
```

**Returns**:
```json
{
  "task_id": 42,
  "goal": "Implement Phase R6.0",
  "description": "Design and implement evidence provenance system",
  "status": "open",
  "priority": 1,
  "parent_id": null,
  "created_at": 1732179123,
  "updated_at": 1732179123
}
```

---

### `intellitask_get_subtasks`

**Purpose**: Get all subtasks for a parent task.

**Parameters**:
```json
{
  "parent_id": "integer (required)"
}
```

**Returns**:
```json
{
  "parent_id": 42,
  "subtasks": [
    {
      "task_id": 43,
      "goal": "Design evidence chain",
      "status": "open"
    },
    {
      "task_id": 44,
      "goal": "Implement tracking",
      "status": "open"
    }
  ],
  "total": 2
}
```

---

### `intellitask_subtask_stats`

**Purpose**: Get subtask statistics for a parent.

**Parameters**:
```json
{
  "parent_id": "integer (required)"
}
```

**Returns**:
```json
{
  "parent_id": 42,
  "total_subtasks": 5,
  "completed": 2,
  "in_progress": 1,
  "open": 2,
  "completion_percentage": 40
}
```

---

### `intellitask_task_statistics`

**Purpose**: Get overall task statistics.

**Parameters**: None

**Returns**:
```json
{
  "total_tasks": 127,
  "by_status": {
    "open": 45,
    "in_progress": 12,
    "done": 68,
    "cancelled": 2
  },
  "by_priority": {
    "1": 15,
    "2": 30,
    "3": 50,
    "4": 20,
    "5": 12
  }
}
```

---

### `intellitask_prd_statistics`

**Purpose**: Get task statistics for a specific PRD.

**Parameters**:
```json
{
  "prd_title": "string (required)"
}
```

**Returns**:
```json
{
  "prd_title": "Phase R5.0 Implementation",
  "total_tasks": 8,
  "completed": 8,
  "in_progress": 0,
  "open": 0,
  "completion_percentage": 100
}
```

---

## Application Tracking Tools

### `application_record`

**Purpose**: Record a code change in application structure map.

**Parameters**:
```json
{
  "file_path": "string (required)",
  "change_type": "string (required: 'create', 'modify', 'delete')",
  "line_start": "integer (required)",
  "line_end": "integer (required)",
  "description": "string (required)",
  "old_content": "string (optional)",
  "new_content": "string (optional)",
  "task_id": "integer (optional)"
}
```

**Usage Examples**:

**LLM Usage**:
```javascript
// Record file creation
mcp__syncore__application_record(
  "/home/feanor/Projects/SynCore/syncore/src/cognition/plan_engine.rs",
  "create",
  1,
  358,
  "Created Plan Engine module - generates execution plans based on intent, patterns, and consistency",
  null,
  "// Full file content...",
  42
)

// Record modification
mcp__syncore__application_record(
  "/home/feanor/Projects/SynCore/syncore/src/cognition/orchestrator.rs",
  "modify",
  270,
  287,
  "Added plan generation in enrich_query_with_context_bundle",
  "    // Step 6: Build EnrichedContext",
  "    // Step 6: Generate execution plan (Phase R5.0)\n    let plan = if let (Some(ref bundle), ...",
  42
)
```

**Returns**:
```json
{
  "change_id": 1234,
  "file_path": "src/cognition/plan_engine.rs",
  "change_type": "create",
  "recorded_at": 1732179123
}
```

---

### `application_get`

**Purpose**: Get all changes for a task.

**Parameters**:
```json
{
  "task_id": "integer (required)"
}
```

**Returns**:
```json
{
  "task_id": 42,
  "changes": [
    {
      "change_id": 1234,
      "file_path": "src/cognition/plan_engine.rs",
      "change_type": "create",
      "description": "Created Plan Engine module"
    }
  ],
  "total_changes": 1
}
```

---

### `application_search`

**Purpose**: Semantic search across code changes.

**Parameters**:
```json
{
  "query": "string (required)"
}
```

**Returns**:
```json
{
  "query": "plan generation implementation",
  "changes": [
    {
      "change_id": 1234,
      "file_path": "src/cognition/plan_engine.rs",
      "description": "Created Plan Engine module",
      "score": 0.92
    }
  ]
}
```

---

### `application_history`

**Purpose**: Get change history for a specific file.

**Parameters**:
```json
{
  "file_path": "string (required)"
}
```

**Returns**:
```json
{
  "file_path": "src/cognition/orchestrator.rs",
  "changes": [
    {
      "change_id": 1235,
      "change_type": "modify",
      "description": "Added plan field to EnrichedContext",
      "timestamp": 1732179100
    },
    {
      "change_id": 1236,
      "change_type": "modify",
      "description": "Added plan generation logic",
      "timestamp": 1732179200
    }
  ],
  "total_changes": 2
}
```

---

## Document Management Tools

### `document_index`

**Purpose**: Index documents from a directory into global knowledge store.

**Parameters**:
```json
{
  "directory": "string (required)"
}
```

**Usage Examples**:

**Human Usage**:
```json
{
  "directory": "/home/user/docs"
}
```

**LLM Usage**:
```javascript
// Index documentation
mcp__syncore__document_index("/home/feanor/Projects/SynCore/syncore/research")
```

**Returns**:
```json
{
  "directory": "/home/feanor/Projects/SynCore/syncore/research",
  "indexed_files": 25,
  "total_chunks": 487,
  "file_types": ["md", "txt", "pdf"]
}
```

---

### `document_search`

**Purpose**: Semantic search across indexed documents.

**Parameters**:
```json
{
  "query": "string (required)",
  "limit": "integer (optional, default: 5)"
}
```

**Returns**:
```json
{
  "query": "evidence provenance design",
  "documents": [
    {
      "chunk_id": 123,
      "content": "Evidence provenance tracks the origin and transformation...",
      "score": 0.89,
      "source_file": "research/evidence_design.md"
    }
  ]
}
```

---

## Graph Database Tools

### `graph_query`

**Purpose**: Execute Cypher query on Neo4j graph database.

**Parameters**:
```json
{
  "cypher": "string (required)",
  "params": "object (optional)"
}
```

**Usage Examples**:

**Human Usage**:
```json
{
  "cypher": "MATCH (n:CodeEntity {name: $name}) RETURN n",
  "params": {"name": "generate_plan"}
}
```

**LLM Usage**:
```javascript
// Find function relationships
const query = `
MATCH (f:Function {name: 'generate_plan'})-[:CALLS]->(called:Function)
RETURN f.name, called.name, called.file_path
LIMIT 10
`
mcp__syncore__graph_query(query)

// Find related entities
const related = `
MATCH (e:CodeEntity {id: $id})-[r]-(related:CodeEntity)
RETURN type(r), related.name, related.entity_type
`
mcp__syncore__graph_query(related, { id: 11296 })
```

**Returns**:
```json
{
  "results": [
    {
      "f.name": "generate_plan",
      "called.name": "determine_strategy",
      "called.file_path": "src/cognition/plan_engine.rs"
    }
  ],
  "total": 1
}
```

---

### `graph_insert`

**Purpose**: Execute write Cypher query (CREATE, MERGE, SET).

**Parameters**:
```json
{
  "cypher": "string (required)",
  "params": "object (optional)"
}
```

**Usage Examples**:

**Human Usage**:
```json
{
  "cypher": "CREATE (n:CodeEntity {name: $name, type: $type})",
  "params": {
    "name": "new_function",
    "type": "Function"
  }
}
```

**Returns**:
```json
{
  "nodes_created": 1,
  "relationships_created": 0,
  "properties_set": 2
}
```

---

### `graph_relate`

**Purpose**: Create relationship between two nodes.

**Parameters**:
```json
{
  "from_id": "integer (required)",
  "to_id": "integer (required)",
  "rel_type": "string (required: 'CALLS', 'DEPENDS_ON', 'IMPLEMENTS', etc.)",
  "from_label": "string (optional)",
  "to_label": "string (optional)"
}
```

**Usage Examples**:

**Human Usage**:
```json
{
  "from_id": 11296,
  "to_id": 11297,
  "rel_type": "CALLS",
  "from_label": "Function",
  "to_label": "Function"
}
```

**Returns**:
```json
{
  "relationship_created": true,
  "from_id": 11296,
  "to_id": 11297,
  "rel_type": "CALLS"
}
```

---

### `raggraph_query`

**Purpose**: Execute RAGGraph query with multi-hop reasoning.

**Parameters**:
```json
{
  "query_text": "string (required)"
}
```

**Returns**:
```json
{
  "entities": [...],
  "paths": [
    {
      "nodes": ["generate_plan", "determine_strategy", "ContextBundle"],
      "relationships": ["CALLS", "USES"]
    }
  ],
  "reasoning_depth": 2
}
```

---

### `raggraph_multihop`

**Purpose**: Execute multi-hop graph traversal from seed nodes.

**Parameters**:
```json
{
  "seed_nodes": "array of integers (required)"
}
```

**Usage Examples**:

**Human Usage**:
```json
{
  "seed_nodes": [11296, 11297, 11298]
}
```

**Returns**:
```json
{
  "seed_nodes": [11296, 11297, 11298],
  "discovered_entities": [
    {
      "id": 11299,
      "name": "build_index_search_plan",
      "hops_from_seed": 1
    },
    {
      "id": 11300,
      "name": "build_graph_traversal_plan",
      "hops_from_seed": 1
    }
  ],
  "total_discovered": 2,
  "max_hops": 2
}
```

---

## Agent Communication Tools

### `agent_register`

**Purpose**: Register agent ID and capabilities.

**Parameters**:
```json
{
  "id": "string (required)",
  "capabilities": "array of strings (required)"
}
```

**Usage Examples**:

**LLM Usage**:
```javascript
// Register as planning agent
mcp__syncore__agent_register(
  "planning_agent_001",
  ["plan_generation", "task_decomposition", "code_analysis"]
)
```

**Returns**:
```json
{
  "agent_id": "planning_agent_001",
  "registered": true,
  "capabilities": ["plan_generation", "task_decomposition", "code_analysis"]
}
```

---

### `agent_send`

**Purpose**: Send message to another agent.

**Parameters**:
```json
{
  "to": "string (required, agent ID)",
  "message": "string (required)"
}
```

**Returns**:
```json
{
  "sent": true,
  "to": "execution_agent_002",
  "message_id": "msg_12345"
}
```

---

### `agent_recv`

**Purpose**: Receive pending messages.

**Parameters**:
```json
{
  "agent": "string (required, agent ID)"
}
```

**Returns**:
```json
{
  "agent": "planning_agent_001",
  "messages": [
    {
      "message_id": "msg_12345",
      "from": "execution_agent_002",
      "content": "Task 42 execution complete",
      "timestamp": 1732179123
    }
  ]
}
```

---

### `agent_poll`

**Purpose**: Wait for next message with timeout.

**Parameters**:
```json
{
  "agent": "string (required)",
  "timeout_ms": "integer (required)"
}
```

**Returns**:
```json
{
  "message": {
    "from": "execution_agent_002",
    "content": "Task 42 execution complete"
  },
  "received": true
}

// Or if timeout:
{
  "received": false,
  "timeout": true
}
```

---

### `agent_status`

**Purpose**: Update agent status.

**Parameters**:
```json
{
  "id": "string (required)",
  "status": "object (required)"
}
```

**Returns**:
```json
{
  "agent_id": "planning_agent_001",
  "status_updated": true
}
```

---

### `agent_list`

**Purpose**: List all registered agents.

**Parameters**: None

**Returns**:
```json
{
  "agents": [
    {
      "id": "planning_agent_001",
      "capabilities": ["plan_generation"],
      "status": "active"
    },
    {
      "id": "execution_agent_002",
      "capabilities": ["code_execution"],
      "status": "idle"
    }
  ],
  "total": 2
}
```

---

### `agent_task`

**Purpose**: Send structured task to agent.

**Parameters**:
```json
{
  "to": "string (required)",
  "task_id": "string (required)",
  "task_type": "string (required)",
  "payload": "object (required)"
}
```

**Returns**:
```json
{
  "task_sent": true,
  "task_id": "task_123",
  "to": "execution_agent_002"
}
```

---

### `agent_result`

**Purpose**: Submit task result.

**Parameters**:
```json
{
  "from": "string (required)",
  "task_id": "string (required)",
  "result": "object (required)"
}
```

**Returns**:
```json
{
  "result_received": true,
  "task_id": "task_123"
}
```

---

## Logging Tools

### `logs_tail`

**Purpose**: Get recent log entries.

**Parameters**:
```json
{
  "n": "integer (optional, default: 50)"
}
```

**Usage Examples**:

**Human Usage**:
```json
{"n": 100}
```

**LLM Usage**:
```javascript
// Check recent activity
const logs = mcp__syncore__logs_tail(50)

// Debug error
const error_logs = mcp__syncore__logs_tail(200)
```

**Returns**:
```json
{
  "logs": [
    {
      "timestamp": 1732179123,
      "level": "INFO",
      "message": "Code entity indexed: generate_plan",
      "metadata": {
        "file": "plan_engine.rs",
        "line": 70
      }
    }
  ],
  "total": 50
}
```

---

## Tool Metadata

### `tool_metadata_list`

**Purpose**: List metadata for all MCP tools (category, cost, side effects).

**Parameters**: None

**Returns**:
```json
{
  "tools": [
    {
      "name": "memory_store",
      "category": "storage",
      "cost": "low",
      "side_effects": true,
      "description": "Store key-value pair in hybrid SQLite + Sled storage"
    },
    {
      "name": "code_graph_fusion_query",
      "category": "search",
      "cost": "medium",
      "side_effects": false,
      "description": "RAGGraph tri-modal fusion query"
    }
  ],
  "total": 67
}
```

---

## Best Practices

### For Humans

1. **Session Initialization**:
   ```
   - Load context: memory_query("project_context")
   - Check tasks: intellitask_list({"status": "open"})
   - Review recent work: logs_tail(50)
   ```

2. **Code Analysis Workflow**:
   ```
   - Index files: code_index_directory(dir, "*.rs")
   - Search semantically: code_graph_fusion_query(query, "attention", 10)
   - Analyze structure: parser_analyze(file_path)
   - Search patterns: parser_search(pattern, context_lines)
   ```

3. **Task Management**:
   ```
   - Generate from PRD: intellitask_generate(prd_content)
   - Get next task: intellitask_next_ready()
   - Update status: intellitask_update_status(id, "in_progress")
   - Track changes: application_record(file, "modify", ...)
   ```

4. **Knowledge Storage**:
   ```
   - Store insights: vector_insert(text, metadata)
   - Store facts: memory_store(key, value)
   - Index docs: document_index(directory)
   ```

### For LLMs

1. **Token Optimization**:
   ```javascript
   // DON'T: Read entire files (5000+ tokens)
   const file = Read("/path/to/file.rs")

   // DO: Use targeted tools (100 tokens)
   const relevant = mcp__syncore__code_graph_fusion_query(
     "generate_plan implementation",
     "simple",
     5
   )
   ```

2. **Context Loading**:
   ```javascript
   // ALWAYS at session start
   const context = mcp__syncore__memory_query("project_context")
   const task = mcp__syncore__memory_query("current_task")
   const blockers = mcp__syncore__memory_query("blockers")
   ```

3. **Context Saving**:
   ```javascript
   // ALWAYS at session end
   mcp__syncore__memory_store("task_status", "Phase R5.0 complete")
   mcp__syncore__memory_store("next_steps", "Begin Phase R6.0 planning")
   mcp__syncore__memory_store("files_modified", "plan_engine.rs, orchestrator.rs")
   ```

4. **Search Before Implementation**:
   ```javascript
   // Before writing code - find similar patterns
   const existing = mcp__syncore__code_search(
     "error handling in async functions",
     10
   )

   // Check for existing implementations
   const similar = mcp__syncore__parser_search(
     "pub async fn.*Result",
     context_lines: 5
   )
   ```

5. **Incremental Indexing**:
   ```javascript
   // After creating/modifying files - index immediately
   mcp__syncore__code_index(
     "/home/user/project/src/new_module.rs"
   )
   ```

---

## Common Workflows

### Workflow 1: Implement New Feature

**For LLMs**:

```javascript
// 1. Load session context
const context = mcp__syncore__memory_query("project_context")
const current_task = mcp__syncore__memory_query("current_task")

// 2. Search for similar implementations
const similar = mcp__syncore__code_graph_fusion_query(
  "similar feature implementation patterns",
  "attention",
  10
)

// 3. Analyze relevant files
const structure = mcp__syncore__parser_analyze(
  "/path/to/relevant/file.rs"
)

// 4. Create task if needed
const task_id = mcp__syncore__task_create(
  "Implement new feature X",
  1
)

// 5. Update status
mcp__syncore__intellitask_update_status(task_id, "in_progress")

// 6. [Implement the feature using other tools]

// 7. Index new/modified files
mcp__syncore__code_index("/path/to/new/file.rs")

// 8. Record changes
mcp__syncore__application_record(
  "/path/to/new/file.rs",
  "create",
  1,
  200,
  "Implemented feature X with error handling",
  null,
  "// file content...",
  task_id
)

// 9. Store knowledge
mcp__syncore__vector_insert(
  "Feature X implementation uses pattern Y for error handling...",
  { feature: "X", phase: "R6.0" }
)

// 10. Complete task
mcp__syncore__intellitask_update_status(task_id, "done")

// 11. Store session progress
mcp__syncore__memory_store("last_completed", "Feature X implementation")
mcp__syncore__memory_store("next_steps", "Write tests for feature X")
```

---

### Workflow 2: Debug Complex Issue

**For LLMs**:

```javascript
// 1. Search relevant code
const relevant_code = mcp__syncore__code_graph_fusion_query(
  "error handling in plan execution",
  "reasoning",
  15
)

// 2. Search for similar issues
const similar_issues = mcp__syncore__vector_search(
  "execution failed with 0 steps completed",
  10
)

// 3. Check logs
const recent_logs = mcp__syncore__logs_tail(100)

// 4. Analyze file structure
const structure = mcp__syncore__parser_analyze(
  "/path/to/problematic/file.rs"
)

// 5. Search for patterns
const error_patterns = mcp__syncore__parser_search(
  "\\.unwrap\\(\\)|\\.expect\\(",
  5
)

// 6. Record debugging thoughts
mcp__syncore__sequential_record(
  1,
  "Error occurs when allow_write=false and tool is write operation",
  "Need to check is_write_operation() function",
  "parser_search",
  "Found is_write_operation() in plan_executor.rs:143"
)

// 7. Store solution
mcp__syncore__vector_insert(
  "Solution: When execute_plan shows 0 steps, check is_write_operation(). Write ops require allow_write=true.",
  { type: "solution", category: "execution" }
)
```

---

### Workflow 3: Analyze Codebase Architecture

**For Humans or LLMs**:

```javascript
// 1. Index entire codebase
mcp__syncore__code_index_directory(
  "/home/user/project/src",
  "*.rs"
)

// 2. Sync to graph database
mcp__syncore__code_graph_sync_neo4j("project_name", 1000)

// 3. Query architecture
const architecture = mcp__syncore__code_graph_fusion_query(
  "main system components and their relationships",
  "reasoning",
  20
)

// 4. Find key modules
const modules = mcp__syncore__graph_query(`
  MATCH (m:Module)
  RETURN m.name, size((m)-[:CONTAINS]->()) as entities
  ORDER BY entities DESC
  LIMIT 10
`)

// 5. Find most connected functions
const hubs = mcp__syncore__graph_query(`
  MATCH (f:Function)
  RETURN f.name, f.file_path,
         size((f)-[:CALLS]->()) as outgoing,
         size((f)<-[:CALLS]-()) as incoming
  ORDER BY (outgoing + incoming) DESC
  LIMIT 10
`)

// 6. Store architecture insights
mcp__syncore__memory_store(
  "architecture_insights",
  JSON.stringify({
    top_modules: modules.results,
    hub_functions: hubs.results,
    total_entities: architecture.debug_info.total_matches
  })
)
```

---

## Error Handling

### Common Errors and Solutions

**Error**: `"Key not found"`
```javascript
// Solution: Check if key exists
const result = mcp__syncore__memory_query("key")
if (result.found) {
  // Use result.value
} else {
  // Handle missing key
}
```

**Error**: `"File not found"`
```javascript
// Solution: Use absolute paths
mcp__syncore__parser_analyze(
  "/home/user/project/src/file.rs"  // ✓ Correct
)
// Not:
mcp__syncore__parser_analyze("src/file.rs")  // ✗ Wrong
```

**Error**: `"Neo4j connection failed"`
```javascript
// Solution: Check environment variables
// NEO4J_URI="bolt://127.0.0.1:7687"
// NEO4J_USER="neo4j"
// NEO4J_PASS="password"

// Or handle gracefully:
const result = mcp__syncore__graph_query(cypher)
if (result.error) {
  // Fallback to vector-only search
}
```

**Error**: `"Ollama not available"`
```javascript
// Solution: Check Ollama is running
// Or use tools that don't require LLM:
// - code_index, code_search (no LLM needed)
// - parser_analyze, parser_search (tree-sitter only)
// - memory_store, memory_query (direct storage)
```

**Error**: `"Write operation requires allow_write=true"`
```javascript
// This is not an error - it's a safety feature!
// Tools that modify data: memory_store, vector_insert, code_index,
//                         file_update, task_create
// Use read-only alternatives when exploring:
// - memory_query instead of memory_store
// - code_search instead of code_index
// - parser_search instead of file modifications
```

---

## Performance Characteristics

| Tool | Latency | Complexity | Storage |
|------|---------|------------|---------|
| memory_store | <1ms | O(1) | SQLite + Sled |
| memory_query | <1ms | O(1) | SQLite + Sled |
| vector_insert | ~5ms | O(d) | HNSW index |
| vector_search | ~10ms | O(n) | Linear scan + SIMD |
| code_index | ~50ms | O(n) | SQLite + Vector + Neo4j |
| code_search | ~15ms | O(n) | Vector search |
| code_graph_fusion_query | ~30ms | O(n + m) | Vector + Graph |
| parser_analyze | ~5ms | O(n) | Tree-sitter |
| parser_search | ~20ms | O(n) | Ripgrep |
| graph_query | ~10ms | O(n + m) | Neo4j Cypher |
| sequential_cycle | ~2000ms | O(k) | Ollama LLM calls |
| intellitask_generate | ~5000ms | O(n) | Ollama LLM calls |

**Legend**:
- d = embedding dimensions (384)
- n = dataset size
- m = number of edges
- k = number of reasoning cycles

---

## Advanced Topics

### RAGGraph Fusion Modes

**When to use each mode**:

- **Simple** (α=0.6): Fast lookup, known entities, simple queries
  ```javascript
  // Example: Find a specific function
  code_graph_fusion_query("generate_plan function", "simple", 5)
  ```

- **Attention** (adaptive α): Context-aware, moderate complexity
  ```javascript
  // Example: Understand relationships
  code_graph_fusion_query(
    "how does orchestrator integrate planning",
    "attention",
    10
  )
  ```

- **Reasoning** (γ term): Complex architectural queries, multi-hop
  ```javascript
  // Example: Deep analysis
  code_graph_fusion_query(
    "how do self-consistency checks influence plan generation across continuity engine",
    "reasoning",
    15
  )
  ```

---

### Memory Key Conventions

**Recommended key patterns**:

```
// Project context
project_context
project_architecture
project_dependencies

// Phase/task tracking
phase_N_status
phase_N_notes
current_task
next_steps

// Technical details
embedding_model
database_schema
api_endpoints

// Session state
last_modified_files
blockers
completed_today

// Analysis results
analysis_embeddings
analysis_performance
architecture_insights
```

---

### Tool Composition Patterns

**Pattern 1: Search → Analyze → Index**
```javascript
// 1. Find relevant files
const files = code_graph_fusion_query("authentication logic", "attention", 10)

// 2. Analyze each file
for (const entity of files.entities) {
  const structure = parser_analyze(entity.file_path)
  // Process structure...
}

// 3. Index new implementations
code_index("/path/to/new/auth.rs")
```

**Pattern 2: Load → Process → Store**
```javascript
// 1. Load context
const context = memory_query("project_context")

// 2. Process with LLM
const result = sequential_cycle(5)

// 3. Store insights
vector_insert(result.insights, {type: "reasoning"})
memory_store("last_reasoning_result", JSON.stringify(result))
```

---

## Appendix A: Tool Categories

**Storage (6 tools)**:
- memory_store, memory_query
- vector_insert, vector_search
- document_index, document_search

**Code Analysis (10 tools)**:
- parser_analyze, parser_search
- code_index, code_index_directory, code_search
- code_graph_fusion_query, code_graph_sync_neo4j
- application_record, application_get, application_history

**Task Management (11 tools)**:
- task_create, task_list
- intellitask_generate, intellitask_prioritize, intellitask_next
- intellitask_subtasks, intellitask_get, intellitask_get_subtasks
- intellitask_update_status, intellitask_subtask_stats
- intellitask_task_statistics, intellitask_prd_statistics
- intellitask_next_ready

**Graph Database (5 tools)**:
- graph_query, graph_insert, graph_relate
- raggraph_query, raggraph_multihop

**Sequential Reasoning (4 tools)**:
- sequential_cycle, sequential_record
- sequential_get, sequential_search

**Agent Communication (8 tools)**:
- agent_register, agent_send, agent_recv, agent_poll
- agent_status, agent_list, agent_task, agent_result

**Logging & Metadata (2 tools)**:
- logs_tail, tool_metadata_list

**Total**: 67 tools

---

## Appendix B: Storage Locations

```
Main Database: syncore.db
  - Tables: memory, tasks, task_links, steps, reasoning_ledger

Code Graph Database: syncore_code_graph.db
  - Tables: code_entities, code_relationships, code_metrics

Vector Index: vector_store/ (HNSW)
  - Format: Binary serialized
  - Dimensions: 384

Cache: sled_cache/
  - Format: Sled B-tree
  - Fast key-value access

Neo4j Graph: bolt://127.0.0.1:7687
  - Labels: CodeEntity, Function, Class, Module
  - Relationships: CALLS, USES, IMPLEMENTS, DEPENDS_ON

Logs: logs/
  - Format: Markdown structured logs
  - Files: reasoning_*.md
```

---

## Appendix C: Environment Variables

```bash
# Database
DB_PATH=/path/to/syncore.db                    # Default: ./syncore.db
CODE_GRAPH_DB=/path/to/syncore_code_graph.db  # Default: ./syncore_code_graph.db

# Neo4j
NEO4J_URI=bolt://127.0.0.1:7687               # Default: bolt://127.0.0.1:7687
NEO4J_USER=neo4j                               # Default: neo4j
NEO4J_PASS=password                            # Required

# Ollama
OLLAMA_HOST=http://127.0.0.1:11434            # Default: http://127.0.0.1:11434
OLLAMA_MODEL=mistral                           # Default: mistral

# MCP
SYNCORE_MODE=mcp                               # Default: mcp
TRANSPORT=stdio                                # Default: stdio

# Logging
RUST_LOG=info                                  # Default: info
LOG_DIR=./logs                                 # Default: ./logs

# Metrics
METRICS_ADDR=127.0.0.1:9090                   # Default: 127.0.0.1:9090
```

---

## Appendix D: Quick Reference Card

```
LOAD CONTEXT                     STORE CONTEXT
memory_query("key")              memory_store("key", "value")
intellitask_list()               memory_store("session_state", json)
logs_tail(50)                    vector_insert(insight, metadata)

SEARCH CODE                      INDEX CODE
code_graph_fusion_query()        code_index(file_path)
code_search(query, limit)        code_index_directory(dir, pattern)
parser_search(pattern)           code_graph_sync_neo4j()

ANALYZE CODE                     MODIFY CODE
parser_analyze(file_path)        application_record(file, "modify", ...)
graph_query(cypher)              [Use external tools]

TASK MANAGEMENT                  REASONING
intellitask_next_ready()         sequential_cycle(max_cycles)
intellitask_update_status()      sequential_record(step, thought, ...)
task_create(goal, priority)      sequential_search(query)

GRAPH QUERIES                    DOCUMENTS
graph_query(cypher, params)      document_index(directory)
raggraph_multihop(seed_nodes)    document_search(query, limit)
```

---

**End of Manual**

For questions or issues, refer to:
- Project README: `/home/feanor/Projects/SynCore/syncore/README.md`
- Claude Instructions: `/home/feanor/Projects/SynCore/CLAUDE.md`
- Source Code: `/home/feanor/Projects/SynCore/syncore/src/`

**Last Updated**: 2025-11-21
**Version**: R5.0
**Total Tools**: 67
