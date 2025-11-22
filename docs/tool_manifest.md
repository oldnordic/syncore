# SynCore MCP Tool Manifest

**Version**: 1.0
**SynCore Version**: 0.1.0
**Generated**: 2025-01-20
**MCP Protocol Version**: 2024-11-05

## Overview

This document provides the authoritative reference for all 50 tools exposed through SynCore's Model Context Protocol (MCP) server.

### Architecture

- **Total Tools**: 50
- **Delegated to RealExecutor**: 42 tools
- **Custom MCP Implementations**: 8 tools
- **Tool Categories**: 11

### Delegation Pattern

42 tools use the unified delegation pattern:
```rust
async fn tool_name(&self, Parameters(params): Parameters<ToolRequest>)
    -> Result<CallToolResult, McpError>
{
    self.mcp_delegate("tool_name", serde_json::json!({
        "param1": params.param1,
        "param2": params.param2
    })).await
}
```

This delegates to `RealExecutor.execute_real_tool_async()` which returns standardized error envelopes.

### Error Envelope Format

All delegated tools return JSON envelopes:

**Success**:
```json
{
  "ok": true,
  "data": { /* result data */ },
  "tool": "tool_name",
  "executor": "RealExecutor"
}
```

**Error**:
```json
{
  "ok": false,
  "error": {
    "message": "Error description",
    "code": "ERROR_CODE",
    "details": "Optional additional details"
  },
  "tool": "tool_name",
  "executor": "RealExecutor"
}
```

---

## Tool Categories

### 1. Memory Tools (2 tools)

Persistent key-value storage using hybrid SQLite + Sled cache.

#### memory_store

**Description**: Store a value in memory
**Delegated**: ✅ Yes
**Async**: Yes

**Parameters**:
| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| key | string | ✅ | - | Memory key identifier |
| value | string | ✅ | - | Value to store |
| dry_run | boolean | ❌ | false | If true, validate without persisting |

**Returns**:
```json
{
  "success": true,
  "key": "example_key",
  "message": "Value stored successfully"
}
```

---

#### memory_query

**Description**: Query a value from memory
**Delegated**: ✅ Yes
**Async**: Yes

**Parameters**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| key | string | ✅ | Memory key to retrieve |

**Returns**:
```json
{
  "key": "example_key",
  "value": "stored value",
  "found": true
}
```

---

### 2. Task Tools (1 tool)

SQLite-backed task management with parent-child hierarchy.

#### task_create

**Description**: Create a new task
**Delegated**: ✅ Yes
**Async**: Yes

**Parameters**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| goal | string | ✅ | Task goal description |
| priority | integer | ❌ | Task priority (1-5) |

**Returns**:
```json
{
  "task_id": 123,
  "goal": "Task description",
  "status": "pending"
}
```

---

### 3. Vector Tools (2 tools)

Semantic search using 384-dimensional embeddings with linear scan.

#### vector_insert

**Description**: Insert text into vector memory
**Delegated**: ✅ Yes
**Async**: Yes

**Parameters**:
| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| text | string | ✅ | - | Text to vectorize and store |
| metadata | object | ❌ | null | Optional metadata JSON |
| dry_run | boolean | ❌ | false | If true, validate without persisting |

**Returns**:
```json
{
  "success": true,
  "vector_id": 456,
  "dimensions": 384
}
```

---

#### vector_search

**Description**: Search vector memory
**Delegated**: ✅ Yes
**Async**: Yes

**Parameters**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| query | string | ✅ | Search query text |
| limit | integer | ❌ | Maximum results to return |

**Returns**:
```json
{
  "results": [
    {
      "text": "matching text",
      "score": 0.95,
      "metadata": {}
    }
  ],
  "count": 1
}
```

---

### 4. Log Tools (1 tool)

Structured log retrieval.

#### logs_tail

**Description**: Get recent log entries
**Delegated**: ✅ Yes
**Async**: Yes

**Parameters**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| n | integer | ❌ | Number of log lines to return |

**Returns**:
```json
[
  "[INFO] Log line 1",
  "[DEBUG] Log line 2"
]
```

---

### 5. Sequential Tools (4 tools)

