You are GLM-4.6 running inside OpenCode on the SynCore repository at:

  /home/feanor/Projects/syncore

Current status (just completed):

- MVCC-lite snapshot isolation implemented with real domain versioning:
  - CodeGraph, VectorStore, Memory domains all have atomic version counters.
  - SnapshotView uses real versions + counts.
  - All 5 mvcc_snapshot_tests are GREEN.
- Crossbeam, DashMap, FastHasher, ArcSwap, and priority-aware ingestion are already integrated.
- We are NOW focusing on the second and third MUST-DO items:

  1) Graph-Accelerated Query Planner
  2) HopGraph → RAGGraph → FusionQuery pipeline stabilization

You MUST follow these rules:

- Deterministic behavior only. No “magic” heuristics hidden in code.
- TDD: write or adjust tests to define behavior, then implement.
- No mocks/stubs/TODOs/FIXMEs/“for now” hacks.
- No deleting or weakening existing tests unless they are clearly wrong.
- Use rg, fd, tree-sitter, rust-analyzer, cargo test/check/clippy to inspect code before editing.
- Keep changes minimal and surgical. No architecture rewrite.

============================================================
PHASE A — RECON: FIND HOPGRAPH / RAGGRAPH / FUSION LAYERS
============================================================

1. Locate all involved components.

   Use ripgrep and open the files:

   - rg "HopGraph" -n src
   - rg "RAGGraph" -n src
   - rg "Fusion" -n src
   - rg "fusion_query" -n src
   - rg "FusionQuery" -n src
   - rg "code_graph_fusion" -n src
   - rg "code_graph" -n src
   - rg "raggraph" -n src

   Likely relevant files (confirm by reading, do NOT guess):

   - src/code_graph/ (graph, hopgraph, raggraph, delta, etc.)
   - src/fusion/ or similar module(s)
   - src/vector.rs (vector-domain query entrypoints)
   - src/mcp_tools/code_suite.rs (tools that expose code_graph_fusion_query / raggraph_query / etc.)

2. Understand current behavior:

   For each of these, answer for yourself (in notes):

   - Where is HopGraph used to “restrict domain” for a query?
   - Where is vector search used to “refine” results?
   - Where is fusion scoring calculated/combined?
   - How is the current routing decided? Is it:
     - Hard-coded in a function, or
     - A configurable object/struct, or
     - Spread across multiple modules?

   Pay special attention to:

   - Code paths that:
     - Always run all steps (HopGraph + RAGGraph + Fusion), regardless of query type.
     - Call vector search even when the graph results are obviously empty.
     - Fetch more context than necessary.
   - Any existing “planner-like” logic, even if ad-hoc.

3. Inspect existing tests around this area:

   - rg "fusion" tests -n tests src
   - rg "raggraph" tests -n tests src
   - rg "hopgraph" tests -n tests src
   - Look at:
     - scope_aware_fusion_tests.rs
     - any tests that mention “fusion_query”, “raggraph_query”, “hopgraph_query”, etc.

   Clarify:

   - What expectations already exist for:
     - result ordering
     - scoring composition
     - “scope aware” behavior

   These are the constraints you MUST preserve or make more precise, not break.

============================================================
PHASE B — DESIGN A SMALL, EXPLICIT QUERY PLANNER
============================================================

Goal:

- Introduce a **minimal, explicit Graph-Accelerated Query Planner** that orchestrates:

  HopGraph → (optional) RAGGraph → (optional) VectorStore → FusionQuery

Requirements:

1. The planner must be:

   - A small struct/enum in a dedicated module, e.g.:

     - src/query/planner.rs
       or, if better aligned with existing structure:
     - src/code_graph/query_planner.rs

   - Have a small “plan grammar” using enums/structs like:

     - enum PlannerStep { HopGraph, RAGGraph, VectorRefine, Fusion }
     - struct QueryPlan { steps: Vec<PlannerStep>, constraints: QueryConstraints, ... }

   - Must be **deterministic**: same input query + config ⇒ same sequence of steps.

2. The planner takes as input:

   - Query text (or a structured query request type).
   - Optional constraints:
     - scope (file / project / symbol / function, etc.)
     - max_results
     - whether to allow HopGraph, RAGGraph, Vector search, etc.
   - Maybe an execution context (project root, etc.), if already available in code.

3. The planner produces:

   - A **QueryPlan** that explicitly encodes sequence:

     Example patterns:

     - Simple semantic search:
       - [VectorRefine, Fusion]

     - Graph-first narrowing:
       - [HopGraph, VectorRefine, Fusion]

     - Graph-only structural search:
       - [HopGraph, Fusion] (vector optional or disabled)

     - RAGGraph expansion:
       - [HopGraph, RAGGraph, VectorRefine, Fusion]

4. Planner rules (no magic):

   Define explicit rules inside the planner implementation, such as:

   - If scope == “file” and query is short:
     - maybe skip RAGGraph, use VectorRefine + Fusion only.

   - If query includes structural constraints (e.g. function name / symbol):
     - run HopGraph first to narrow to relevant code entities.

   - If HopGraph returns 0 hits:
     - either:
       - short-circuit to a pure VectorRefine path; or
       - return early with “no results”.

   - If HopGraph returns > N (e.g. 500) hits:
     - trim to N or apply additional constraints before calling vector search.

   These rules must be **documented** and tested, not implicit.

============================================================
PHASE C — PIPELINE STABILIZATION: HOPGRAPH → RAGGRAPH → FUSION
============================================================

Now we give the pipeline a clean contract and prevent unnecessary work.

