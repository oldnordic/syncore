//! Tree Logger Integration Tests
//!
//! Tests the complete Tree Logger system integration with ToTEngine.
//! Validates deterministic logging, event ordering, and tree structure.

use anyhow::Result;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use syncore::databases::cognition_graph::ThoughtNodeProperties;
use syncore::databases::logs::{ReasoningLogEntry, ReasoningLogWriter};
use syncore::reasoning::{engine::ToTEngine, tree_logger::TreeLogger, ReasoningNodeContext};
use tempfile::TempDir;

/// Create test database and logger
fn create_test_logger() -> (Arc<TreeLogger>, Arc<Mutex<Connection>>, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test_logs.db");
    let conn = Arc::new(Mutex::new(Connection::open(db_path).unwrap()));
    let log_writer = Arc::new(ReasoningLogWriter::new(conn.clone()).unwrap());
    let tree_logger = Arc::new(TreeLogger::new(log_writer));
    (tree_logger, conn, temp_dir)
}

/// Create mock Neo4j client for testing
struct MockNeo4jClient;

impl MockNeo4jClient {
    fn new() -> Arc<Self> {
        Arc::new(Self)
    }
}

#[test]
fn test_tree_logger_basic_functionality() -> Result<()> {
    let (tree_logger, _conn, _temp) = create_test_logger();
    let session_id = "test_session";
    let node_id = "test_node";

    // Test session start logging
    tree_logger.log_session_start(session_id, Some("task_123"), Some("Test session"))?;

    // Test expansion logging
    let branches = vec![
        "Branch A: Continue approach".to_string(),
        "Branch B: Alternative strategy".to_string(),
        "Branch C: Explore edge case".to_string(),
    ];
    tree_logger.log_expansion(session_id, node_id, 1, branches.len(), &branches)?;

    // Test score logging
    tree_logger.log_score(session_id, node_id, 1, 0.85, "High quality reasoning")?;

    // Test prune logging
    let pruned_branches = vec!["Weak branch".to_string()];
    tree_logger.log_prune(session_id, node_id, 2, "Low confidence", &pruned_branches)?;

    // Verify logs are stored
    let logs = tree_logger.get_formatted_logs(session_id)?;
    assert!(logs.contains("🚀"));
    assert!(logs.contains("🌳"));
    assert!(logs.contains("📊"));
    assert!(logs.contains("✂️"));

    Ok(())
}

#[test]
fn test_log_formatting_and_indentation() -> Result<()> {
    let (tree_logger, _conn, _temp) = create_test_logger();
    let session_id = "test_session";

    // Create a tree structure with different depths
    tree_logger.log_session_start(session_id, None, None)?;
    tree_logger.log_expansion(
        session_id,
        "root",
        0,
        2,
        &["Branch 1".to_string(), "Branch 2".to_string()],
    )?;
    tree_logger.log_expansion(
        session_id,
        "branch1",
        1,
        2,
        &["Branch 1.1".to_string(), "Branch 1.2".to_string()],
    )?;
    tree_logger.log_score(session_id, "branch1_1", 2, 0.9, "Excellent")?;
    tree_logger.log_prune(session_id, "branch1_2", 2, "Poor quality", &[])?;

    let formatted = tree_logger.get_formatted_logs(session_id)?;

    // Check proper indentation
    assert!(formatted.contains("🚀")); // Root level
    assert!(formatted.contains("🌳")); // Depth 0 - first expansion (no indentation)
    assert!(formatted.contains("  🌳")); // Depth 1 - second expansion (2 spaces)
    assert!(formatted.contains("    📊")); // Depth 2 - score (4 spaces)
    assert!(formatted.contains("    ✂️")); // Depth 2 - prune (4 spaces)

    // Check chronological ordering
    let lines: Vec<&str> = formatted.lines().collect();
    let mut session_start_pos = None;
    let mut expansion_pos = None;
    let mut score_pos = None;

    for (i, line) in lines.iter().enumerate() {
        if line.contains("🚀") {
            session_start_pos = Some(i);
        } else if line.contains("🌳") && line.contains("Branch 1") {
            expansion_pos = Some(i);
        } else if line.contains("📊") && line.contains("Excellent") {
            score_pos = Some(i);
        }
    }

    assert!(session_start_pos < expansion_pos);
    assert!(expansion_pos < score_pos);

    Ok(())
}