Multi-step reasoning and thought chain management.

#### sequential_cycle

**Description**: Run sequential thinking cycles for complex task processing
**Delegated**: ✅ Yes
**Async**: Yes

**Parameters**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| max_cycles | integer | ❌ | Maximum reasoning cycles to execute |

**Returns**:
```json
{
  "cycles_completed": 5,
  "final_state": "converged",
  "steps": []
}
```

---

#### sequential_record

**Description**: Record a thought step in the reasoning chain
**Delegated**: ✅ Yes
**Async**: Yes

**Parameters**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| task_id | integer | ❌ | Associated task ID |
| step_number | integer | ✅ | Step number in sequence |
| thought | string | ✅ | Thought content |
| reasoning | string | ✅ | Reasoning explanation |
| action | string | ❌ | Action taken |
| observation | string | ❌ | Observation from action |

**Returns**:
```json
{
  "success": true,
  "step_id": 789
}
```

---

#### sequential_get

**Description**: Get all thought steps for a task
**Delegated**: ✅ Yes
**Async**: Yes

**Parameters**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| task_id | integer | ✅ | Task ID |

**Returns**:
```json
{
  "steps": [],
  "count": 5
}
```

---

#### sequential_search

**Description**: Search thought steps by semantic content
**Delegated**: ✅ Yes
**Async**: Yes

**Parameters**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| query | string | ✅ | Search query |

**Returns**:
```json
{
  "results": [],
  "count": 3
}
```

---

### 6. Parser Tools (2 tools)

Tree-sitter based code analysis supporting Rust, JavaScript, Python, JSON, TOML, Bash.

#### parser_analyze

**Description**: Analyze code structure using tree-sitter parser
**Delegated**: ✅ Yes
**Async**: Yes

**Parameters**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| file_path | string | ✅ | Path to source file to analyze |

**Returns**:
```json
{
  "language": "rust",
  "functions": [],
  "classes": [],
  "imports": []
}
```

---

#### parser_search

**Description**: Search code patterns using ripgrep
**Delegated**: ✅ Yes
**Async**: Yes

**Parameters**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| pattern | string | ✅ | Regex pattern to search |
| path | string | ❌ | Directory or file path to search |
| context_lines | integer | ❌ | Number of context lines around matches |

**Returns**:
```json
{
  "matches": [
    {
      "file": "src/main.rs",
      "line": 42,
      "text": "matching code"
    }
  ],
  "count": 1
}
```

---

### 7. Code Tools (3 tools)

Semantic code indexing and search.

#### code_index

**Description**: Index a source code file for semantic and structural search
**Delegated**: ✅ Yes
**Async**: Yes

**Parameters**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| file_path | string | ✅ | Path to source file to index |

**Returns**:
```json
{
  "success": true,
  "file_path": "src/main.rs",
  "symbols_indexed": 25
}
```

---

#### code_search

**Description**: Search code using semantic meaning and structural relationships
**Delegated**: ✅ Yes
**Async**: Yes

**Parameters**:
| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| query | string | ✅ | - | Semantic search query |
| limit | integer | ❌ | 10 | Maximum results |

**Returns**:
```json
{
  "results": [
    {
      "file": "src/lib.rs",
      "symbol": "fn process",
      "score": 0.89
    }
  ],
  "count": 1
}
```

---

#### code_index_directory

**Description**: Index all code files in a directory matching a glob pattern
**Delegated**: ✅ Yes
**Async**: Yes

**Parameters**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| directory | string | ✅ | Directory path to index |
| pattern | string | ✅ | Glob pattern (e.g., '**/*.rs') |

**Returns**:
```json
{
  "success": true,
  "files_indexed": 42,
  "total_symbols": 567
}
```

---

### 8. Document Tools (2 tools)

Document indexing and semantic search.

#### document_index

**Description**: Index documents from a directory into global knowledge store
**Delegated**: ✅ Yes
**Async**: Yes

**Parameters**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| directory | string | ✅ | Directory containing documents |

**Returns**:
```json
{
  "success": true,
  "documents_indexed": 15
}
```

---

#### document_search

