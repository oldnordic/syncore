#!/bin/bash
# Store Phase 14.4 Clippy Cleanup Progress

echo "Storing Phase 14.4 progress to memory..."

# Create the JSON value for the progress data
PROGRESS_DATA='{
  "phase": "14.4",
  "name": "Clippy Pedantic Style Cleanup - In Progress",
  "status": "PARTIALLY_COMPLETE",
  "started_at": "2025-12-02",
  "initial_warnings": 4875,
  "current_warnings": 4853,
  "warnings_fixed": 22,
  "fixes_applied": [
    "needless_continue in src/code_directory_indexer.rs:122",
    "redundant_else in src/common/db_paths.rs:31-36",
    "unreadable_literal in src/config.rs:224 (1048576 → 1_048_576)",
    "unreadable_literal in src/autonomy.rs:286 (1234567890 → 1_234_567_890)",
    "similar_names allowances in src/code_graph/edge_persistence.rs and src/code_graph/explain.rs",
    "Raw string hash fixes in multiple files:",
    "- src/code_graph/graph.rs (4 SQL blocks)",
    "- src/mcp_tools/refrag_suite.rs (help text)",
    "- src/mcp_stdio.rs (JSON test string)",
    "- src/project_analysis/refactor_hotspots.rs (5 SQL queries)",
    "- src/project_analysis/rust_backend_ingestion.rs (2 JSON strings)",
    "- src/project_analysis/rust_macro_expander.rs (2 test strings, 1 regex allowed)",
    "- src/project_analysis/unused_imports.rs (SQL query)",
    "- src/project_analysis/cleanup.rs (2 JSON strings)",
    "- src/project_analysis/complexity_dashboard.rs (SQL query)",
    "- src/project_analysis/cycles.rs (SQL query)",
    "- src/project_analysis/dead_code.rs (SQL query)",
    "- src/project_analysis/deps.rs (2 SQL queries)",
    "- src/project_analysis/hotspots.rs (SQL query)"
  ],
  "remaining_work": {
    "total_remaining_warnings": 4853,
    "main_categories": [
      "unnecessary raw string hashes in many project_analysis files",
      "similar_names warnings that need allowances or fixes",
      "other pedantic style warnings"
    ],
    "next_steps": [
      "Continue fixing raw string hashes in remaining project_analysis files",
      "Address similar_names warnings with appropriate allowances",
      "Fix other pedantic warnings like derivable_impls, needless_return, etc.",
      "Run final verification with cargo clippy -- -W clippy::pedantic"
    ]
  },
  "notes": "Manual fixes required for raw strings containing quotes. SQL queries without quotes can safely use regular strings. JSON strings need proper escaping. Regex patterns with quotes may need allowances."
}'

# Create temporary file for MCP conversation
TEMP_FILE=$(mktemp)

# Build the MCP conversation
echo '{"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"phase14_progress_tracker","version":"1.0.0"}}}' > $TEMP_FILE
echo '{"jsonrpc":"2.0","method":"notifications/initialized"}' >> $TEMP_FILE
echo "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{\"name\":\"memory_suite\",\"arguments\":{\"command\":\"store\",\"key\":\"phase14_clippy_cleanup_progress\",\"value\":$PROGRESS_DATA}}}" >> $TEMP_FILE

# Run the conversation
echo "Running MCP conversation to store progress..."
cat $TEMP_FILE | ./target/release/syncore_mcp_stdio

# Clean up
rm -f $TEMP_FILE

echo ""
echo "Phase 14.4 progress stored successfully."