#[test]
fn test_deterministic_log_ordering() -> Result<()> {
    let (tree_logger, _conn, _temp) = create_test_logger();
    let session_id = "test_session";

    // Log events in a specific order
    let events = vec![
        ("session_start", "root", 0, "Session started"),
        ("expansion", "root", 0, "Expanded into 2 branches"),
        ("score", "branch1", 1, "Score: 0.800"),
        ("score", "branch2", 1, "Score: 0.600"),
        ("prune", "branch2", 1, "Pruned: Low score"),
    ];

    for (event_type, node_id, depth, message) in events {
        tree_logger.log_tree_event(session_id, node_id, event_type, message, depth)?;
    }

    // Verify logs are returned in the same order
    let logs = tree_logger.get_formatted_logs(session_id)?;
    let lines: Vec<&str> = logs.lines().filter(|line| !line.trim().is_empty()).collect();

    // Filter out the header line
    let log_lines: Vec<&str> = lines.iter().filter(|line| !line.contains("===")).copied().collect();

    assert_eq!(log_lines.len(), 5);
    assert!(log_lines[0].contains("🚀"));
    assert!(log_lines[1].contains("🌳"));
    assert!(log_lines[2].contains("📊"));
    assert!(log_lines[3].contains("📊"));
    assert!(log_lines[4].contains("✂️"));

    Ok(())
}

#[test]
fn test_node_specific_logs() -> Result<()> {
    let (tree_logger, _conn, _temp) = create_test_logger();
    let session_id = "test_session";
    let node_id = "test_node";

    // Log multiple events for the same node
    tree_logger.log_expansion(
        session_id,
        node_id,
        1,
        3,
        &["A".to_string(), "B".to_string(), "C".to_string()],
    )?;
    tree_logger.log_score(session_id, node_id, 1, 0.75, "Good quality")?;
    tree_logger.log_prune(session_id, node_id, 2, "Refinement needed", &["C".to_string()])?;

    // Get node-specific logs
    let node_logs = tree_logger.get_node_logs(node_id)?;
    assert_eq!(node_logs.len(), 3);

    // Verify all logs belong to the correct node
    for log in &node_logs {
        assert_eq!(log.node_id, node_id);
        assert_eq!(log.session_id, session_id);
    }

    // Verify event types
    let event_types: Vec<String> = node_logs.iter().map(|log| log.event_type.clone()).collect();
    assert!(event_types.contains(&"expansion".to_string()));
    assert!(event_types.contains(&"score".to_string()));
    assert!(event_types.contains(&"prune".to_string()));

    Ok(())
}

#[test]
fn test_recent_logs_functionality() -> Result<()> {
    let (tree_logger, _conn, _temp) = create_test_logger();
    let session_id = "test_session";

    // Create logs with different timestamps by adding small delays
    for i in 0..5 {
        let node_id = format!("node_{}", i);
        tree_logger.log_expansion(session_id, &node_id, i, 1, &[format!("Branch {}", i)])?;

        // Small delay to ensure different timestamps
        std::thread::sleep(std::time::Duration::from_millis(1));
    }

    // Get recent 3 logs
    let recent_logs = tree_logger.get_recent_logs(session_id, 3)?;
    assert_eq!(recent_logs.len(), 3);

    // Should be 3 most recent logs in chronological order
    assert!(recent_logs[0].node_id.contains("node_2"));
    assert!(recent_logs[1].node_id.contains("node_3"));
    assert!(recent_logs[2].node_id.contains("node_4"));

    Ok(())
}

#[test]
fn test_clear_session_logs() -> Result<()> {
    let (tree_logger, _conn, _temp) = create_test_logger();
    let session_id = "test_session";

    // Add some logs
    tree_logger.log_session_start(session_id, None, None)?;
    tree_logger.log_expansion(session_id, "root", 0, 2, &["A".to_string(), "B".to_string()])?;

    // Verify logs exist
    let logs_before = tree_logger.get_formatted_logs(session_id)?;
    assert!(logs_before.contains("🚀"));
    assert!(logs_before.contains("🌳"));

    // Clear logs
    let deleted_count = tree_logger.clear_session_logs(session_id)?;
    assert!(deleted_count > 0);

    // Verify logs are gone
    let logs_after = tree_logger.get_formatted_logs(session_id)?;
    assert!(!logs_after.contains("🚀"));
    assert!(!logs_after.contains("🌳"));

    Ok(())
}

#[test]
fn test_prune_event_structure() -> Result<()> {
    let (tree_logger, _conn, _temp) = create_test_logger();
    let session_id = "test_session";
    let node_id = "test_node";

    // Test prune with branches
    let pruned_branches = vec![
        "Weak branch A".to_string(),
        "Irrelevant branch B".to_string(),
        "Duplicate branch C".to_string(),
    ];
    tree_logger.log_prune(session_id, node_id, 2, "Quality filtering", &pruned_branches)?;

    let logs = tree_logger.get_node_logs(node_id)?;
    assert_eq!(logs.len(), 1);

    let log = &logs[0];
    assert_eq!(log.event_type, "prune");
    assert_eq!(log.depth, 2);
    assert!(log.message.contains("Pruned 3 branches"));
    assert!(log.message.contains("Weak branch A"));
    assert!(log.message.contains("Quality filtering"));

    // Test prune without branches
    tree_logger.log_prune(session_id, "node2", 1, "Node eliminated", &[])?;

    let logs2 = tree_logger.get_node_logs("node2")?;
    assert_eq!(logs2.len(), 1);
    assert!(logs2[0].message.contains("Pruned node: Node eliminated"));

    Ok(())
}

