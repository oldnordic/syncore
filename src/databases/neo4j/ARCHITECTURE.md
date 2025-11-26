# Neo4j Canonical Module Architecture

## Architecture Boundary: CRUD Layer vs Rebuild Layer

This document formalizes the architectural boundary between safe canonical CRUD operations and unsafe rebuild utilities in the Neo4j integration.

## Two-Layer Architecture

### Layer 1: Canonical CRUD API (Safe Operations)

**Location**: `src/databases/neo4j/{schema.rs, writer.rs, reader.rs}`

**Characteristics**:
- Type-safe schema validation at compile time
- Namespace-isolated operations
- Idempotent writes (MERGE-based)
- Parameterized queries (SQL injection safe)
- Double-label pattern (`:Function:SynCore`)
- Entity identification by ID
- Comprehensive error handling

**When to Use**: All normal application operations - creating, reading, updating, and deleting code entities and their relationships.

**API Functions**:

#### Schema (schema.rs)
- `NodeLabel` enum - Compile-time validated entity types
- `RelationType` enum - Compile-time validated relationship types
- `NodeProperties` struct - Complete property validation
- `PROJECT_LABEL` constant - Project namespace label
- `project_namespace()` - Runtime namespace resolution

#### Write Operations (writer.rs)
- `upsert_entity(client, label, props)` - Create/update typed entity by ID
- `create_relationship(client, src_id, dst_id, rel_type)` - Link existing entities
- `update_git_metadata(client, id, ...)` - Update temporal metadata
- `batch_upsert_entities(client, label, entities, batch_size)` - Bulk entity creation
- `batch_create_relationships(client, relationships, batch_size)` - Bulk relationship creation
- `delete_entity(client, id)` - Remove entity and relationships
- `delete_file_entities(client, path)` - Remove all entities in file

#### Read Operations (reader.rs)
- `get_entity_by_id(client, id)` - Fetch single entity
- `get_file_entities(client, path)` - Get all entities in file
- `get_function_callees(client, id)` - Get called functions
- `get_function_callers(client, id)` - Get calling functions
- `find_entities_by_name(client, pattern)` - Search by name
- `get_entities_by_type(client, label)` - Filter by type
- `count_entities_by_type(client)` - Entity type distribution
- `get_neighbors(client, id)` - Get connected entities
- `find_orphan_entities(client)` - Entities without relationships
- `validate_structure(client)` - Comprehensive graph statistics

**Guarantees**:
- ✅ Type safety - Invalid labels/relationships caught at compile time
- ✅ Namespace isolation - Queries never cross namespaces
- ✅ Idempotency - Safe to call multiple times
- ✅ Data integrity - Parameterized queries prevent injection
- ✅ Consistency - All operations use same schema definitions

### Layer 2: Rebuild Utilities (Unsafe Operations)

**Location**: `src/graph_rebuilder/neo4j_push.rs`

**Characteristics**:
- Bulk operations for performance
- Generic schema (`:CodeEntity` label)
- Node identification by NAME (not ID)
- MERGE operations that can create orphan nodes
- Bypass canonical type validation
- Direct execute_query usage

**When to Use**: ONLY during graph reconstruction, imports, or administrative operations. NEVER in normal application flows.

**API Functions**:

#### BatchEdgePusher
- `push_edges(edges)` - Batch edge creation by ID (uses canonical internally)
- `clear_all_edges()` - **DANGEROUS** bulk DELETE all relationships
- `push_edges_by_name(edges)` - MERGE nodes by name without IDs
- `push_typed_named_edges(edges, type)` - Create edges with generic label

**Dangers**:
- ⚠️ No type safety - Can create invalid node types
- ⚠️ No ID validation - Can create orphan nodes
- ⚠️ Destructive operations - clear_all_edges() deletes entire graph
- ⚠️ Name-based matching - Can match wrong entities
- ⚠️ Performance over safety - Trades correctness for speed

