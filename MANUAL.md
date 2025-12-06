# SynCore User Manual

Practical guide to using SynCore MCP tools with real examples.

**Last Updated**: December 2025
**Version**: 0.2.0
**Status**: Production-Ready Core Features, Experimental AI Features

## Table of Contents

1. [Memory Tools](#memory-tools)
2. [Vector Search](#vector-search)
3. [Code Intelligence](#code-intelligence)
4. [Parser Tools](#parser-tools)
5. [Graph Database](#graph-database)
6. [Code Graph & Fusion](#code-graph--fusion)
7. [Task Management](#task-management)
8. [Agent Coordination](#agent-coordination)
9. [Application Mapping](#application-mapping)
10. [Sequential Reasoning](#sequential-reasoning)
11. [Application Change Tracking](#application-change-tracking)
12. [Project Analysis](#project-analysis)
13. [Meta-Tools](#meta-tools)
14. [Common Workflows](#common-workflows)
15. [Troubleshooting](#troubleshooting)

---

## Memory Tools

Persistent key-value storage that survives across sessions. Uses SQLite for persistence + Sled for fast caching.

### memory_store

Store any value with a key. Overwrites existing values.

```
Tool: memory_store
Parameters:
  key: "project_context"
  value: "Working on authentication module refactoring"
  dry_run: false  # optional, if true doesn't actually store
```

**Response:**
```json
{
  "key": "project_context",
  "stored": true
}
```

### memory_query

Retrieve a stored value.

```
Tool: memory_query
Parameters:
  key: "project_context"
```

**Response:**
```json
{
  "found": true,
  "value": "Working on authentication module refactoring"
}
```

**Practical Example: Session Continuity**

At the end of a session:
```
memory_store(key="last_task", value="Implementing user validation")
memory_store(key="blockers", value="Need to add rate limiting")
memory_store(key="files_modified", value="src/auth.rs, src/validation.rs")
```

At the start of next session:
```
memory_query(key="last_task")      # Returns what you were working on
memory_query(key="blockers")       # Returns open issues
memory_query(key="files_modified") # Returns which files changed
```

---

## Vector Search

Semantic search using fastembed embeddings (384 dimensions). Finds similar content by meaning, not exact keywords.

### vector_insert

Add text to the vector store.

```
Tool: vector_insert
Parameters:
  text: "The authentication module handles user login and session management"
  dry_run: false  # optional
  metadata: {}    # optional JSON metadata
```

**Response:**
```json
{
  "inserted": true,
  "vector_id": 35042
}
```

### vector_search

Find semantically similar content.

```
Tool: vector_search
Parameters:
  query: "user login system"
  limit: 5  # optional, default 10
```

**Response:**
```json
{
  "count": 3,
  "results": [
    {"id": 35042, "score": 0.85, "text": "The authentication module handles..."},
    {"id": 12003, "score": 0.72, "text": "Session tokens are validated..."},
    {"id": 8891, "score": 0.65, "text": "Password hashing uses bcrypt..."}
  ]
}
```

**Note:** Scores closer to 1.0 are better matches. TF-IDF embeddings work well for code/technical content but may miss nuanced semantic relationships that transformer models would catch.

---

## Code Intelligence

Index and search code using semantic understanding. Supports incremental indexing (PHASE 5).

### code_index

Index a source file for semantic search. **Incremental:** Returns 0 if file is unchanged.

```
Tool: code_index
Parameters:
  file_path: "/path/to/src/auth.rs"
```

**Response (first time):**
```json
{
  "indexed": true,
  "entities": 15,
  "file_path": "/path/to/src/auth.rs"
}
```

**Response (unchanged file):**
```json
{
  "indexed": true,
  "entities": 0,
  "file_path": "/path/to/src/auth.rs",
  "skipped": "unchanged"
}
```

### code_search

Find code by semantic meaning.

```
Tool: code_search
Parameters:
  query: "password validation logic"
  limit: 5
```

**Response:**
```json
{
  "count": 3,
  "results": [
    {
      "id": 8891,
      "score": 0.78,
      "entity_type": "Function",
      "name": "validate_password",
      "file_path": "/path/src/auth.rs",
      "line_start": 45
    }
  ]
}
```

### code_index_directory

Batch index all code files matching a pattern. **Incremental:** Only processes changed files.

```
Tool: code_index_directory
Parameters:
  directory: "/path/to/src"
  pattern: "*.rs"
```

**Response:**
```json
{
  "indexed": true,
  "files_processed": 45,
  "files_skipped": 120,
  "total_entities": 1250,
  "new_entities": 35
}
```

---

## Parser Tools

Tree-sitter based code analysis. Supports: Rust, JavaScript, Python, JSON, TOML, Bash.

### parser_analyze

Extract AST structure from code. **Key feature:** `persist=true` writes to SQLite, HNSW, and Neo4j.

```
Tool: parser_analyze
Parameters:
  file_path: "/path/to/src/auth.rs"
  persist: true  # IMPORTANT: Set true to index entities
```

**Response:**
```json
{
  "file_path": "/path/to/src/auth.rs",
  "language": "rust",
  "entities": [
    {
      "kind": "function",
      "name": "validate_password",
      "line_start": 45,
      "line_end": 67,
      "visibility": "pub"
    },
    {
      "kind": "struct",
      "name": "AuthConfig",
      "line_start": 12,
      "line_end": 20
    }
  ],
  "imports": ["std::collections::HashMap", "crate::crypto"],
  "persisted": true,
  "entities_indexed": 15
}
```

### parser_search

Search code patterns using ripgrep.

```
Tool: parser_search
Parameters:
  pattern: "fn.*validate"
  path: "/path/to/src"  # optional
  context_lines: 3      # optional lines before/after match
```

**Response:**
```json
{
  "matches": [
    {
      "file": "/path/src/auth.rs",
      "line": 45,
      "content": "pub fn validate_password(input: &str) -> Result<bool> {",
      "context_before": ["/// Validates password strength", "///"],
      "context_after": ["    if input.len() < 8 {"]
    }
  ],
  "count": 5
}
```

---

## Graph Database

Neo4j integration for knowledge graph operations. **Requires Neo4j running.**

### graph_query

Execute a Cypher read query.

```
Tool: graph_query
Parameters:
  cypher: "MATCH (n:CodeEntity) RETURN count(n) as total"
  params: {}  # optional parameters
```

**Response:**
```json
{
  "results": [{"total": 2145}]
}
```

**Useful Queries:**

```cypher
-- Count entities by type
MATCH (n:CodeEntity) RETURN n.entity_type, count(n) ORDER BY count(n) DESC

-- Find functions that call other functions
MATCH (f:CodeEntity)-[:CALLS]->(g:CodeEntity)
WHERE f.entity_type = 'Function'
RETURN f.name, collect(g.name) as calls LIMIT 10

-- Count relationship types
MATCH ()-[r]->() RETURN type(r), count(r) ORDER BY count(r) DESC
```

### graph_insert

Execute a Cypher write query.

```
Tool: graph_insert
Parameters:
  cypher: "CREATE (n:Concept {name: 'Authentication', description: 'User identity verification'})"
```

### graph_relate

Create a relationship between nodes by ID.

```
Tool: graph_relate
Parameters:
  from_id: 123
  to_id: 456
  rel_type: "DEPENDS_ON"
  from_label: "CodeEntity"  # optional
  to_label: "CodeEntity"    # optional
```

---

## Code Graph & Fusion

Advanced search combining vector similarity with graph relationships.

### code_graph_fusion_query

Tri-mode search with automatic mode selection:
- **Simple**: Vector-only search (fast)
- **Attention**: Weighted vector + graph scores
- **Reasoning**: Multi-hop graph traversal (thorough)

```
Tool: code_graph_fusion_query
Parameters:
  query: "HNSW vector index implementation"
  top_k: 5
  mode_hint: null         # or "simple", "attention", "reasoning"
  scope: "global"         # "local", "project", "workspace", "global", "auto"
  project_label: null     # filter by project
  local_root: null        # filter by path prefix
  namespace: null         # filter by namespace
```

**Response:**
```json
{
  "entities": [
    {
      "entity": {
        "id": 74339,
        "file_path": "/path/to/src/vector.rs",
        "entity_type": "Function",
        "name": "search_hnsw",
        "line_start": 245
      },
      "combined_score": 0.85,
      "vector_score": 0.78,
      "graph_score": 0.12
    }
  ],
  "selected_mode": "attention",
  "applied_scope": "global"
}
```

### code_graph_sync_neo4j

Sync entities from SQLite to Neo4j. Run after indexing to populate graph.

```
Tool: code_graph_sync_neo4j
Parameters:
  limit: 100       # optional, entities per batch
  namespace: null  # optional filter
```

**Response:**
```json
{
  "synced": true,
  "entities_synced": 100,
  "relationships_synced": 350
}
```

### code_graph_enrich_temporal

Add git history and filesystem metadata to entities.

```
Tool: code_graph_enrich_temporal
Parameters:
  only_missing: true  # only enrich entities without temporal data
  limit: null         # optional limit
```

### raggraph_query

RAG query with multi-hop graph reasoning.

```
Tool: raggraph_query
Parameters:
  query_text: "How does authentication work?"
```

**Response:**
```json
{
  "results": [...],
  "reasoning_path": [...]
}
```

### raggraph_multihop

Execute multi-hop graph diffusion from seed nodes.

```
Tool: raggraph_multihop
Parameters:
  seed_nodes: [74339, 74340, 74341]  # entity IDs
```

---

## Task Management

Track tasks with priorities and relationships.

### task_create

Create a new task.

```
Tool: task_create
Parameters:
  goal: "Implement rate limiting for API endpoints"
  priority: 1  # optional, 1=highest
```

### intellitask_list

List all tasks with optional filtering.

```
Tool: intellitask_list
Parameters:
  status: null     # optional: "open", "done", "in_progress"
  parent_id: null  # optional: filter by parent task
  prd_title: null  # optional: filter by PRD
```

### intellitask_get

Get task by ID.

```
Tool: intellitask_get
Parameters:
  task_id: 15
```

### intellitask_update_status

Update a task's status.

```
Tool: intellitask_update_status
Parameters:
  task_id: 15
  status: "done"  # "open", "done", "in_progress", "blocked"
```

### intellitask_next_ready

Get next task ready to work on (all dependencies satisfied).

```
Tool: intellitask_next_ready
Parameters: {}
```

### intellitask_get_subtasks

Get subtasks for a parent task.

```
Tool: intellitask_get_subtasks
Parameters:
  parent_id: 15
```

### intellitask_subtask_stats

Get subtask statistics.

```
Tool: intellitask_subtask_stats
Parameters:
  parent_id: 15
```

### intellitask_task_statistics

Get overall task statistics.

```
Tool: intellitask_task_statistics
Parameters: {}
```

### intellitask_prd_statistics

Get statistics for a specific PRD.

```
Tool: intellitask_prd_statistics
Parameters:
  prd_title: "Authentication System"
```

### intellitask_save

Save task breakdown to database.

```
Tool: intellitask_save
Parameters:
  breakdown_json: "{...}"  # JSON task breakdown
```

### AI-Powered Tools (Require Ollama)

These tools require Ollama running locally:

**intellitask_generate** - Generate task breakdown from PRD
```
Tool: intellitask_generate
Parameters:
  prd_content: "## Feature: User Authentication\n\n..."
```

**intellitask_subtasks** - Generate subtasks for a task
```
Tool: intellitask_subtasks
Parameters:
  parent_task_id: "15"
  parent_task_json: "{...}"
  codebase_context: null  # optional
```

**intellitask_prioritize** - AI task prioritization
```
Tool: intellitask_prioritize
Parameters:
  tasks_json: "[{...}, {...}]"
  business_context: null  # optional
```

**intellitask_next** - AI next task suggestion
```
Tool: intellitask_next
Parameters:
  completed_tasks: ["task1", "task2"]
  remaining_tasks_json: "[{...}]"
```

---

## Agent Coordination

Message bus for multi-agent workflows.

### agent_register

Register an agent with capabilities.

```
Tool: agent_register
Parameters:
  id: "code_reviewer"
  capabilities: ["review", "suggest", "refactor"]
```

### agent_send

Send message to another agent.

```
Tool: agent_send
Parameters:
  to: "code_reviewer"
  message: "Please review src/auth.rs"
```

### agent_recv

Receive pending messages for an agent.

```
Tool: agent_recv
Parameters:
  agent: "code_reviewer"
```

### agent_poll

Wait for next message (blocking with timeout).

```
Tool: agent_poll
Parameters:
  agent: "code_reviewer"
  timeout_ms: 5000
```

### agent_list

List all registered agents.

```
Tool: agent_list
Parameters: {}
```

### agent_status

Update agent status.

```
Tool: agent_status
Parameters:
  id: "code_reviewer"
  status: {"state": "busy", "current_task": "reviewing auth.rs"}
```

### agent_task

Send structured task envelope.

```
Tool: agent_task
Parameters:
  to: "code_reviewer"
  task_id: "review_123"
  task_type: "code_review"
  payload: {"file": "src/auth.rs", "focus": "security"}
```

### agent_result

Submit completed task result.

```
Tool: agent_result
Parameters:
  from: "code_reviewer"
  task_id: "review_123"
  result: {"approved": true, "comments": [...]}
```

---

## Application Mapping

Track file structure, imports, exports, and dependencies.

### mapping_record

Record a file node with its relationships.

```
Tool: mapping_record
Parameters:
  path: "src/auth.rs"
  kind: "module"
  language: "rust"
  imports: ["std::collections", "crate::crypto"]
  exports: ["validate_password", "AuthConfig"]
  dependencies: ["src/crypto.rs"]
```

### mapping_get

Get a file node.

```
Tool: mapping_get
Parameters:
  path: "src/auth.rs"
```

### mapping_search

Search files by semantic query.

```
Tool: mapping_search
Parameters:
  query: "authentication related files"
```

### mapping_deps

Get all transitive dependencies for a file.

```
Tool: mapping_deps
Parameters:
  path: "src/auth.rs"
```

---

## Sequential Reasoning

Record and search reasoning chains.

### sequential_record

Record a thought step.

```
Tool: sequential_record
Parameters:
  step_number: 1
  thought: "Need to understand the authentication flow"
  reasoning: "Start by mapping the entry points"
  task_id: 15    # optional
  action: null   # optional
  observation: null  # optional
```

### sequential_get

Get all thought steps for a task.

```
Tool: sequential_get
Parameters:
  task_id: 15
```

### sequential_search

Search thought steps by content.

```
Tool: sequential_search
Parameters:
  query: "authentication flow analysis"
```

### sequential_cycle (Requires Ollama)

Run full reasoning cycle with LLM.

```
Tool: sequential_cycle
Parameters:
  max_cycles: 5  # optional
```

---

## Application Change Tracking

Record and search code changes.

### application_record

Record a code change.

```
Tool: application_record
Parameters:
  file_path: "src/auth.rs"
  change_type: "modify"  # "add", "modify", "delete"
  line_start: 45
  line_end: 67
  description: "Added password strength validation"
  old_content: null  # optional
  new_content: null  # optional
  task_id: 15       # optional link to task
```

### application_get

Get all changes for a task.

```
Tool: application_get
Parameters:
  task_id: 15
```

### application_history

Get change history for a file.

```
Tool: application_history
Parameters:
  file_path: "src/auth.rs"
```

### application_search

Search changes by content.

```
Tool: application_search
Parameters:
  query: "password validation changes"
```

---

## Project Analysis

LLM-free, deterministic codebase intelligence tools. All read-only - no database modifications.

### project_file_report

Generate a comprehensive report for a single file including entities, relationships, imports, and metrics.

```
Tool: project_file_report
Parameters:
  file_path: "/path/to/src/auth.rs"
```

**Response:**
```json
{
  "ok": true,
  "data": {
    "file_path": "/path/to/src/auth.rs",
    "loc": 245,
    "entities": [
      {"id": 123, "name": "validate_password", "entity_type": "Function", "line_start": 45, "line_end": 67}
    ],
    "calls_out": [
      {"src_entity_name": "validate_password", "dst_entity_name": "hash_check", "edge_type": "calls"}
    ],
    "calls_in": [...],
    "imports": [
      {"module": "use std::collections::HashMap;", "line": 1}
    ],
    "uses": [...],
    "metrics": {"fan_in": 5, "fan_out": 3, "entity_count": 12}
  }
}
```

### project_hotspots

Find code complexity hotspots - files with high coupling, many entities, or high LOC.

```
Tool: project_hotspots
Parameters:
  limit: 10
  min_fan_in: 5      # optional
  min_fan_out: 5     # optional
  min_loc: 100       # optional
  min_entity_count: 5  # optional
```

**Response:**
```json
{
  "ok": true,
  "data": {
    "hotspots": [
      {
        "file_path": "/path/to/src/vector.rs",
        "fan_in": 78,
        "fan_out": 59,
        "entity_count": 100,
        "loc": 1676,
        "score": 236.5
      }
    ]
  }
}
```

### project_dead_code

Identify potentially unused entities (no incoming references).

```
Tool: project_dead_code
Parameters:
  exclude_public: true  # optional, default true - excludes pub items
  limit: 20            # optional
```

**Response:**
```json
{
  "ok": true,
  "data": {
    "dead_entities": [
      {
        "id": 456,
        "name": "unused_helper",
        "entity_type": "Function",
        "file_path": "/path/to/src/utils.rs",
        "visibility": "private",
        "line_start": 89
      }
    ]
  }
}
```

### project_cycles

Detect circular dependencies between files.

```
Tool: project_cycles
Parameters:
  max_cycles: 10   # maximum cycles to return
  max_depth: 5     # maximum cycle length
```

**Response:**
```json
{
  "ok": true,
  "data": {
    "cycles": [
      {
        "files": ["src/a.rs", "src/b.rs", "src/a.rs"],
        "relation_kinds": ["imports", "imports"],
        "cycle_length": 2
      }
    ]
  }
}
```

### project_unused_imports

Find imports that aren't actually used in the code.

```
Tool: project_unused_imports
Parameters:
  file_path: null  # optional - specific file or all
  limit: 20        # optional
```

**Response:**
```json
{
  "ok": true,
  "data": {
    "unused_imports": [
      {
        "file_path": "/path/to/src/handlers.rs",
        "import_name": "std::io::Write",
        "line": 5,
        "module": "std::io"
      }
    ]
  }
}
```

### project_module_map

Generate a module-level dependency map of the project.

```
Tool: project_module_map
Parameters:
  root: null        # optional - start from specific path
  max_modules: 50   # optional
```

**Response:**
```json
{
  "ok": true,
  "data": {
    "modules": [
      {
        "id": "src/vector.rs",
        "file_path": "/path/to/src/vector.rs",
        "entity_count": 100,
        "fan_in": 78,
        "fan_out": 59,
        "loc": 1676
      }
    ],
    "edges": [
      {"from_file": "src/mcp_server.rs", "to_file": "src/vector.rs", "relationship_type": "imports"}
    ]
  }
}
```

### project_refactor_suggestions

Generate heuristic-based refactoring suggestions.

```
Tool: project_refactor_suggestions
Parameters:
  limit: 10
  loc_threshold: 300          # optional - suggest split above this LOC
  fan_out_threshold: 20       # optional - suggest facade above this
  fan_in_threshold: 30        # optional
  entity_threshold: 30        # optional
```

**Response:**
```json
{
  "ok": true,
  "data": {
    "suggestions": [
      {
        "kind": "SplitFile",
        "description": "Split /path/to/src/vector.rs (100 entities, ~1676 LOC) into smaller, focused modules",
        "file_path": "/path/to/src/vector.rs",
        "related_files": null,
        "metrics": {"loc": 1676, "entity_count": 100}
      },
      {
        "kind": "PruneDeadCode",
        "description": "Remove 22 unused entities from /path/to/src/vector.rs",
        "file_path": "/path/to/src/vector.rs",
        "metrics": {"dead_entities": 22}
      }
    ]
  }
}
```

**Suggestion Kinds:**
- `SplitFile` - File is too large, split into modules
- `ExtractFacade` - High fan-out, extract a facade/interface
- `ReduceCycle` - Circular dependency detected
- `PruneDeadCode` - Unused code found
- `SimplifyDependency` - Complex dependency pattern

---

## Meta-Tools

Meta-tools aggregate data from multiple PAE analysis tools into unified, actionable reports. All are read-only (no database modifications).

### project_architecture_overview

Get a comprehensive project-wide summary including entity counts, dependency statistics, and top files by coupling.

```
Tool: project_architecture_overview
Parameters: {}
```

**Response:**
```json
{
  "ok": true,
  "data": {
    "total_files": 45,
    "total_entities": 2145,
    "entity_breakdown": {
      "Function": 890,
      "Struct": 234,
      "Impl": 156,
      "Const": 89
    },
    "total_edges": 4521,
    "edge_breakdown": {
      "calls": 1234,
      "imports": 890,
      "contains": 567
    },
    "top_files_by_fan_in": [
      {"file_path": "src/vector.rs", "fan_in": 78}
    ],
    "top_files_by_fan_out": [
      {"file_path": "src/mcp_server.rs", "fan_out": 89}
    ],
    "avg_fan_in": 5.2,
    "avg_fan_out": 4.8,
    "max_loc": 1676,
    "total_loc": 15420
  }
}
```

### project_complexity_dashboard

Get a complexity-focused dashboard with hotspots, cycles, and a computed health score.

```
Tool: project_complexity_dashboard
Parameters:
  hotspot_limit: 10     # optional, default 10
  cycle_limit: 5        # optional, default 5
```

**Response:**
```json
{
  "ok": true,
  "data": {
    "health_score": 72.5,
    "health_grade": "C",
    "top_hotspots": [
      {
        "file_path": "src/vector.rs",
        "score": 236.5,
        "fan_in": 78,
        "fan_out": 59,
        "entity_count": 100,
        "loc": 1676
      }
    ],
    "worst_cycles": [
      {
        "files": ["src/a.rs", "src/b.rs"],
        "cycle_length": 2
      }
    ],
    "complexity_by_file": [
      {"file_path": "src/vector.rs", "complexity_score": 236.5}
    ],
    "summary": {
      "files_analyzed": 45,
      "hotspot_count": 12,
      "cycle_count": 3,
      "avg_complexity": 45.2
    }
  }
}
```

**Health Score Grades:**
- A (90-100): Excellent - few hotspots, no cycles
- B (75-89): Good - some complexity but manageable
- C (60-74): Fair - notable complexity issues
- D (40-59): Poor - significant refactoring needed
- F (0-39): Critical - major architectural problems

### project_improvement_roadmap

Get a prioritized list of improvements with effort/impact analysis and category breakdown.

```
Tool: project_improvement_roadmap
Parameters:
  limit_per_category: 20      # optional
  high_priority_only: false   # optional
  hotspot_loc_threshold: 100  # optional
  project_label: null         # optional
```

**Response:**
```json
{
  "ok": true,
  "data": {
    "summary": {
      "total_improvements": 45,
      "by_priority": {"Critical": 3, "High": 12, "Medium": 20, "Low": 10},
      "by_type": {"RemoveDeadCode": 15, "BreakCycle": 3, "ReduceComplexity": 8},
      "estimated_total_effort": 22.5,
      "files_affected": 28
    },
    "improvements": [
      {
        "id": "cycle_0",
        "improvement_type": "BreakCycle",
        "priority": "Critical",
        "file_path": "src/a.rs",
        "line_number": null,
        "description": "Break circular dependency in 2 files",
        "effort": 4,
        "impact": 5,
        "metadata": {"cycle_length": 2, "files": ["src/a.rs", "src/b.rs"]}
      }
    ],
    "by_category": {
      "dead_code": [...],
      "unused_imports": [...],
      "refactor_suggestions": [...],
      "cycle_fixes": [...],
      "complexity_reductions": [...]
    },
    "effort_impact_matrix": {
      "quick_wins": [...],
      "major_projects": [...],
      "fill_ins": [...],
      "reconsider": [...]
    }
  }
}
```

**Effort/Impact Matrix:**
- `quick_wins`: Low effort (≤2), high impact (≥4) - do these first
- `major_projects`: High effort (≥4), high impact (≥4) - plan carefully
- `fill_ins`: Low effort (≤2), low impact (≤2) - do when time permits
- `reconsider`: High effort (≥4), low impact (≤2) - likely not worth it

### project_refactor_action_plan

Get an actionable refactoring plan with specific items to address.

```
Tool: project_refactor_action_plan
Parameters: {}
```

**Response:**
```json
{
  "ok": true,
  "data": {
    "high_risk_hotspots": [
      {
        "file_path": "src/vector.rs",
        "score": 236.5,
        "fan_in": 78,
        "fan_out": 59,
        "entity_count": 100,
        "loc": 1676
      }
    ],
    "dead_code_cleanup": [
      {
        "id": 456,
        "name": "unused_helper",
        "entity_type": "Function",
        "file_path": "src/utils.rs",
        "line_start": 89
      }
    ],
    "unused_imports": [
      {
        "file_path": "src/handlers.rs",
        "import_name": "std::io::Write",
        "line": 5,
        "module": "std::io"
      }
    ],
    "cycle_break_candidates": [
      {"file_path": "src/a.rs"},
      {"file_path": "src/b.rs"}
    ],
    "module_refactor_ops": [
      {
        "file_path": "src/vector.rs",
        "operation": "split",
        "loc": 1676,
        "entity_count": 100,
        "reason": "Module exceeds 500 LOC (actual: 1676)"
      },
      {
        "file_path": "src/tiny.rs",
        "operation": "merge_candidate",
        "loc": 45,
        "entity_count": 3,
        "reason": "Small module with 45 LOC and 3 entities"
      }
    ]
  }
}
```

**Action Plan Thresholds:**
- `high_risk_hotspots`: Files with complexity score ≥ 100
- `dead_code_cleanup`: All detected unused entities
- `unused_imports`: Top 10 unused imports
- `cycle_break_candidates`: All files involved in circular dependencies
- `module_refactor_ops`:
  - `split`: Files with LOC > 500
  - `merge_candidate`: Files with LOC < 100 AND entity_count < 5

---

## Common Workflows

### Workflow 1: Start a New Session

```
# 1. Load previous context
memory_query(key="current_project")
memory_query(key="last_task")
memory_query(key="blockers")

# 2. Check task status
intellitask_list()
intellitask_next_ready()

# 3. Find relevant code
code_graph_fusion_query(query="<what you're working on>", top_k=10)
```

### Workflow 2: Index a Codebase (First Time)

```
# 1. Index all code files
code_index_directory(directory="/path/to/src", pattern="*.rs")

# 2. Sync to Neo4j for graph queries
code_graph_sync_neo4j(limit=500)

# 3. Enrich with git history
code_graph_enrich_temporal(only_missing=true)

# 4. Verify
graph_query(cypher="MATCH (n:CodeEntity) RETURN count(n)")
```

### Workflow 3: Incremental Re-index (After Changes)

```
# Just run code_index_directory again - it skips unchanged files
code_index_directory(directory="/path/to/src", pattern="*.rs")
# Response shows: files_skipped: 120, files_processed: 3
```

### Workflow 4: Understand Unfamiliar Code

```
# 1. Search by concept
code_search(query="the feature you're looking for", limit=10)

# 2. Get deeper context with fusion
code_graph_fusion_query(query="<specific functionality>", top_k=5)

# 3. Query relationships
graph_query(cypher="MATCH (f:CodeEntity)-[:CALLS]->(g) WHERE f.name CONTAINS 'auth' RETURN f.name, g.name")

# 4. Parse specific file for structure
parser_analyze(file_path="/path/to/interesting/file.rs", persist=false)
```

### Workflow 5: Document Your Work

```
# 1. Store what you learned
vector_insert(text="The rate limiter uses token bucket algorithm in src/ratelimit.rs")

# 2. Record reasoning
sequential_record(step_number=1, thought="Analyzed rate limiter", reasoning="Token bucket chosen for burst handling")

# 3. Save session state
memory_store(key="session_notes", value="Completed rate limiter, needs testing")
memory_store(key="next_steps", value="Add unit tests for edge cases")

# 4. Create follow-up task
task_create(goal="Test rate limiter edge cases", priority=1)
```

### Workflow 6: Multi-Agent Collaboration

```
# 1. Register agents
agent_register(id="planner", capabilities=["plan", "decompose"])
agent_register(id="coder", capabilities=["implement", "refactor"])
agent_register(id="reviewer", capabilities=["review", "test"])

# 2. Send task to planner
agent_task(to="planner", task_id="feature_1", task_type="plan", payload={"feature": "auth"})

# 3. Planner submits result
agent_result(from="planner", task_id="feature_1", result={"subtasks": [...]})

# 4. Route to coder
agent_task(to="coder", task_id="impl_1", task_type="implement", payload={"subtask": "..."})
```

### Workflow 7: Codebase Health Check

```
# 1. Find complexity hotspots
project_hotspots(limit=10)
# Returns: Top files by coupling and size

# 2. Check for dead code
project_dead_code(limit=20)
# Returns: Unused entities to potentially remove

# 3. Detect circular dependencies
project_cycles(max_cycles=10, max_depth=5)
# Returns: Files with circular imports

# 4. Find unused imports
project_unused_imports(limit=20)
# Returns: Imports that can be removed

# 5. Get refactoring suggestions
project_refactor_suggestions(limit=10)
# Returns: Actionable suggestions like "Split vector.rs", "Prune 22 dead entities"
```

### Workflow 8: Analyze a Specific File

```
# Get full report for a file
project_file_report(file_path="/path/to/src/complex_module.rs")
# Returns:
# - entities: All functions, structs, etc.
# - calls_out: What this file depends on
# - calls_in: What depends on this file
# - imports: Import statements
# - metrics: fan_in, fan_out, entity_count
# - loc: Lines of code estimate
```

---

## Troubleshooting

### Vector search returns irrelevant results
- Try more specific queries with technical terms
- Embeddings are TF-IDF based - exact technical terms work better than natural language
- Check if content was actually indexed: `vector_search(query="exact phrase from indexed content")`

### Graph queries fail
- Check Neo4j is running: `systemctl status neo4j`
- Verify credentials: `NEO4J_URI`, `NEO4J_USER`, `NEO4J_PASS`
- Test connection: `graph_query(cypher="RETURN 1")`

### code_index returns 0 entities
- File unchanged since last index (incremental behavior)
- To force re-index, modify file or clear `file_index_state` table

### IntelliTask tools fail
- Candle GGUF models must be available: check models directory contains .gguf files
- Ensure GGUF model files are valid and accessible

### Slow startup
- First startup loads embedding model (~500MB)
- Subsequent startups use HNSW snapshot (fast)
- Cold HNSW falls back to brute-force search (slower but works)

### Tasks not persisting
- Check `DB_PATH` is set and directory is writable
- Database is SQLite, survives restarts

### Agent messages not received
- Agents must be registered first
- Check agent ID matches exactly
- Messages are not persisted - agents must poll while running

---

## Tool Cost Reference

| Cost | Meaning | Examples |
|------|---------|----------|
| Low | <10ms, no I/O | memory_query, task_create |
| Medium | 10-100ms, local computation | vector_search, parser_analyze |
| High | 100ms-1s, network/disk I/O | graph_query, code_index |
| Very High | >1s, batch operations | code_index_directory, document_index |

---

## Environment Variables Reference

| Variable | Default | Required | Description |
|----------|---------|----------|-------------|
| `DB_PATH` | `syncore.db` | No | SQLite database path |
| `HTTP_PORT` | `3001` | No | HTTP streaming port |
| `NEO4J_URI` | `bolt://127.0.0.1:7687` | For graph | Neo4j connection |
| `NEO4J_USER` | `neo4j` | For graph | Neo4j username |
| `NEO4J_PASS` | - | For graph | Neo4j password |
| `RUST_LOG` | `info` | No | Log level (debug, info, warn, error) |
