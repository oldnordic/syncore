# Complete Embedding Usage Map

**Date**: 2025-11-26
**Purpose**: Map ALL embedding model usage across codebase before making changes

---

## Production Code Paths

### 1. MCP Server Main (`src/mcp_stdio_main.rs`)

**CODE Domain** (Line 66):
```rust
let code_embeddings = Box::new(HuggingFaceEmbeddings::new_bge()?);
let mut code_store = VectorStore::new(code_embeddings);
```
- **Model**: BGE-small-en-v1.5
- **Purpose**: Code entity embeddings
- **Used by**: CodeGraph indexing, code search
- **Status**: ✅ CORRECT

**GENERAL Domain** (Line 73):
```rust
let general_embeddings = Box::new(HuggingFaceEmbeddings::new()?);
let mut general_store = VectorStore::new(general_embeddings);
```
- **Model**: all-MiniLM-L6-v2
- **Purpose**: Documents, tasks, notes, memories
- **Used by**: Document indexing, task search, memory search
- **Status**: ✅ CORRECT

### 2. Code Graph Indexer (`src/code_graph/indexer.rs`)

**Receives vector_store from caller** (Line 505-510):
```rust
let mut vector_store = self.vector_store.lock()?;
vector_store.insert_text(entity_id, None, &text, "code_entity")?;
```
- **Model**: Determined by caller (should be BGE from mcp_stdio_main)
- **Hardcoded metadata** (Line 517): ❌ `"all-MiniLM-L6-v2"` (WRONG!)
- **Status**: ❌ METADATA BUG

### 3. Global Vector Store (`src/global_store.rs`)

**Uses HuggingFaceEmbeddings::new()** (Lines 52, 75):
```rust
let embeddings = HuggingFaceEmbeddings::new()?;
let mut store = VectorStore::new(Box::new(embeddings));
```
- **Model**: all-MiniLM-L6-v2 (default)
- **Purpose**: Global document store
- **Status**: ✅ CORRECT (documents use GENERAL model)

### 4. Document Indexer (`src/document_indexer.rs`)

**Uses GlobalVectorStore::new()** (Lines 18, 35):
```rust
let mut vector_store = GlobalVectorStore::new()?;
```
- **Model**: all-MiniLM-L6-v2 (via GlobalVectorStore)
- **Purpose**: Document chunking and search
- **Status**: ✅ CORRECT

### 5. Fusion Queries (`src/code_graph/rag_graph_api.rs`)

**FusionAttention** (Line 116):
```rust
let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
let fusion = FusionAttention::new(embeddings);
```
- **Model**: all-MiniLM-L6-v2
- **Purpose**: Compute attention scores for fusion
- **Context**: Query context embedding
- **Status**: ⚠️ **INVESTIGATE** - Should this use CODE model (BGE)?

**FusionReasoning** (Line 146):
```rust
let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
let fusion = FusionReasoning::new(self.neo4j.clone(), vector_store);
```
- **Model**: all-MiniLM-L6-v2
- **Purpose**: Reasoning-based fusion
- **Status**: ⚠️ **INVESTIGATE** - Should this use CODE model (BGE)?

**Debug embedding** (Line 268):
```rust
let embeddings_for_debug = HuggingFaceEmbeddings::new()?;
let embedding = embeddings_for_debug.embed(context)?;
```
- **Model**: all-MiniLM-L6-v2
- **Purpose**: Debug info context embedding
- **Status**: ⚠️ **INVESTIGATE** - Context is code-related, should use BGE?

---

## Test Code Paths (Using StubEmbeddings or HuggingFace)

### Tests using StubEmbeddings (Fast, no actual model)
- `src/code_graph/delta.rs`: CodeGraph tests
- `src/http_stream_server.rs`: HTTP server tests
- `src/router.rs`: Router tests (multiple occurrences)
- `src/macro_tools/executor_real.rs`: Executor tests

### Tests using HuggingFaceEmbeddings::new() (MiniLM)
- `src/cognition/plan_executor.rs`: Plan execution tests
- `src/code_graph/index_application.rs`: Index application tests
- `src/workers/snapshot.rs`: Snapshot worker tests
- `src/code_directory_indexer.rs`: Directory indexer tests
- `src/code_graph/edge_persistence.rs`: Edge persistence tests
- `src/code_graph/fusion_reasoning.rs`: Fusion reasoning tests
- `src/code_graph/fusion_attention.rs`: Fusion attention tests
- `src/cognitive.rs`: Cognitive tests

### Tests using HuggingFaceEmbeddings::new_bge() (BGE)
- ❌ **NONE FOUND** - No tests specifically use BGE!

---

## GraphBERT Integration (GRAPH Domain)

**Location**: `src/code_graph/graph_bert.rs`

**Status**: ✅ Implemented but not integrated into production

**Purpose**: Combine CODE embeddings with graph features

