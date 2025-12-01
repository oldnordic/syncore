# Clippy Cleanup Plan for SynCore

## Current Status
- **Total warnings/errors**: 121
- **Build status**: Compiles successfully with warnings

## Category Summary

### C1: Basic Hygiene (24 warnings)
- **Unused imports**: 16 warnings
- **Unused variables**: 8 warnings  
- **Dead code**: 0 warnings

### C2: needless_* Issues (0 warnings)
- **needless_borrow**: 0 warnings
- **needless_clone**: 0 warnings
- **unnecessary_to_owned**: 0 warnings

### C3: Complexity Issues (2 warnings)
- **too_many_arguments**: 2 warnings
- **type_complexity**: 0 warnings
- **large_enum_variant**: 0 warnings

### C4: Async/Lock Issues (2 warnings)
- **await_holding_lock**: 2 warnings

### C5: Deprecated/Unsafe/Correctness (10 warnings)
- **deprecated**: 8 warnings
- **unsafe**: 2 warnings
- **correctness**: 0 warnings

### Other Issues (83 warnings)
- **empty_line_after_doc_comments**: 1 warning
- **method never used**: 1 warning
- **parameter only used in recursion**: 5 warnings
- **ToString implementation**: 1 warning
- **unnecessary_filter_map**: 1 warning
- **double_ended_iterator_last**: 2 warnings
- **useless_asref**: 1 warning
- **collapsible_if**: 1 warning
- **assertions_on_constants**: 5 warnings
- **lines_filter_map_ok**: 1 warning
- **various other**: 63 warnings

## Top 10 Files with Most Warnings

1. `src/raggraph/validation.rs` - 2 warnings
2. `tests/vector_insert_mcp_test.rs` - 1 warning
3. `tests/mcp_crud_tests.rs` - 1 warning
4. `tests/config_unification_tests.rs` - 3 warnings
5. `src/validation/cross_domain_validator.rs` - 1 warning
6. `src/raggraph/storage_adapter.rs` - 1 warning
7. `src/raggraph/rag_query.rs` - 1 warning
8. `src/raggraph/config.rs` - 1 warning
9. `src/protocol.rs` - 1 warning
10. `src/project_analysis/refactor.rs` - 2 warnings

## Execution Order

### Phase 1: C1 - Basic Hygiene (Low Risk)
**Target**: 24 warnings
1. Remove unused imports (16 warnings)
2. Fix unused variables (8 warnings)
3. Remove trivial dead code (0 warnings)

**Files to prioritize**:
- `src/cognition/plan_engine.rs`
- `src/databases/neo4j/reader.rs`
- `src/lsp_bridge/diagnostics.rs`
- `src/mcp_tools/code_suite.rs`
- `src/memory_service/toon_engine.rs`
- `src/message_bus/adapter.rs`
- `src/message_bus/message.rs`
- `src/portfolio/code_graph_refactor.rs`
- `src/project_analysis/rust_backend_ingestion.rs`
- `src/refrag/compression.rs`
- `src/refrag/expand.rs`
- `src/rust_tools/clippy.rs`

### Phase 2: C2 - needless_* Issues (Low Risk)
**Target**: 0 warnings
- No issues to fix in this category

### Phase 3: C3 - Complexity Issues (Medium Risk)
**Target**: 2 warnings
1. Fix `too_many_arguments` in `src/code_graph/rag_graph_api.rs`
2. Fix `too_many_arguments` in `src/cognition/self_consistency.rs`

### Phase 4: C4 - Async/Lock Issues (Medium Risk)
**Target**: 2 warnings
1. Fix `await_holding_lock` in `src/code_graph/graph.rs` (2 instances)

### Phase 5: C5 - Deprecated/Unsafe/Correctness (Medium Risk)
**Target**: 10 warnings
1. Fix deprecated `HeartbeatMonitor::new` calls (4 instances in `src/autonomy.rs`)
2. Fix deprecated `SynCoreState` constructors (4 instances)
3. Remove unnecessary unsafe blocks (2 instances in `src/code_graph/edge_extractor.rs`)

### Phase 6: Other Issues (Mixed Risk)
**Target**: 83 warnings
1. Fix doc comment formatting
2. Remove unused methods
3. Fix recursion-only parameters
4. Implement Display instead of ToString
5. Fix iterator usage patterns
6. Remove constant assertions
7. Fix various other style issues

## Notes
- Many warnings appear to be style-related rather than functional issues
- Deprecated API usage should be updated to modern equivalents
- Async/lock issues need careful refactoring to maintain correctness
- No major architectural changes should be required
- All fixes should maintain existing public APIs