#[test]
fn test_expansion_event_content_truncation() -> Result<()> {
    let (tree_logger, _conn, _temp) = create_test_logger();
    let session_id = "test_session";
    let node_id = "test_node";

    // Create very long branch content
    let long_content = "This is a very long branch content that should be truncated when displayed in the logs to keep them readable and manageable".to_string();
    let branches = vec![long_content.clone()];

    tree_logger.log_expansion(session_id, node_id, 1, branches.len(), &branches)?;

    let logs = tree_logger.get_node_logs(node_id)?;
    assert_eq!(logs.len(), 1);

    let log = &logs[0];
    assert!(log.message.contains("Expanded into 1 branches"));
    // Should be truncated
    assert!(log.message.len() < long_content.len());
    assert!(log.message.contains("..."));

    Ok(())
}

#[test]
fn test_score_event_formatting() -> Result<()> {
    let (tree_logger, _conn, _temp) = create_test_logger();
    let session_id = "test_session";
    let node_id = "test_node";

    // Test various score values
    let test_cases = vec![
        (0.0, "Minimum score"),
        (0.5, "Average score"),
        (0.75, "Good score"),
        (1.0, "Perfect score"),
        (0.123456789, "Precise score"),
    ];

    for (score, evaluation) in test_cases {
        tree_logger.log_score(session_id, &format!("node_{}", score), 1, score, evaluation)?;
    }

    let logs = tree_logger.get_formatted_logs(session_id)?;

    // Check score formatting (3 decimal places)
    assert!(logs.contains("Score: 0.000"));
    assert!(logs.contains("Score: 0.500"));
    assert!(logs.contains("Score: 0.750"));
    assert!(logs.contains("Score: 1.000"));
    assert!(logs.contains("Score: 0.123"));

    Ok(())
}

#[test]
fn test_tree_logger_error_handling() -> Result<()> {
    let (tree_logger, _conn, _temp) = create_test_logger();
    let session_id = "test_session";

    // Test with empty strings
    tree_logger.log_tree_event(session_id, "node", "", "", 0)?;
    tree_logger.log_expansion(session_id, "node", 0, 0, &[])?;
    tree_logger.log_score(session_id, "node", 0, 0.0, "")?;
    tree_logger.log_prune(session_id, "node", 0, "", &[])?;

    // Should not panic and logs should be created
    let logs = tree_logger.get_formatted_logs(session_id)?;
    assert!(logs.lines().count() > 0);

    Ok(())
}

#[test]
fn test_concurrent_logging() -> Result<()> {
    let (tree_logger, _conn, _temp) = create_test_logger();
    let session_id = "test_session";

    // Spawn multiple threads to log concurrently
    let mut handles = vec![];
    for i in 0..5 {
        let logger_clone = Arc::clone(&tree_logger);
        let session_id_clone = session_id.to_string();
        let handle = std::thread::spawn(move || -> Result<()> {
            for j in 0..3 {
                let node_id = format!("node_{}_{}", i, j);
                logger_clone.log_expansion(
                    &session_id_clone,
                    &node_id,
                    i,
                    1,
                    &[format!("Branch {}-{}", i, j)],
                )?;
            }
            Ok(())
        });
        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap()?;
    }

    // Verify all logs were created
    let logs = tree_logger.get_formatted_logs(session_id)?;
    let expansion_count = logs.matches("🌳").count();
    assert_eq!(expansion_count, 15); // 5 threads * 3 expansions each

    Ok(())
}

#[test]
fn test_integration_with_reasoning_context() -> Result<()> {
    let (tree_logger, _conn, _temp) = create_test_logger();
    let session_id = "test_session";

    // Create a mock reasoning context
    let node = ThoughtNodeProperties {
        id: "test_node".to_string(),
        session_id: session_id.to_string(),
        parent_id: None,
        step_index: 0,
        content: "Initial reasoning step".to_string(),
        score: Some(1.0),
    };

    let context =
        ReasoningNodeContext::from_properties(node, session_id.to_string(), Vec::new(), Vec::new());

    // Log reasoning step using context
    tree_logger.log_reasoning_step(
        &context.session_id,
        &context.node.id,
        context.depth() as i64,
        "analysis",
        &context.node.content,
    )?;

    // Verify log was created with correct depth
    let logs = tree_logger.get_node_logs(&context.node.id)?;
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].depth, 0); // Root node depth
    assert!(logs[0].message.contains("analysis"));
    assert!(logs[0].message.contains("Initial reasoning step"));

    Ok(())
}