**Use Cases**:
- Full graph rebuild after schema changes
- Edge extraction re-processing
- Importing graph fragments from external sources
- Graph cleanup during development/testing
- Bulk operations where performance > safety

**DO NOT USE For**:
- Normal application operations
- Incremental updates
- User-triggered operations
- Production entity creation
- Any operation requiring data integrity guarantees

## Industry Pattern Alignment

This two-layer architecture follows established patterns from:

### Neo4j Admin Import
- **Admin Layer**: `neo4j-admin import` - Bulk CSV import, bypasses transaction log
- **CRUD Layer**: Cypher queries via driver - Type-safe, transactional operations
- **Pattern**: Fast bulk import vs safe incremental updates

### APOC Procedures
- **Admin Layer**: `apoc.periodic.*`, `apoc.refactor.*` - Batch operations, schema changes
- **CRUD Layer**: Standard Cypher - Node/relationship CRUD
- **Pattern**: Administrative utilities vs application queries

### Graph Loading Libraries
- **DGL/NetworkX**: Bulk graph construction from edge lists
- **Spark GraphX**: DataFrame-based bulk graph creation
- **Pattern**: Import layer vs query/traversal layer

### Database Migration Tools
- **Flyway/Liquibase**: Schema migrations, bulk data transforms
- **Application ORM**: Type-safe entity operations
- **Pattern**: Migration layer vs application data access layer

## Decision Tree: Which Layer to Use?

```
┌─────────────────────────────────────┐
│ Do you need to perform an operation │
│ on the code graph?                  │
└────────────┬────────────────────────┘
             │
             ▼
     ┌───────────────┐
     │ Is this a     │
     │ bulk rebuild  │
     │ or import?    │
     └───┬───────┬───┘
         │       │
        Yes     No
         │       │
         ▼       ▼
    ┌────────┐ ┌─────────────┐
    │ Rebuild│ │ Normal      │
    │ Layer  │ │ operation?  │
    └────────┘ └──────┬──────┘
                      │
                      ▼
              ┌───────────────┐
              │ Do you have   │
              │ entity IDs?   │
              └───┬───────┬───┘
                  │       │
                 Yes     No
                  │       │
                  ▼       ▼
         ┌─────────────┐ ┌────────────┐
         │ Canonical   │ │ Get IDs    │
         │ CRUD API    │ │ first via  │
         │ (SAFE)      │ │ canonical  │
         └─────────────┘ │ then CRUD  │
                         └────────────┘
```

## Examples

### Example 1: Creating a New Function Entity (CORRECT)

```rust
use crate::databases::neo4j::{NodeLabel, NodeProperties, upsert_entity};

// Use canonical CRUD API
let props = NodeProperties {
    id: function_id,
    name: "calculate_total".to_string(),
    path: "src/billing.rs".to_string(),
    start_line: 42,
    end_line: 58,
    // ... other properties
};

upsert_entity(&neo4j, NodeLabel::Function, props).await?;
```

### Example 2: Creating Relationships (CORRECT)

```rust
use crate::databases::neo4j::{RelationType, create_relationship};

// Link caller -> callee
create_relationship(
    &neo4j,
    caller_id,
    callee_id,
    RelationType::Calls
).await?;
```

### Example 3: Full Graph Rebuild (CORRECT - Use Rebuild Layer)

```rust
use crate::graph_rebuilder::neo4j_push::BatchEdgePusher;

// ONLY during graph rebuild operations
let pusher = BatchEdgePusher::new(neo4j);

// Clear existing edges
let deleted = pusher.clear_all_edges().await?;
eprintln!("Deleted {} edges for rebuild", deleted);

// Push new edge set by name (when IDs not available)
let edges = vec![
    ("main", "calculate_total", EdgeType::Calls),
    ("billing_module", "main", EdgeType::Contains),
];
pusher.push_edges_by_name(&edges).await?;
```

### Example 4: Incremental Update (WRONG - Don't Use Rebuild Layer)

