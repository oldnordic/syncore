# PEDANTIC_POLICY.md

SynCore / SQLiteGraph / GGUFEngine  
Rust Clippy + Pedantic Cleanup Standards  
Version: 1.0  
Last Updated: 2025-12-03

This document defines mandatory rules for code quality, pedantic cleanup,
and LLM-driven refactoring inside the SynCore ecosystem.  
These rules MUST be followed for any refactor, fix, cleanup, or test addition.

---

## 1. BASE RUST QUALITY POLICY

Whenever touching a file, **the following MUST be 100% clean**:

- No unused imports  
- No unused variables  
- No dead code  
- No unreachable code  
- No needless borrows  
- No needless clones  
- No needless returns  
- No shadowed variables  
- No redundant closures  
- No invalid match patterns  
- No partial moves unless deliberate  
- No ignored `Result` values unless explicitly justified  
- No lingering legacy code  
- No commented-out code blocks  

You must fix every base Clippy warning that appears in file.

---

## 2. PEDANTIC LINT POLICY

When any file is modified, pedantic linting MUST be applied according to rules below.

### 2.1 SAFE PEDANTIC LINTS (MUST FIX)
These MUST be fixed because they never change logic:

- `clippy::collapsible_if`
- `clippy::redundant_else`
- `clippy::needless_return`
- `clippy::unreadable_literal`
- `clippy::needless_borrow`
- `clippy::needless_closure`
- `clippy::redundant_clone`
- `clippy::unnecessary_cast`
- `clippy::multiple_crate_versions`
- `clippy::manual_range_contains`
- `clippy::let_and_return`

Safe means: fixing them cannot alter behavior.

### 2.2 NOISY OR STYLE PEDANTIC LINTS (ALLOW, WITH REASON)
These produce excessive noise or force unnatural code, so they are allowed:

- `clippy::similar_names`
- `clippy::module_name_repetitions`
- `clippy::too_many_arguments`
- `clippy::large_enum_variant`
- `clippy::enum_variant_names`
- `clippy::struct_excessive_bools`
- `clippy::missing_errors_doc`
- `clippy::missing_panics_doc`

For each, add:

```rust
#[allow(clippy::<lint>)] // Reason: <short justification>
```

### 2.3 DANGEROUS PEDANTIC LINTS (DO NOT FIX)

These MUST NOT be applied because they risk breaking logic:

- mutation-related lints on interior mutability
- concurrency lints altering ordering or memory semantics
- float-accuracy lints that modify numeric behavior
- lints that change public API signatures
- lints that break serialization or schemas
- `clippy::needless_raw_string_hashes` (can break SQL/JSON strings)
- `clippy::items_after_statements` (can break test organization)
- `clippy::unseparated_literal_suffix` (can break numeric types)
- `clippy::self_named_module_files` (can break module structure)

Instead, you allow them with:
```rust
#[allow(clippy::<lint>)] // Reason: Fix would alter behavior or API
```

---

## 3. FILE TOUCH RULE

When touching any file, you MUST:

- Fix all base Clippy warnings for that file.
- Apply Pedantic Policy:
  - Fix safe pedantic lints.
  - Allow noisy pedantic lints with justification.
  - Block dangerous pedantic lints with justification.
- Ensure no regressions in:
  - MCP protocol
  - GGUFEngine
  - SQLiteGraph
  - Graph backend parity
  - IntelliTask
  - LLM metrics & health
- Do NOT change behavior unless explicitly asked.
- Keep file under 300 LOC, unless:
  - It predates this policy, AND
  - It is a compound file (e.g., GGUFEngine), AND
  - Splitting it would break architecture or tests.

---

## 4. SYNCORE MEMORY STORAGE REQUIREMENT

Every time you perform cleanup, refactoring, removal, or fix:

You MUST write a memory entry using MCP:
```bash
syncore - memory_suite (MCP)(
    command: "store",
    key: "<meaningful_key>",
    value: "<detailed summary of changes + reasons + files touched>"
)
```

If this is not done, the decision is lost forever after the editor closes.

---

## 5. TEST REQUIREMENTS

After ANY modification:

- `cargo test --all` MUST pass
- All SQLiteGraph tests must pass
- All Neo4j parity tests must pass
- All MCP tool tests must pass
- All GGUFEngine tests must pass
- All LLM metrics/health tests must pass
- NO NEW warnings may be introduced
- Regression in ANY subsystem is forbidden.

---

## 6. NO NEW FEATURES DURING CLEANUP

During cleanup/refactor phases you MUST NOT:

- Add features
- Change APIs
- Modify architecture
- Introduce new modules
- Add dependencies

Cleanup ONLY, nothing else.

---

## 7. DOCUMENTATION REQUIREMENTS

When a lint is allowed, reason MUST be written:
```rust
#[allow(clippy::similar_names)] // Reason: domain model uses caller/callee pairs
```

When a lint is fixed, and fix changes code style:
- Add a short comment if needed
- Never remove context-critical comments
- Never rewrite explanation comments

## 8. SPECIAL CASES

### 8.1 Raw Strings with Quotes
For SQL queries, JSON strings, or regex patterns containing quotes:
- Use `r#""#` when necessary
- Add `#[allow(clippy::unnecessary_raw_string_hashes)]` if clippy complains
- Never escape quotes manually in complex strings

### 8.2 Test Code
Test files may have:
- Longer functions
- More arguments in test helpers
- Intentional duplication for clarity
- `#[allow(clippy::similar_names)]` for test variables

### 8.3 Legacy Code
When dealing with legacy code that cannot be safely refactored:
- Add `#[allow(clippy::<lint>)] // Reason: Legacy code, safe refactor requires broader changes`
- Create a task in memory system for future cleanup

---

## 9. ENFORCEMENT

These rules are enforced by:
1. Pre-commit hooks (when available)
2. CI/CD pipeline checks
3. Code review processes
4. LLM-driven refactoring tools

Violations must be fixed before merge.

---

## 10. EXAMPLES

### Good Fix (Safe Pedantic)
```rust
// BEFORE
fn example() -> Result<i32> {
    let x = compute();
    return Ok(x);
}

// AFTER
fn example() -> Result<i32> {
    Ok(compute())
}
```

### Good Allow (Noisy Pedantic)
```rust
#[allow(clippy::similar_names)] // Reason: domain model uses caller/callee pairs
fn upsert_call_edge(caller_id: i64, callee_id: i64) -> Result<()> {
    // ...
}
```

### Good Allow (Dangerous Pedantic)
```rust
#[allow(clippy::unnecessary_raw_string_hashes)] // Reason: SQL contains quotes
let query = r#"SELECT * FROM users WHERE name = "John""#;
```

---

## 11. END OF POLICY

This document governs ALL future cleanup/refactor work in SynCore.

Breaking this policy means work must be reverted and redone.

---

## 12. VERSION HISTORY

- v1.0 (2025-12-03): Initial version with comprehensive pedantic policy
- Based on Phase 14.4 cleanup experience and SynCore development standards

---

This document is living and will be updated as new patterns emerge or tooling changes.