**Description**: Semantic search across indexed documents using vector embeddings
**Delegated**: ✅ Yes
**Async**: Yes

**Parameters**:
| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| query | string | ✅ | - | Search query |
| limit | integer | ❌ | 5 | Maximum results |

**Returns**:
```json
{
  "results": [
    {
      "document": "README.md",
      "excerpt": "matching text...",
      "score": 0.92
    }
  ],
  "count": 1
}
```

---

### 9. IntelliTask Tools (13 tools)

AI-powered task management using Ollama integration.

#### intellitask_generate

**Description**: Generate intelligent task breakdown from PRD using AI
**Delegated**: ❌ **Custom MCP Implementation**
**Async**: Yes
**Requires**: Ollama

**Custom Implementation**: Uses Ollama for AI-powered PRD parsing and task generation

**Parameters**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| prd_content | string | ✅ | Product Requirements Document text |

**Returns**:
```json
{
  "tasks": [],
  "metadata": {}
}
```

---

#### intellitask_subtasks

**Description**: Generate subtasks for a parent task
**Delegated**: ❌ **Custom MCP Implementation**
**Async**: Yes
**Requires**: Ollama

**Custom Implementation**: Uses Ollama for AI-powered subtask generation

**Parameters**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| parent_task_id | string | ✅ | Parent task identifier |
| parent_task_json | string | ✅ | Parent task JSON string |
| codebase_context | string | ❌ | Optional codebase context |

**Returns**:
```json
{
  "subtasks": [],
  "parent_id": "task_123"
}
```

---

#### intellitask_prioritize

**Description**: Prioritize tasks using AI reasoning
**Delegated**: ❌ **Custom MCP Implementation**
**Async**: Yes
**Requires**: Ollama

**Custom Implementation**: Uses Ollama for AI-powered task prioritization

**Parameters**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| tasks_json | string | ✅ | JSON array of tasks |
| business_context | string | ❌ | Business context for prioritization |

**Returns**:
```json
{
  "prioritized_tasks": [],
  "reasoning": "AI explanation"
}
```

---

#### intellitask_next

**Description**: Suggest next task to work on
**Delegated**: ❌ **Custom MCP Implementation**
**Async**: Yes
**Requires**: Ollama

**Custom Implementation**: Uses Ollama for AI-powered next task suggestion

**Parameters**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| completed_tasks | array | ✅ | Array of completed task strings |
| remaining_tasks_json | string | ✅ | JSON of remaining tasks |

**Returns**:
```json
{
  "suggested_task": {},
  "reasoning": "AI explanation"
}
```

---

#### intellitask_save

**Description**: Save IntelliTask breakdown to database
**Delegated**: ❌ **Custom MCP Implementation**
**Async**: Yes

**Custom Implementation**: Schema validation and persistence with integrity checks

**Parameters**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| breakdown_json | string | ✅ | Task breakdown JSON string |

**Returns**:
```json
{
  "success": true,
  "tasks_saved": 10
}
```

---

#### intellitask_get

**Description**: Get task by ID from database
**Delegated**: ✅ Yes
**Async**: Yes

**Parameters**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| task_id | integer | ✅ | Task ID |

**Returns**:
```json
{
  "task": {}
}
```

---

#### intellitask_list

**Description**: List tasks with optional filtering
**Delegated**: ✅ Yes
**Async**: Yes

**Parameters**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| status | string | ❌ | Filter by status |
| prd_title | string | ❌ | Filter by PRD title |
| parent_id | integer | ❌ | Filter by parent task ID |

**Returns**:
```json
{
  "tasks": [],
  "count": 5
}
```

---

#### intellitask_update_status

**Description**: Update task status
**Delegated**: ✅ Yes
**Async**: Yes

**Parameters**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| task_id | integer | ✅ | Task ID |
| status | string | ✅ | New status (pending/in-progress/done/cancelled) |

**Returns**:
```json
{
  "success": true,
  "task_id": 123,
  "new_status": "in-progress"
}
```

---

#### intellitask_next_ready

**Description**: Get next task ready to work on (dependencies satisfied)
**Delegated**: ✅ Yes
**Async**: Yes