**Usage Pattern** (Future):
```rust
// Step 1: Get CODE embedding (BGE)
let code_embedding = code_store.embed(entity_text)?;

// Step 2: Get graph features (degree, edges, etc.)
let graph_features = extract_graph_features(entity_id)?;

// Step 3: Apply GraphBERT transformation
let graph_bert = GraphBertModel::new()?;
let graph_embedding = graph_bert.embed_with_graph(&code_embedding, &graph_features);

// Step 4: Use graph_embedding for fusion queries
```

**Current State**: GraphBERT exists but is NOT called anywhere in production code

---

## Issue Analysis

### Issue 1: Hardcoded Model Metadata ❌ CRITICAL
**Location**: `src/code_graph/indexer.rs:517`
```rust
"all-MiniLM-L6-v2",  // Hardcoded - should be dynamic
```
**Impact**:
- CODE entities indexed with BGE but metadata says MiniLM
- Impossible to verify which model was actually used
- Will cause confusion during model upgrades

### Issue 2: Fusion Queries May Use Wrong Model ⚠️ INVESTIGATE
**Locations**:
- `src/code_graph/rag_graph_api.rs:116` (FusionAttention)
- `src/code_graph/rag_graph_api.rs:146` (FusionReasoning)
- `src/code_graph/rag_graph_api.rs:268` (Debug embedding)

**Questions**:
1. Should fusion attention use BGE (code-optimized) or MiniLM (general)?
2. Is context embedding comparing query (general) vs entities (code)?
3. Does model mismatch affect attention scores?

**Investigation Needed**:
- Check if fusion queries compare query embeddings vs entity embeddings
- If yes, models MUST match (both BGE or both MiniLM)
- If no (using graph scores only), model choice doesn't matter

### Issue 3: No BGE Tests ⚠️ LOW PRIORITY
**Impact**: Can't verify BGE model works correctly in tests
**Solution**: Add at least one integration test using BGE

### Issue 4: GraphBERT Not Integrated ℹ️ FUTURE WORK
**Status**: Implemented but unused
**Impact**: Missing third embedding domain
**Priority**: Low (not required for current functionality)

---

## Semantic Mismatch in Search Results (Original Bug)

**Root Cause** (Now Understood):
1. ✅ BGE embeddings ARE being used for indexing (correct)
2. ✅ Query embeddings use same BGE model (correct)
3. ❌ **Body snippets don't contain semantic keywords**

**Example**:
- Query: "function that creates Neo4j nodes"
- Expected: `create_code_entity_node`
- Actual: `vector_index_path`

**Why**:
- `vector_index_path` body contains: "vector", "index", "path" (exact keywords)
- `create_code_entity_node` body contains: "upsert_entity", "NodeLabel", "props" (different vocabulary)

**This is NOT a bug** - transformer models do lexical + semantic matching:
- Query keywords: "function", "creates", "Neo4j", "nodes"
- `vector_index_path` matches: "function" (✅), "index" (similar), "path" (similar)
- `create_code_entity_node` matches: "function" (✅), but no "creates"/"Neo4j"/"nodes"

**Solution**: Use better queries that match actual code vocabulary
- ✅ Good: "creates Neo4j nodes from code entities"
- ❌ Bad: "function that creates Neo4j nodes"

---

## Required Fixes

### Fix 1: Add model_name() to Embeddings Trait ✅ READY
**Priority**: HIGH
**Files**: `src/vector.rs`

### Fix 2: Fix Hardcoded Model Metadata ✅ READY
**Priority**: HIGH
**Files**: `src/code_graph/indexer.rs`

### Fix 3: Investigate Fusion Query Embeddings ⏳ REQUIRED
**Priority**: MEDIUM
**Questions to Answer**:
1. Does FusionAttention compare query embedding vs entity embeddings?
2. Should fusion use BGE (code) or MiniLM (general)?
3. Does model mismatch break attention scores?

### Fix 4: Add BGE Tests ⏳ OPTIONAL
**Priority**: LOW
**Files**: New test file or add to existing tests

### Fix 5: Integrate GraphBERT ⏳ FUTURE
**Priority**: LOW (not blocking)
**Files**: Production fusion query code paths

---

## Investigation Questions (MUST ANSWER BEFORE FIXING)

1. **Fusion Queries**:
   - What text is being embedded in fusion attention?
   - Is it the query string or the context?
   - Does it need to match CODE embeddings (BGE)?

2. **Model Consistency**:
   - When comparing embeddings, do models need to match?
   - Or does cosine similarity work across different models?
   - (Answer: Models MUST match - different embedding spaces)

3. **GraphBERT Integration**:
   - Where should GraphBERT be called?
   - Should it replace or augment existing embeddings?
   - Does it need its own vector store?

---

## Next Steps

1. ✅ Complete embedding usage map (this document)
2. ⏳ **Investigate fusion query embedding usage** (CRITICAL)
3. ⏳ Add model_name() to trait
4. ⏳ Fix hardcoded metadata
5. ⏳ Test and validate
6. ⏳ Re-index if needed

---

**Status**: Investigation phase complete
**Ready for**: Fusion query investigation
**Blocking**: Need to understand fusion embedding usage before fixing
