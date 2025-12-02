# TASK 3 — Parity Issues Found

## Issue #1: Entity Label Mismatch Between Backends

### Description
SQLiteGraph and Neo4j backends return different `label` values for the same entity.

### Test Case
`test_comprehensive_entity_crud_operations` - Entity CRUD operations test

### Expected Behavior
Both backends should return identical `label` values for entities.

### Actual Behavior
- **SQLiteGraph**: Returns `label: "Struct"` 
- **Neo4j**: Returns `label: "CodeGraph"`

### Root Cause Analysis

#### SQLiteGraph Implementation
- File: `src/graph/sqlitegraph_impl.rs:190`
- Method: `get_entity_from_sqlite()`
- Code: `label: obj.get("entity_type").and_then(|v| v.as_str()).unwrap_or("").to_string(),`
- Behavior: Returns the `entity_type` field from database, which stores the EntityType enum as string (e.g., "Struct", "Function")

#### Neo4j Implementation  
- File: `src/databases/neo4j/reader.rs:63`
- Method: `get_entity_by_id()`
- Code: `labels(e)[0] as label,`
- Behavior: Returns the first Neo4j node label, which appears to be "CodeGraph" for all entities

### Impact
- **High**: Breaks backend parity guarantees
- **High**: Applications depending on consistent label behavior will fail
- **Medium**: Affects entity type filtering and classification

### Fix Required

#### Option 1: Fix Neo4j Label Retrieval
Update Neo4j reader to return the correct entity type label instead of the generic "CodeGraph" label.

#### Option 2: Fix SQLite Label Retrieval  
Update SQLite to return "CodeGraph" label to match Neo4j behavior.

#### Option 3: Standardize Both Backends
Update both backends to return a consistent label format (recommended).

### Recommendation
**Option 1** - Fix Neo4j to return specific entity type labels like SQLite does. This provides more granular entity type information which is useful for applications.

### Implementation Steps
1. Investigate Neo4j node creation to understand why all nodes get "CodeGraph" label
2. Update Neo4j entity creation to use specific type labels (Function, Struct, etc.)
3. Update Neo4j reader to return the correct label
4. Re-run parity tests to verify fix
5. Update any dependent code that relies on current behavior

### Test Evidence
```
Error: entity_crud_4: Entity 1 mismatch
SQLite: {"signature": String("signature_4"), "end_line": Number(55), "id": Number(5), "path": String("/tmp/comprehensive_test_4.rs"), "label": String("Struct"), "name": String("comprehensive_entity_4"), "body_snippet": String("body_4"), "start_line": Number(50)}
Neo4j: {"end_line": Number(55), "name": String("comprehensive_entity_4"), "signature": String("signature_4"), "start_line": Number(50), "body_snippet": String("body_4"), "id": Number(5), "label": String("CodeGraph"), "path": String("/tmp/comprehensive_test_4.rs")}
```

### Status
🔴 **CRITICAL** - Parity failure detected, fix required

---

## Additional Tests Status

### Comprehensive Parity Test Suite
- ✅ **Compilation**: All tests compile successfully
- ✅ **Setup**: Backend creation and configuration works
- ✅ **Basic CRUD**: Entity creation, retrieval, update, deletion works
- ❌ **Label Consistency**: Label values differ between backends
- 🔄 **In Progress**: Full test suite execution

### Test Coverage Implemented
1. **Entity CRUD Operations** - Create, Read, Update, Delete entities
2. **Batch Operations** - Bulk entity creation and retrieval  
3. **Relationship Operations** - Create and query relationships
4. **Pattern Matching** - Graph pattern queries
5. **Error Handling** - Invalid IDs, missing entities
6. **Ordering Behavior** - Deterministic result ordering

### Next Steps
1. Fix the label parity issue
2. Complete full test suite execution
3. Document any additional parity issues found
4. Create final backend equivalence matrix
5. Complete TASK3_COMPLETED.md summary

---

## Test Files Created
- `tests/dual_backend_parity_comprehensive.rs` - Main comprehensive test suite
- `tests/dual_parity/` - Modular test organization
  - `crud_parity_tests.rs` - CRUD operation parity
  - `relationship_parity_tests.rs` - Relationship parity
  - `pattern_parity_tests.rs` - Pattern matching parity
  - `raggraph_parity_tests.rs` - RAGGraph operations parity
  - `error_behavior_parity_tests.rs` - Error handling parity
  - `ordering_parity_tests.rs` - Deterministic ordering tests
- `tests/dual_parity/mod.rs` - Module organization
- `run_dual_parity_tests.sh` - Test execution script

## Test Execution Command
```bash
cargo test dual_backend_parity_comprehensive -- --test-threads=1
```