**Parameters**: None

**Returns**:
```json
{
  "task": {}
}
```

---

#### intellitask_get_subtasks

**Description**: Get subtasks for a parent task
**Delegated**: ✅ Yes
**Async**: Yes

**Parameters**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| parent_id | integer | ✅ | Parent task ID |

**Returns**:
```json
{
  "subtasks": [],
  "parent_id": 123
}
```

---

#### intellitask_subtask_stats

**Description**: Get subtask statistics for a parent task
**Delegated**: ✅ Yes
**Async**: Yes

**Parameters**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| parent_id | integer | ✅ | Parent task ID |

**Returns**:
```json
{
  "total": 10,
  "completed": 7,
  "pending": 2,
  "in_progress": 1
}
```

---

#### intellitask_task_statistics

**Description**: Get overall task statistics across all tasks
**Delegated**: ✅ Yes
**Async**: Yes

**Parameters**: None

**Returns**:
```json
{
  "total_tasks": 50,
  "by_status": {},
  "by_priority": {}
}
```

---

#### intellitask_prd_statistics

**Description**: Get task statistics for a specific PRD
**Delegated**: ✅ Yes
**Async**: Yes

**Parameters**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| prd_title | string | ✅ | PRD title |

**Returns**:
```json
{
  "prd_title": "Example PRD",
  "task_count": 25,
  "completion_rate": 0.80
}
```

---

### 10. Graph Tools (3 tools)

Neo4j Cypher query execution and relationship management.

#### graph_query

**Description**: Execute a Cypher query on Neo4j graph database and return results
**Delegated**: ✅ Yes
**Async**: Yes

**Parameters**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| cypher | string | ✅ | Cypher query string |
| params | object | ❌ | Query parameters |

**Returns**:
```json
{
  "results": [],
  "count": 3
}
```

---

#### graph_insert

**Description**: Execute a Cypher write query (CREATE, MERGE, SET) on Neo4j
**Delegated**: ✅ Yes
**Async**: Yes

**Parameters**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| cypher | string | ✅ | Cypher write query |
| params | object | ❌ | Query parameters |

**Returns**:
```json
{
  "success": true,
  "nodes_created": 5,
  "relationships_created": 3
}
```

---

#### graph_relate

**Description**: Create a relationship between two nodes in Neo4j
**Delegated**: ✅ Yes
**Async**: Yes

**Parameters**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| from_id | integer | ✅ | Source node ID |
| to_id | integer | ✅ | Target node ID |
| rel_type | string | ✅ | Relationship type |
| from_label | string | ❌ | Source node label |
| to_label | string | ❌ | Target node label |

**Returns**:
```json
{
  "success": true,
  "relationship_id": 456
}
```

---

### 11. Agent Tools (8 tools)

Multi-agent message bus and coordination.

#### agent_send

**Description**: Send message to another agent via message bus
**Delegated**: ✅ Yes
**Async**: Yes

**Parameters**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| to | string | ✅ | Target agent ID |
| message | string | ✅ | Message content |

**Returns**:
```json
{
  "success": true,
  "message_id": "msg_789"
}
```

---

#### agent_recv

**Description**: Receive pending messages for a given agent ID
**Delegated**: ✅ Yes
**Async**: Yes

**Parameters**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| agent | string | ✅ | Agent ID to receive messages for |

**Returns**:
```json
{
  "messages": [],
  "count": 2
}
```

---

#### agent_poll

**Description**: Wait for the next message addressed to the specified agent
**Delegated**: ❌ **Custom MCP Implementation**
**Async**: Yes
**Blocking I/O**: Yes

**Custom Implementation**: Blocking I/O with timeout and message bus integration

**Parameters**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| agent | string | ✅ | Agent ID to poll for |
| timeout_ms | integer | ✅ | Timeout in milliseconds |

**Returns**:
```json
{
  "message": {},
  "timeout": false
}
```

---

#### agent_register

**Description**: Register an agent ID and its capabilities
**Delegated**: ✅ Yes
**Async**: Yes

**Parameters**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| id | string | ✅ | Agent ID |
| capabilities | array | ✅ | Array of capability strings |

