// ============================================================================
// APEX 2.16 Domain Isolation Migration
// ----------------------------------------------------------------------------
// Purpose:
//   Retag existing Neo4j data so CodeGraph, CognitionGraph, and PortfolioGraph
//   occupy distinct graph domains with unique project labels + namespaces.
//
// Usage:
//   1. Run against a backup clone first (neo4j-shell, cypher-shell, or browser).
//   2. Validate counts at the end before pointing production tools at the DB.
//   3. Keep the pre-migration dump referenced in Phase 0 for rollback.
// ============================================================================

// -----------------------------
// Step 1: CodeGraph retrofits
// -----------------------------
// Identify legacy SynCore code nodes (file/path entities) and give them the new
// CodeGraph label, graph_domain flag, and prefixed namespace.

MATCH (n)
WHERE
    (n.project = 'SynCore' OR 'SynCore' IN labels(n))
    AND (
        EXISTS(n.path)
        OR EXISTS(n.start_line)
        OR EXISTS(n.end_line)
        OR EXISTS(n.signature)
    )
SET n:CodeGraph,
    n.graph_domain = 'code',
    n.project = 'CodeGraph',
    n.namespace = CASE
        WHEN n.namespace STARTS WITH 'code_' THEN n.namespace
        ELSE 'code_' + COALESCE(n.namespace, 'syncore_default')
    END;

// Ensure standalone File nodes that belong to code graph follow the same rules.
MATCH (f:File)
WHERE (f.project = 'SynCore' OR 'SynCore' IN labels(f))
SET f:CodeGraph,
    f.graph_domain = 'code',
    f.project = 'CodeGraph',
    f.namespace = CASE
        WHEN f.namespace STARTS WITH 'code_' THEN f.namespace
        ELSE 'code_' + COALESCE(f.namespace, 'syncore_default')
    END;

// -----------------------------
// Step 2: Cognition graph
// -----------------------------
// Tag reasoning episodes + references with CognitionGraph label and domain.
MATCH (e:ReasoningEpisode)
WHERE e.project IS NULL OR e.project = 'SynCore'
SET e:CognitionGraph,
    e.graph_domain = 'cognition',
    e.project = 'CognitionGraph',
    e.namespace = CASE
        WHEN e.namespace STARTS WITH 'cognition_' THEN e.namespace
        ELSE 'cognition_' + COALESCE(e.namespace, 'syncore_default')
    END;

// Rename lightweight references from :CodeEntity to :CodeReference for clarity.
MATCH (ref:CodeEntity)
WHERE ref.project = 'SynCore'
SET ref:CognitionGraph,
    ref.graph_domain = 'cognition',
    ref.project = 'CognitionGraph',
    ref.namespace = CASE
        WHEN ref.namespace STARTS WITH 'cognition_' THEN ref.namespace
        ELSE 'cognition_' + COALESCE(ref.namespace, 'syncore_default')
    END
REMOVE ref:CodeEntity
SET ref:CodeReference;

// -----------------------------
// Step 3: Portfolio graph
// -----------------------------
MATCH (n)
WHERE (n:Patch OR n:Step OR n:Task)
  AND (n.project = 'SynCore' OR 'SynCore' IN labels(n))
SET n:PortfolioGraph,
    n.graph_domain = 'portfolio',
    n.project = 'PortfolioGraph',
    n.namespace = CASE
        WHEN n.namespace STARTS WITH 'portfolio_' THEN n.namespace
        ELSE 'portfolio_' + COALESCE(n.namespace, 'syncore_default')
    END;

// Portfolio File attachments (if any) should also receive the new namespace.
MATCH (f:File:PortfolioGraph)
WHERE f.namespace IS NULL OR NOT f.namespace STARTS WITH 'portfolio_'
SET f.namespace = 'portfolio_' + COALESCE(f.namespace, 'syncore_default'),
    f.graph_domain = 'portfolio',
    f.project = 'PortfolioGraph';

// -----------------------------
// Step 4: Verification helpers
// -----------------------------
// Run after the above statements to confirm domain separation.

MATCH (n)
RETURN n.graph_domain AS graph_domain,
       n.project AS project,
       COUNT(*) AS total
ORDER BY graph_domain, project;

// Check for any remaining SynCore labels / projects that need manual cleanup.
MATCH (n:SynCore)
RETURN labels(n) AS labels, COUNT(*) AS total;

MATCH (n {project: 'SynCore'})
RETURN labels(n) AS labels, COUNT(*) AS total;