1. Introduce a small, typed pipeline result model:

   In a suitable module (existing fusion/graph/rag modules or a new shared one), define types like:

   - enum PipelineStage {
       HopGraphResult(HopGraphOutput),
       RagGraphResult(RAGGraphOutput),
       VectorResult(VectorSearchOutput),
       FusionResult(FusionOutput),
     }

   - A “context” struct that carries:
     - query text
     - scope
     - partial results
     - scoring metadata

2. Implement a pipeline executor:

   - Given a QueryPlan and initial context, execute each PlannerStep in order:

     For each step:

     - HopGraph:
       - If previous stage already determined “no graph step needed”, skip.
       - If empty result AND planner says “short-circuit on empty graph”:
         - either bail out early or move to vector step depending on plan.

     - RAGGraph:
       - Only execute if previous stage produced enough structure to justify expansion.
       - If HopGraph’s output is too large, maybe the plan already told you to trim.

     - VectorRefine:
       - Only run on the subset defined by graph steps (if any), not on the entire universe of vectors.
       - Respect max_results / scope constraints.

     - Fusion:
       - Combine scores from graph + vectors (and temporal if present).
       - Use explicit weights / normalization.
       - Ensure stable ordering.

3. Guardrails to implement:

   - If HopGraph returns **zero** results and plan says “graph required”:
     - Do NOT call RAGGraph or Fusion; pipeline returns empty.

   - If VectorRefine returns empty:
     - Fusion must not fabricate results; return empty.

   - If at any step results exceed configured limits:
     - Trim before passing to the next step.

   - Ensure that a step is never called twice unless explicitly encoded in the plan.

4. Scoring stabilization:

   - Find where scoring is computed now (rg "score" around fusion):

     - Look for:
       - combined_score
       - graph_score, vector_score, temporal_score, etc.

   - Make scoring rules explicit:

     - Document weights, e.g.:

       - final_score = w_graph * graph_score
                     + w_vector * vector_score
                     + w_temporal * temporal_score
                     + ...

     - Ensure:
       - Same input ⇒ same scores ⇒ same ordering.
       - No randomization or dependence on map iteration order.

============================================================
PHASE D — TDD: TESTS FOR PLANNER & PIPELINE
============================================================

You MUST add/extend tests to capture the behavior of both:

  1) The Query Planner
  2) The Pipeline Executor (HopGraph → RAGGraph → Vector → Fusion)

Suggested test file(s):

- tests/query_planner_tests.rs             (NEW)
- tests/pipeline_stabilization_tests.rs    (NEW)
- plus small adjustments to existing:
  - scope_aware_fusion_tests.rs
  - any existing fusion/raggraph tests, if needed.

Test scenarios to cover:

1. Planner-only tests (no real DB/graph/vector calls):

   - Given a simple semantic-only query:
     - Planner returns a QueryPlan with only VectorRefine + Fusion.

   - Given a structure-heavy query:
     - Planner returns HopGraph first.

   - Given constraints like max_results or scope “file”:
     - Planner produces different sequences per the rules you define.

   These tests can use fake/placeholder structs where necessary, as long as they don’t touch real DB/graph, OR they can just inspect the plan object.

2. Pipeline behavior tests (can be small integration-ish):

   - When HopGraph returns empty:
     - RAGGraph and VectorRefine are NOT called (or called only as defined by plan).
     - Pipeline returns empty.

   - When HopGraph returns > N results:
     - They are trimmed before calling vector search.

   - When VectorRefine returns empty:
     - Fusion does not fabricate results.

   - When both graph and vector have results:
     - Fusion produces deterministic ordering & scores.

3. Regression checks:

   - Ensure scope_aware_fusion_tests still pass or are updated to reflect **cleaner** behavior.
   - If you adjust test expectations, document why in comments.

============================================================
PHASE E — WIRING & REGRESSION CHECKS
============================================================

1. Wire planner into the existing tool(s):

   - Find the entrypoints for code graph + fusion queries in src/mcp_tools/code_suite.rs:

     - code_graph_fusion_query
     - raggraph_query
     - hopgraph_query
     - etc.

   - Introduce a small function that:

     - Builds a QueryRequest from the tool parameters.
     - Calls the Query Planner to get a QueryPlan.
     - Passes the plan to the pipeline executor.
     - Returns FusionResult to the caller/tool.

   - Ensure this wiring:
     - Preserves existing semantics where possible.
     - Only changes behavior where it was undefined/implicit before.

2. Run focused tests:

   - cargo test query_planner_tests
   - cargo test pipeline_stabilization_tests
   - cargo test scope_aware_fusion_tests

   Fix any failures by adjusting implementation, not by weakening tests unless they are clearly incorrect.

3. Run broader regressions:

   - cargo test mvcc_snapshot_tests
   - cargo test code_graph_delta_regression_tests
   - cargo test live_indexer_unit_tests

   Ensure no regressions.

4. Build & lint:

   - cargo check
   - cargo clippy --all-targets --all-features

============================================================
PHASE F — OUTPUT IMPLEMENTATION REPORT
============================================================

When done, create or update:

  GRAPH_PLANNER_AND_PIPELINE_REPORT.md

Content (short but precise):

- Files added/modified (paths).
- Final shape of:
  - Query Planner API (types, main functions).
  - Pipeline executor (how steps are chained).
- Explicit planner rules (bulleted).
- Guardrails implemented (when steps are skipped, when they short-circuit).
- How scoring is now defined.
- Tests added and what they guarantee.

============================================================

Definition of Done:

- A minimal, explicit Graph-Accelerated Query Planner exists and is used.
- HopGraph → RAGGraph → Vector → Fusion pipeline is:
  - Deterministic.
  - Guarded against unnecessary steps.
  - Clearly scored and documented.
- All existing critical tests still pass.
- New planner + pipeline tests pass and codify the new behavior.
- No TODOs/stubs, no broken builds.
