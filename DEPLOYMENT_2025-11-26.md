# SynCore Deployment - Embedding Metadata Fix

**Date**: 2025-11-26
**Version**: Release build with embedding metadata fixes
**Status**: ✅ DEPLOYED

---

## Deployment Steps Completed

### 1. ✅ Process Management
- Killed running syncore_mcp_stdio process
- Stopped background test processes

### 2. ✅ Build Preparation
- Removed old binary from `/home/feanor/.config/syncore/`
- Ran `cargo clean` (removed 12,661 files, 5.5GB)

### 3. ✅ Release Build
- Built with `cargo build --release --bin syncore_mcp_stdio`
- Build time: 41.24 seconds
- No compilation errors (51 pre-existing warnings)

### 4. ✅ Binary Deployment
- Copied `target/release/syncore_mcp_stdio` to `/home/feanor/.config/syncore/`
- Binary size: 59MB
- Permissions: -rwxr-xr-x (executable)
- Build ID: a61156385ab88a04f82791e830cb2f2c4f888e33

### 5. ✅ Verification
- Binary type: ELF 64-bit LSB pie executable
- Platform: x86-64, GNU/Linux 6.1.0
- Startup test: Successful (server initializes correctly)

---

## What's New in This Build

### Embedding Model Metadata Fixes

#### Fix 1: Dynamic Model Name Tracking
- Added `model_name()` method to Embeddings trait
- HuggingFaceEmbeddings now tracks which model is being used
- Supports three models:
  - "all-MiniLM-L6-v2" (GENERAL domain)
  - "BGE-small-en-v1.5" (CODE domain)
  - "semantic-word-vectors" (RealEmbeddings fallback)

#### Fix 2: VectorStore Model Name Exposure
- VectorStore now exposes `model_name()` method
- Enables runtime verification of which embedding model is in use

#### Fix 3: Indexer Metadata Correction
- Fixed hardcoded "all-MiniLM-L6-v2" in code_graph/indexer.rs
- Now dynamically retrieves model name from vector_store
- Ensures database metadata matches actual embedding model used

### Testing
- 4 new unit tests added and passing
- All existing tests pass
- No regressions detected

---

## Next Steps Required

### Re-index Codebase (CRITICAL)

The database still contains old metadata. To fix this:

```bash
# 1. Delete old embeddings
sqlite3 ~/.config/syncore/syncore_code_graph.db "DELETE FROM code_embeddings;"

# 2. Delete HNSW snapshot (if exists)
rm -f ~/.config/syncore/*.hnsw

# 3. Re-index via MCP tools
# Use code_suite with command='index_directory'
# directory: /home/feanor/Projects/SynCore/syncore
# pattern: **/*.rs

# 4. Verify metadata
sqlite3 ~/.config/syncore/syncore_code_graph.db \
  "SELECT DISTINCT model_version FROM code_embeddings;"
# Expected: BGE-small-en-v1.5
```

### Verification Checklist

After re-indexing:
- [ ] Verify code_embeddings.model_version shows "BGE-small-en-v1.5"
- [ ] Test code search returns correct results
- [ ] Test fusion queries work correctly
- [ ] Verify GENERAL domain documents show "all-MiniLM-L6-v2"

---

## Rollback Instructions (If Needed)

If issues arise, rollback by:
1. Stop the server
2. Restore from git: `git checkout HEAD -- src/vector.rs src/code_graph/indexer.rs`
3. Rebuild: `cargo build --release --bin syncore_mcp_stdio`
4. Copy binary to config directory

---

## Binary Details

**Location**: `/home/feanor/.config/syncore/syncore_mcp_stdio`
**Size**: 59MB
**Build**: Release (optimized)
**Built**: 2025-11-26 21:42
**Commit**: Embedding metadata fixes

**Startup Configuration**:
- Config file: syncore.toml (uses defaults if not found)
- Database path: syncore.db
- HTTP Streaming server: 127.0.0.1:8080
- Excluded directories: 23 (node_modules, target, etc.)

---

## Files Modified in This Release

1. **src/vector.rs**
   - Added model_name() to Embeddings trait
   - Updated HuggingFaceEmbeddings with model_name field
   - Implemented model_name() for all embedding types
   - Added VectorStore.model_name() method
   - Added 4 new unit tests

2. **src/code_graph/indexer.rs**
   - Replaced hardcoded model version with dynamic lookup
   - Lines 513-530 modified

---

## Risk Assessment

**Risk Level**: LOW

**Reasons**:
- Metadata-only changes (no algorithm modifications)
- All tests pass
- Clean compilation
- No breaking API changes
- Backward compatible (old data still works, just has wrong metadata)

**Mitigation**:
- Re-indexing will fix all metadata issues
- Rollback available if needed
- No production impact until re-index

---

**Deployment completed**: 2025-11-26 21:42
**Status**: ✅ SUCCESS
**Next action**: Re-index codebase to update metadata