```rust
// ❌ WRONG - Using rebuild layer for incremental update
let pusher = BatchEdgePusher::new(neo4j);
pusher.push_edges_by_name(&[("foo", "bar", EdgeType::Calls)]).await?;

// ✅ CORRECT - Use canonical CRUD API
use crate::databases::neo4j::{upsert_entity, create_relationship};

// First ensure entities exist
upsert_entity(&neo4j, NodeLabel::Function, foo_props).await?;
upsert_entity(&neo4j, NodeLabel::Function, bar_props).await?;

// Then create relationship
create_relationship(&neo4j, foo_id, bar_id, RelationType::Calls).await?;
```

## Migration Guidelines

When migrating code that uses `execute_query` directly:

### 1. Identify Operation Category

- **Entity CRUD** → Use `upsert_entity()`, `delete_entity()`
- **Relationship CRUD** → Use `create_relationship()`, `batch_create_relationships()`
- **Reads** → Use `get_entity_by_id()`, `find_entities_by_name()`, etc.
- **Statistics** → Use `validate_structure()`, `count_entities_by_type()`
- **Partial Updates** → Use specialized functions like `update_git_metadata()`
- **Bulk Rebuild** → Keep in rebuild layer, document as unsafe

### 2. Check for Rebuild Indicators

If code contains:
- `DETACH DELETE` on all relationships
- Node creation by NAME without ID validation
- Generic labels like `:CodeEntity`
- Comments like "for rebuild" or "bulk import"

→ This is rebuild layer code, document as unsafe utility

### 3. Apply Migration Pattern

See `/tmp/migration_knowledge.txt` for detailed migration patterns:
- Pattern 1: Entity CRUD
- Pattern 2: Relationships
- Pattern 3: Reads
- Pattern 4: Statistics/Aggregations
- Pattern 5: Partial Updates

### 4. Test After Migration

```bash
# Run canonical module tests
cargo test --lib databases::neo4j

# Run integration tests
NEO4J_URI="bolt://127.0.0.1:7687" NEO4J_USER="neo4j" NEO4J_PASS="testpassword123" cargo test
```

## Namespace Isolation

Both layers enforce namespace isolation but differently:

**Canonical CRUD**:
```rust
// Automatically includes namespace filter in all queries
MATCH (e {id: $id, namespace: $ns})
```

**Rebuild Utilities**:
```rust
// Must explicitly filter by namespace
MATCH ()-[r]->()
WHERE startNode(r).namespace = $ns AND endNode(r).namespace = $ns
DELETE r
```

Default namespace: `GRAPH_NAMESPACE` env var (default: "syncore_default")

## Schema Evolution

When adding new entity types or relationship types:

1. **Add to canonical schema** (`schema.rs`):
   ```rust
   pub enum NodeLabel {
       Function,
       Struct,
       NewEntityType,  // Add here
   }
   ```

2. **Canonical operations automatically support new types** - no migration needed

3. **Rebuild utilities require updates** - add new edge type handling in `push_edge_batch()`

## Testing Requirements

**Canonical CRUD Tests**: Must verify type safety, namespace isolation, idempotency
**Rebuild Utilities Tests**: Must verify bulk operations, namespace filtering, error handling

**Current test status**:
- Canonical: 6/6 tests passing
- Code graph: 69/69 tests passing
- Graph rebuilder: 3/3 tests passing

## References

- **Canonical Module**: `src/databases/neo4j/{mod.rs, schema.rs, writer.rs, reader.rs}`
- **Rebuild Utilities**: `src/graph_rebuilder/neo4j_push.rs`
- **Migration Knowledge**: `/tmp/migration_knowledge.txt`
- **Original Problem Analysis**: `NEO4J_FRAGMENTATION_AUDIT.md`

## Summary

- ✅ **Use canonical CRUD API** for all normal application operations
- ⚠️ **Use rebuild utilities** ONLY for graph reconstruction and imports
- 🚫 **Never expose rebuild utilities** to user-facing features
- 📋 **Always document** when using rebuild layer operations
- ✨ **Follow the decision tree** when choosing which layer to use