**Returns**:
```json
{
  "success": true,
  "agent_id": "agent_123"
}
```

---

#### agent_list

**Description**: List all registered agents and their metadata
**Delegated**: ✅ Yes
**Async**: Yes

**Parameters**: None

**Returns**:
```json
{
  "agents": [],
  "count": 5
}
```

---

#### agent_status

**Description**: Update the status of the specified agent
**Delegated**: ✅ Yes
**Async**: Yes

**Parameters**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| id | string | ✅ | Agent ID |
| status | object | ✅ | Status object |

**Returns**:
```json
{
  "success": true
}
```

---

#### agent_task

**Description**: Send a structured task envelope to a specified agent
**Delegated**: ✅ Yes
**Async**: Yes

**Parameters**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| to | string | ✅ | Target agent ID |
| task_id | string | ✅ | Task identifier |
| task_type | string | ✅ | Task type |
| payload | object | ✅ | Task payload |

**Returns**:
```json
{
  "success": true,
  "task_id": "task_456"
}
```

---

#### agent_result

**Description**: Submit the result of a completed task to the router
**Delegated**: ✅ Yes
**Async**: Yes

**Parameters**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| from | string | ✅ | Source agent ID |
| task_id | string | ✅ | Task identifier |
| result | object | ✅ | Task result |

**Returns**:
```json
{
  "success": true
}
```

---

### 12. Mapping Tools (4 tools)

Application structure mapping and dependency tracking.

#### mapping_record

**Description**: Record a file node in the application structure map
**Delegated**: ✅ Yes
**Async**: Yes

**Parameters**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| path | string | ✅ | File path |
| kind | string | ✅ | File kind (module, component, etc.) |
| language | string | ❌ | Programming language |
| imports | array | ✅ | Array of import strings |
| exports | array | ✅ | Array of export strings |
| dependencies | array | ✅ | Array of dependency paths |

**Returns**:
```json
{
  "success": true,
  "path": "src/main.rs"
}
```

---

#### mapping_get

**Description**: Get a file node from the application structure map
**Delegated**: ✅ Yes
**Async**: Yes

**Parameters**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| path | string | ✅ | File path to retrieve |

**Returns**:
```json
{
  "node": {}
}
```

---

#### mapping_search

**Description**: Search for files related to a query using semantic search
**Delegated**: ✅ Yes
**Async**: Yes

**Parameters**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| query | string | ✅ | Search query |

**Returns**:
```json
{
  "files": [],
  "count": 3
}
```

---

#### mapping_deps

**Description**: Get all transitive dependencies for a file
**Delegated**: ✅ Yes
**Async**: Yes

**Parameters**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| path | string | ✅ | File path |

**Returns**:
```json
{
  "dependencies": [],
  "count": 8
}
```

---

### 13. Application Tools (4 tools)

Code change tracking and history.

#### application_record

**Description**: Record a code change in the application
**Delegated**: ✅ Yes
**Async**: Yes

**Parameters**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| file_path | string | ✅ | File path |
| change_type | string | ✅ | Change type (add/modify/delete) |
| line_start | integer | ✅ | Starting line number |
| line_end | integer | ✅ | Ending line number |
| old_content | string | ❌ | Previous content |
| new_content | string | ❌ | New content |
| description | string | ✅ | Change description |
| task_id | integer | ❌ | Associated task ID |

**Returns**:
```json
{
  "success": true,
  "change_id": 789
}
```

---

#### application_get

**Description**: Get all code changes for a task
**Delegated**: ✅ Yes
**Async**: Yes

**Parameters**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| task_id | integer | ✅ | Task ID |

**Returns**:
```json
{
  "changes": [],
  "count": 5
}
```

---

#### application_history

**Description**: Get change history for a specific file
**Delegated**: ✅ Yes
**Async**: Yes

**Parameters**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| file_path | string | ✅ | File path |

**Returns**:
```json
{
  "history": [],
  "count": 12
}
```

---

#### application_search

**Description**: Search code changes by semantic content
**Delegated**: ❌ **Custom MCP Implementation**
**Async**: Yes

