//! SQLiteGraph Async Backend Module
//!
//! Provides a hybrid async façade for synchronous GraphBackend implementations.
//! This module enables seamless async/await usage while keeping the core SQLiteGraph
//! implementation fully synchronous.
//!
//! ## Architecture
//!
//! - **AsyncSQLiteBackend**: Thin async wrapper using spawn_blocking
//! - **Core SQLiteGraph**: Remains fully synchronous (unchanged)
//! - **Error Handling**: Proper JoinError mapping and propagation
//! - **Thread Safety**: Uses Arc<dyn GraphBackend> for thread-safe sharing

/*
=================================================================
PHASE 11: BACKEND & TRAIT DISCOVERY DOCUMENTATION
==================================================================

BACKEND TRAITS AND IMPLEMENTORS DISCOVERED:

1. PRIMARY ASYNC TRAIT (src/graph/backend.rs):
   - GraphBackend (async trait) - THE CANONICAL INTERFACE
     - All methods are async fn
     - Implemented by: SQLiteGraphBackend, Neo4jBackend
     - Used by: SynCoreState.graph_backend, MCP server via graph_suite

2. SYNC WRAPPER TRAIT (src/sqlitegraph/async_sqlite_backend.rs):
   - SyncGraphBackend (sync trait) - DUPLICATE INTERFACE
     - All methods are sync fn (mirrors GraphBackend exactly)
     - Implemented by: AsyncSQLiteBackend only
     - Used by: SQLiteGraphStorageAdapter (raggraph module)

3. ADAPTER LAYERS:
   - AsyncSQLiteBackend: Wrapper around Arc<dyn GraphBackend>
     - Provides SyncGraphBackend implementation
     - Uses spawn_blocking/block_in_place internally
     - Creates runtime when needed

WHO CALLS WHICH INTERFACE:

- ASYNC CONTEXTS (use GraphBackend directly):
  * SynCoreState.graph_backend: Arc<dyn GraphBackend>
  * MCP server graph_suite tools
  * Router with_graph_backend_from_config()
  * Most high-level application code

- SYNC CONTEXTS (use SyncGraphBackend via AsyncSQLiteBackend):
  * SQLiteGraphStorageAdapter (raggraph module)
  * Code that needs sync interface but calls async backend

DRIFT/CONFUSION IDENTIFIED:

1. DUPLICATE TRAIT SURFACE:
   - SyncGraphBackend mirrors GraphBackend 1:1 (125+ methods)
   - Creates confusion about which interface to use
   - Double maintenance burden

2. ADAPTER DEPENDENCY:
   - SQLiteGraphStorageAdapter requires sync interface
   - But wraps async GraphBackend via AsyncSQLiteBackend
   - Creates unnecessary async→sync→async chain

3. INCONSISTENT USAGE:
   - Some code uses Arc<dyn GraphBackend> (async)
   - Other code uses Arc<AsyncSQLiteBackend> (sync wrapper)
   - No clear boundary or decision point

4. RUNTIME CREATION:
   - AsyncSQLiteBackend creates new tokio runtime per operation
   - Expensive and potentially problematic
   - Different async/sync boundary handling patterns

CANONICAL CONTRACT DECISION (STEP 2):

CHOICE: Option A - Async GraphBackend is the ONE canonical interface

REASONING (2-4 lines):
- Most high-level code already uses Arc<dyn GraphBackend> (router, MCP server)
- SQLiteGraphBackend and Neo4jBackend both implement async GraphBackend natively
- Only SQLiteGraphStorageAdapter needs sync interface - easier to change one adapter
- Async-to-sync bridging is cleaner than sync-to-async wrapping

INVARIANT: All higher-level code MUST use async GraphBackend.
Any sync interface is considered an internal implementation detail.

CANONICAL CONTRACT ENFORCEMENT (STEP 4):
- SQLiteGraphStorageAdapter now uses Arc<dyn GraphBackend> directly (async)
- No dependency on SyncGraphBackend or AsyncSQLiteBackend wrapper
- StorageAdapter trait remains sync for backward compatibility
- Async calls are wrapped internally using block_in_place
- This ensures single canonical backend interface while preserving existing semantics

==================================================================
*/

pub mod async_sqlite_backend;

// Note: AsyncSQLiteBackend and SyncGraphBackend are no longer exported as public API
// They exist only as internal implementation details for compatibility
// The canonical public interface is async GraphBackend only