**Custom Implementation**: Semantic search using vector similarity (not yet in RealExecutor)

**Parameters**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| query | string | ✅ | Search query |

**Returns**:
```json
{
  "results": [],
  "count": 3
}
```

---

### 14. Meta Tools (1 tool)

MCP server metadata and tool information.

#### tool_metadata_list

**Description**: List metadata for all MCP tools (category, cost, side effects)
**Delegated**: ❌ **Custom MCP Implementation**
**Async**: Yes

**Custom Implementation**: MCP-specific metadata and tool categorization

**Parameters**: None

**Returns**:
```json
[
  {
    "name": "memory_store",
    "category": "memory",
    "cost": "low",
    "has_side_effects": true,
    "description": "Store a value in memory"
  }
]
```

---

## Custom Tool Implementations

8 tools have custom MCP-specific implementations:

1. **intellitask_generate** - Ollama AI for PRD parsing
2. **intellitask_subtasks** - Ollama AI for subtask generation
3. **intellitask_prioritize** - Ollama AI for task prioritization
4. **intellitask_next** - Ollama AI for next task suggestion
5. **intellitask_save** - Schema validation and persistence
6. **agent_poll** - Blocking I/O with message bus timeout
7. **application_search** - Semantic vector search (not in RealExecutor)
8. **tool_metadata_list** - MCP server metadata

---

## Usage Examples

### Memory Storage
```json
// Request
{
  "name": "memory_store",
  "arguments": {
    "key": "user_preference",
    "value": "dark_mode",
    "dry_run": false
  }
}

// Response (Success)
{
  "ok": true,
  "data": {
    "success": true,
    "key": "user_preference",
    "message": "Value stored successfully"
  },
  "tool": "memory_store",
  "executor": "RealExecutor"
}
```

### Vector Search
```json
// Request
{
  "name": "vector_search",
  "arguments": {
    "query": "authentication middleware",
    "limit": 5
  }
}

// Response (Success)
{
  "ok": true,
  "data": {
    "results": [
      {
        "text": "JWT authentication middleware implementation",
        "score": 0.94,
        "metadata": {"file": "src/auth.rs"}
      }
    ],
    "count": 1
  },
  "tool": "vector_search",
  "executor": "RealExecutor"
}
```

### Error Example
```json
// Response (Error)
{
  "ok": false,
  "error": {
    "message": "Database connection failed",
    "code": "DB_CONNECTION_ERROR",
    "details": "SQLite connection pool exhausted"
  },
  "tool": "memory_store",
  "executor": "RealExecutor"
}
```

---

## Statistics

| Metric | Count |
|--------|-------|
| **Total Tools** | 50 |
| **Delegated to RealExecutor** | 42 |
| **Custom Implementations** | 8 |
| **Tools Requiring Ollama** | 4 |
| **Tools with Blocking I/O** | 1 |
| **Tool Categories** | 11 |

---

## Implementation Notes

### Delegation Flow

```
MCP Client
    ↓
syncore_mcp_stdio
    ↓
MCPServerHandler::tool_name()
    ↓
mcp_delegate("tool_name", params)
    ↓
RealExecutor::execute_real_tool_async()
    ↓
Error Envelope {ok, data/error, tool, executor}
    ↓
MCP CallToolResult
    ↓
MCP Client
```

### Error Handling

All delegated tools use unified error envelopes from RealExecutor:
- Consistent error format across all tools
- Standardized error codes
- Detailed error messages with context
- Stack traces in development mode

### Async Behavior

All 50 tools are async functions using Tokio runtime:
- Non-blocking I/O operations
- Concurrent request handling
- Efficient resource utilization
- 1 tool (agent_poll) uses intentional blocking with timeout

---

## Version History

**1.0 (2025-01-20)**
- Initial manifest after RealExecutor migration
- 42 tools delegated to unified executor
- 8 custom MCP implementations
- Standardized error envelopes

---

## See Also

- [SynCore Architecture](../CLAUDE.md)
- [RealExecutor Implementation](../src/macro_tools/executor_real.rs)
- [MCP Server Implementation](../src/mcp_server.rs)
- [Error Envelope Specification](error_envelopes.md)
