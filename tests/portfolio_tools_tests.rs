//! TDD Tests for Portfolio Enhancement Tools
//!
//! Tests mapping_tool, sequential_step, and application_tool
//! with real database operations (NO MOCKS, NO STUBS)

use anyhow::Result;
use std::sync::{Arc, Mutex};
use syncore::memory::Memory;
use syncore::message_bus::{
    message::{AgentId, MsgKind},
    MessageBus,
};
use syncore::router::SynCoreState;
use syncore::tasks::Tasks;
use syncore::vector::{RealEmbeddings, VectorStore};

/// Helper to create isolated test state with MessageBus
fn create_test_state() -> SynCoreState {
    let id = std::process::id();
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let mem_path = format!("/tmp/syncore_portfolio_test_mem_{}_{}.db", id, ts);
    let task_path = format!("/tmp/syncore_portfolio_test_task_{}_{}.db", id, ts);

    let memory = Memory::new(&mem_path).expect("Failed to create memory");
    let tasks = Tasks::new(&task_path).expect("Failed to create tasks");
    let embeddings = Box::new(RealEmbeddings::new(384).expect("Failed to create embeddings"));
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));

    let bus = MessageBus::new();
    SynCoreState::new(memory, tasks, vector_store).with_message_bus(bus)
}

// ==============================================================================
// TEST: mapping_tool - Application Structure Mapping
// ==============================================================================

#[test]
fn test_mapping_tool_records_file_structure() {
    use syncore::portfolio::mapping_tool::{FileNode, MappingTool};

    let state = create_test_state();
    let mapper = MappingTool::new(state.clone());

    // Map a file with its structure
    let file_node = FileNode {
        path: "src/main.rs".to_string(),
        kind: "file".to_string(),
        language: Some("rust".to_string()),
        imports: vec!["syncore::router".to_string(), "anyhow::Result".to_string()],
        exports: vec!["main".to_string()],
        dependencies: vec!["src/router.rs".to_string()],
    };

    let result = mapper.record_file(&file_node);
    assert!(result.is_ok(), "Failed to record file: {:?}", result.err());

    // Verify persisted in SQLite
    let retrieved = mapper.get_file("src/main.rs");
    assert!(retrieved.is_ok());
    let node = retrieved.unwrap().expect("File should exist");
    assert_eq!(node.path, "src/main.rs");
    assert_eq!(node.language, Some("rust".to_string()));
    assert_eq!(node.imports.len(), 2);
}

#[test]
fn test_mapping_tool_broadcasts_event() {
    use syncore::portfolio::mapping_tool::{FileNode, MappingTool};

    let state = create_test_state();
    let bus = state.message_bus.clone().expect("Bus should exist");
    let _rx = bus.register_agent(AgentId::Glm46);

    let mapper = MappingTool::new(state);

    let file_node = FileNode {
        path: "src/lib.rs".to_string(),
        kind: "file".to_string(),
        language: Some("rust".to_string()),
        imports: vec![],
        exports: vec!["protocol".to_string()],
        dependencies: vec![],
    };

    mapper.record_file(&file_node).unwrap();

    // Check that event was broadcast
    let history = bus.message_history();
    let mapping_events: Vec<_> = history
        .iter()
        .filter(|msg| matches!(&msg.kind, MsgKind::Event(e) if e == "mapping_update"))
        .collect();

    assert!(!mapping_events.is_empty(), "Should broadcast mapping event");
    assert_eq!(mapping_events[0].payload["path"], "src/lib.rs");
}

#[test]
fn test_mapping_tool_semantic_search() {
    use syncore::portfolio::mapping_tool::{FileNode, MappingTool};

    let state = create_test_state();
    let mapper = MappingTool::new(state);

    // Record multiple files
    let files = vec![
        FileNode {
            path: "src/auth/login.rs".to_string(),
            kind: "file".to_string(),
            language: Some("rust".to_string()),
            imports: vec!["jwt".to_string()],
            exports: vec!["LoginHandler".to_string()],
            dependencies: vec![],
        },
        FileNode {
            path: "src/api/routes.rs".to_string(),
            kind: "file".to_string(),
            language: Some("rust".to_string()),
            imports: vec!["axum".to_string()],
            exports: vec!["Router".to_string()],
            dependencies: vec!["src/auth/login.rs".to_string()],
        },
    ];

    for f in &files {
        mapper.record_file(f).unwrap();
    }

    // Semantic search for authentication
    let results = mapper.search_related("user authentication").unwrap();
    assert!(!results.is_empty(), "Should find related files");
    // Login file should be more relevant
    assert!(results.iter().any(|r| r.path.contains("login")));
}

#[test]
fn test_mapping_tool_dependency_graph() {
    use syncore::portfolio::mapping_tool::{FileNode, MappingTool};

    let state = create_test_state();
    let mapper = MappingTool::new(state);

    // Create dependency chain: A -> B -> C
    let files = vec![
        FileNode {
            path: "src/a.rs".to_string(),
            kind: "file".to_string(),
            language: Some("rust".to_string()),
            imports: vec![],
            exports: vec!["A".to_string()],
            dependencies: vec!["src/b.rs".to_string()],
        },
        FileNode {
            path: "src/b.rs".to_string(),
            kind: "file".to_string(),
            language: Some("rust".to_string()),
            imports: vec![],
            exports: vec!["B".to_string()],
            dependencies: vec!["src/c.rs".to_string()],
        },
        FileNode {
            path: "src/c.rs".to_string(),
            kind: "file".to_string(),
            language: Some("rust".to_string()),
            imports: vec![],
            exports: vec!["C".to_string()],
            dependencies: vec![],
        },
    ];

    for f in &files {
        mapper.record_file(f).unwrap();
    }

    // Query transitive dependencies
    let deps = mapper.get_all_dependencies("src/a.rs").unwrap();
    assert_eq!(deps.len(), 2);
    assert!(deps.contains(&"src/b.rs".to_string()));
    assert!(deps.contains(&"src/c.rs".to_string()));
}

// ==============================================================================
// TEST: sequential_step - Reasoning Chain Steps
// ==============================================================================

#[test]
fn test_sequential_step_records_thought() {
    use syncore::portfolio::sequential_step::{SequentialStep, ThoughtStep};

    let state = create_test_state();
    let sequential = SequentialStep::new(state.clone());

    let step = ThoughtStep {
        task_id: Some(42),
        step_number: 1,
        thought: "Analyzing the user's request for authentication".to_string(),
        action: Some("Read auth module".to_string()),
        observation: None,
        reasoning: "Need to understand current auth implementation".to_string(),
    };

    let result = sequential.record_step(&step);
    assert!(result.is_ok(), "Failed to record step: {:?}", result.err());

    // Verify step is persisted
    let steps = sequential.get_steps_for_task(42).unwrap();
    assert_eq!(steps.len(), 1);
    assert_eq!(
        steps[0].thought,
        "Analyzing the user's request for authentication"
    );
}

#[test]
fn test_sequential_step_broadcasts_event() {
    use syncore::portfolio::sequential_step::{SequentialStep, ThoughtStep};

    let state = create_test_state();
    let bus = state.message_bus.clone().expect("Bus should exist");
    let _rx = bus.register_agent(AgentId::Claude);

    let sequential = SequentialStep::new(state);

    let step = ThoughtStep {
        task_id: Some(10),
        step_number: 1,
        thought: "Planning implementation".to_string(),
        action: Some("Create schema".to_string()),
        observation: None,
        reasoning: "Database first approach".to_string(),
    };

    sequential.record_step(&step).unwrap();

    let history = bus.message_history();
    let step_events: Vec<_> = history
        .iter()
        .filter(|msg| matches!(&msg.kind, MsgKind::Event(e) if e == "sequential_step"))
        .collect();

    assert!(
        !step_events.is_empty(),
        "Should broadcast sequential step event"
    );
    assert_eq!(step_events[0].payload["task_id"], 10);
    assert_eq!(step_events[0].payload["step_number"], 1);
}

#[test]
fn test_sequential_step_chains_correctly() {
    use syncore::portfolio::sequential_step::{SequentialStep, ThoughtStep};

    let state = create_test_state();
    let sequential = SequentialStep::new(state);

    // Record multiple steps in sequence
    for i in 1..=3 {
        let step = ThoughtStep {
            task_id: Some(100),
            step_number: i,
            thought: format!("Step {} thought", i),
            action: Some(format!("Step {} action", i)),
            observation: Some(format!("Step {} result", i)),
            reasoning: format!("Step {} reasoning", i),
        };
        sequential.record_step(&step).unwrap();
    }

    let steps = sequential.get_steps_for_task(100).unwrap();
    assert_eq!(steps.len(), 3);

    // Verify ordering
    for (idx, step) in steps.iter().enumerate() {
        assert_eq!(step.step_number, (idx + 1) as i32);
    }
}

#[test]
fn test_sequential_step_search_by_content() {
    use syncore::portfolio::sequential_step::{SequentialStep, ThoughtStep};

    let state = create_test_state();
    let sequential = SequentialStep::new(state);

    let step = ThoughtStep {
        task_id: Some(200),
        step_number: 1,
        thought: "Implementing vector similarity search with FAISS".to_string(),
        action: Some("Write FAISS integration".to_string()),
        observation: None,
        reasoning: "FAISS provides efficient nearest neighbor search".to_string(),
    };

    sequential.record_step(&step).unwrap();

    // Search by semantic content
    let results = sequential
        .search_steps("vector database performance")
        .unwrap();
    assert!(!results.is_empty());
    assert!(results[0].thought.contains("FAISS"));
}

// ==============================================================================
// TEST: application_tool - Code Change Tracking
// ==============================================================================

#[test]
fn test_application_tool_records_change() {
    use syncore::portfolio::application_tool::{ApplicationTool, CodeChange};

    let state = create_test_state();
    let app_tool = ApplicationTool::new(state.clone());

    let change = CodeChange {
        file_path: "src/router.rs".to_string(),
        change_type: "add".to_string(),
        old_content: None,
        new_content: Some("pub fn new_route() {}".to_string()),
        line_start: 100,
        line_end: 102,
        description: "Added new route handler".to_string(),
        task_id: Some(42),
    };

    let result = app_tool.record_change(&change);
    assert!(
        result.is_ok(),
        "Failed to record change: {:?}",
        result.err()
    );

    // Verify persisted
    let changes = app_tool.get_changes_for_task(42).unwrap();
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].file_path, "src/router.rs");
}

#[test]
fn test_application_tool_broadcasts_event() {
    use syncore::portfolio::application_tool::{ApplicationTool, CodeChange};

    let state = create_test_state();
    let bus = state.message_bus.clone().expect("Bus should exist");
    let _rx = bus.register_agent(AgentId::Internal("watcher".into()));

    let app_tool = ApplicationTool::new(state);

    let change = CodeChange {
        file_path: "src/api.rs".to_string(),
        change_type: "modify".to_string(),
        old_content: Some("fn old() {}".to_string()),
        new_content: Some("fn new() {}".to_string()),
        line_start: 50,
        line_end: 55,
        description: "Refactored API handler".to_string(),
        task_id: Some(10),
    };

    app_tool.record_change(&change).unwrap();

    let history = bus.message_history();
    let change_events: Vec<_> = history
        .iter()
        .filter(|msg| matches!(&msg.kind, MsgKind::Event(e) if e == "code_change"))
        .collect();

    assert!(
        !change_events.is_empty(),
        "Should broadcast code change event"
    );
    assert_eq!(change_events[0].payload["file_path"], "src/api.rs");
    assert_eq!(change_events[0].payload["change_type"], "modify");
}

#[test]
fn test_application_tool_semantic_change_search() {
    use syncore::portfolio::application_tool::{ApplicationTool, CodeChange};

    let state = create_test_state();
    let app_tool = ApplicationTool::new(state);

    let changes = vec![
        CodeChange {
            file_path: "src/auth.rs".to_string(),
            change_type: "add".to_string(),
            old_content: None,
            new_content: Some("async fn login() {}".to_string()),
            line_start: 1,
            line_end: 10,
            description: "Added user authentication login endpoint".to_string(),
            task_id: Some(1),
        },
        CodeChange {
            file_path: "src/db.rs".to_string(),
            change_type: "modify".to_string(),
            old_content: Some("fn connect() {}".to_string()),
            new_content: Some("async fn connect() {}".to_string()),
            line_start: 5,
            line_end: 15,
            description: "Made database connection async".to_string(),
            task_id: Some(2),
        },
    ];

    for c in &changes {
        app_tool.record_change(c).unwrap();
    }

    // Search for auth-related changes
    let results = app_tool.search_changes("authentication login").unwrap();
    assert!(!results.is_empty());
    assert!(results[0].file_path.contains("auth"));
}

#[test]
fn test_application_tool_change_history_for_file() {
    use syncore::portfolio::application_tool::{ApplicationTool, CodeChange};

    let state = create_test_state();
    let app_tool = ApplicationTool::new(state);

    // Multiple changes to same file
    for i in 1..=3 {
        let change = CodeChange {
            file_path: "src/config.rs".to_string(),
            change_type: "modify".to_string(),
            old_content: Some(format!("version {}", i - 1)),
            new_content: Some(format!("version {}", i)),
            line_start: 1,
            line_end: 5,
            description: format!("Update config version to {}", i),
            task_id: Some(i as i64),
        };
        app_tool.record_change(&change).unwrap();
    }

    let history = app_tool.get_file_history("src/config.rs").unwrap();
    assert_eq!(history.len(), 3);

    // Should be in chronological order
    for (idx, change) in history.iter().enumerate() {
        let expected = format!("Update config version to {}", idx + 1);
        assert_eq!(change.description, expected);
    }
}

// ==============================================================================
// TEST: Integration - All Tools Work Together
// ==============================================================================

#[test]
fn test_all_tools_share_message_bus() {
    use syncore::portfolio::application_tool::{ApplicationTool, CodeChange};
    use syncore::portfolio::mapping_tool::{FileNode, MappingTool};
    use syncore::portfolio::sequential_step::{SequentialStep, ThoughtStep};

    let state = create_test_state();
    let bus = state.message_bus.clone().expect("Bus should exist");

    // Register a listener agent
    let _rx = bus.register_agent(AgentId::Glm46);

    let mapper = MappingTool::new(state.clone());
    let sequential = SequentialStep::new(state.clone());
    let app_tool = ApplicationTool::new(state);

    // Each tool publishes an event
    mapper
        .record_file(&FileNode {
            path: "src/test.rs".to_string(),
            kind: "file".to_string(),
            language: Some("rust".to_string()),
            imports: vec![],
            exports: vec![],
            dependencies: vec![],
        })
        .unwrap();

    sequential
        .record_step(&ThoughtStep {
            task_id: Some(1),
            step_number: 1,
            thought: "Test thought".to_string(),
            action: None,
            observation: None,
            reasoning: "Test reasoning".to_string(),
        })
        .unwrap();

    app_tool
        .record_change(&CodeChange {
            file_path: "src/test.rs".to_string(),
            change_type: "add".to_string(),
            old_content: None,
            new_content: Some("// test".to_string()),
            line_start: 1,
            line_end: 1,
            description: "Added test file".to_string(),
            task_id: Some(1),
        })
        .unwrap();

    // All events should be in shared history
    let history = bus.message_history();
    assert_eq!(history.len(), 3, "Should have 3 events from 3 tools");

    let event_types: Vec<String> = history
        .iter()
        .filter_map(|msg| match &msg.kind {
            MsgKind::Event(e) => Some(e.clone()),
            _ => None,
        })
        .collect();

    assert!(event_types.contains(&"mapping_update".to_string()));
    assert!(event_types.contains(&"sequential_step".to_string()));
    assert!(event_types.contains(&"code_change".to_string()));
}

// ==============================================================================
// NEO4J INTEGRATION TESTS (TDD - Write tests first)
// ==============================================================================

/// Helper to create test state with Neo4j
async fn create_neo4j_test_state() -> SynCoreState {
    use syncore::graph::neo4j_client::Neo4jClient;

    let id = std::process::id();
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let mem_path = format!("/tmp/syncore_portfolio_neo4j_test_mem_{}_{}.db", id, ts);
    let task_path = format!("/tmp/syncore_portfolio_neo4j_test_task_{}_{}.db", id, ts);

    let memory = Memory::new(&mem_path).expect("Failed to create memory");
    let tasks = Tasks::new(&task_path).expect("Failed to create tasks");
    let embeddings = Box::new(RealEmbeddings::new(384).expect("Failed to create embeddings"));
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let bus = MessageBus::new();

    let neo4j = Neo4jClient::connect("bolt://localhost:7687", "neo4j", "testpassword123")
        .await
        .expect("Failed to connect to Neo4j");

    SynCoreState::new(memory, tasks, vector_store)
        .with_message_bus(bus)
        .with_neo4j(Arc::new(neo4j))
}

#[tokio::test(flavor = "multi_thread")]
async fn test_mapping_tool_creates_neo4j_file_nodes() {
    use syncore::portfolio::mapping_tool::{FileNode, MappingTool};

    let state = create_neo4j_test_state().await;
    let mapper = MappingTool::new(state.clone());

    // Record a file with dependencies
    let file_node = FileNode {
        path: "src/test_neo4j_a.rs".to_string(),
        kind: "file".to_string(),
        language: Some("rust".to_string()),
        imports: vec!["anyhow".to_string()],
        exports: vec!["test_fn".to_string()],
        dependencies: vec!["src/test_neo4j_b.rs".to_string()],
    };

    mapper.record_file(&file_node).unwrap();

    // Verify Neo4j has the File node and DEPENDS_ON relationship
    let neo4j = state.neo4j.as_ref().unwrap();
    let results = neo4j
        .execute_query(
            "MATCH (f:File {path: $path}) RETURN f.path as path",
            vec![("path", serde_json::json!("src/test_neo4j_a.rs"))],
        )
        .await
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["path"], "src/test_neo4j_a.rs");

    // Verify DEPENDS_ON relationship was created
    let dep_results = neo4j
        .execute_query(
            "MATCH (a:File {path: $from})-[:DEPENDS_ON]->(b:File {path: $to}) RETURN b.path as dep",
            vec![
                ("from", serde_json::json!("src/test_neo4j_a.rs")),
                ("to", serde_json::json!("src/test_neo4j_b.rs")),
            ],
        )
        .await
        .unwrap();

    assert_eq!(dep_results.len(), 1);
    assert_eq!(dep_results[0]["dep"], "src/test_neo4j_b.rs");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_sequential_step_creates_neo4j_step_nodes() {
    use syncore::portfolio::sequential_step::{SequentialStep, ThoughtStep};

    let state = create_neo4j_test_state().await;
    let seq = SequentialStep::new(state.clone());

    // Record two consecutive steps
    let step1 = ThoughtStep {
        task_id: Some(999),
        step_number: 1,
        thought: "Analyze the problem".to_string(),
        action: Some("Read code".to_string()),
        observation: Some("Found bug".to_string()),
        reasoning: "Need to understand context".to_string(),
    };
    let step1_id = seq.record_step(&step1).unwrap();

    let step2 = ThoughtStep {
        task_id: Some(999),
        step_number: 2,
        thought: "Fix the bug".to_string(),
        action: Some("Edit file".to_string()),
        observation: None,
        reasoning: "Apply patch".to_string(),
    };
    let step2_id = seq.record_step(&step2).unwrap();

    // Verify Neo4j has the Step nodes
    let neo4j = state.neo4j.as_ref().unwrap();
    let ns = neo4j.namespace();
    let results = neo4j.execute_query(
        "MATCH (s:Step {id: $id, namespace: $ns}) RETURN s.id as id, s.step_number as step_number",
        vec![
            ("id", serde_json::json!(step1_id)),
            ("ns", serde_json::json!(ns))
        ]
    ).await.unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["id"], step1_id);

    // Verify FOR_TASK relationship
    let task_results = neo4j.execute_query(
        "MATCH (s:Step {id: $id, namespace: $ns})-[:FOR_TASK]->(t:Task {id: $task_id, namespace: $ns}) RETURN t.id as task_id",
        vec![
            ("id", serde_json::json!(step1_id)),
            ("ns", serde_json::json!(ns)),
            ("task_id", serde_json::json!(999))
        ]
    ).await.unwrap();

    assert_eq!(task_results.len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_application_tool_creates_neo4j_patch_nodes() {
    use syncore::portfolio::application_tool::{ApplicationTool, CodeChange};

    let state = create_neo4j_test_state().await;
    let app = ApplicationTool::new(state.clone());

    // Record a code change
    let change = CodeChange {
        file_path: "src/neo4j_test_file.rs".to_string(),
        change_type: "modify".to_string(),
        old_content: Some("old code".to_string()),
        new_content: Some("new code".to_string()),
        line_start: 10,
        line_end: 20,
        description: "Fixed memory leak".to_string(),
        task_id: Some(888),
    };

    let patch_id = app.record_change(&change).unwrap();

    // Verify Neo4j has the Patch node
    let neo4j = state.neo4j.as_ref().unwrap();
    let ns = neo4j.namespace();
    let results = neo4j
        .execute_query(
            "MATCH (p:Patch {id: $id, namespace: $ns}) RETURN p.id as id",
            vec![
                ("id", serde_json::json!(patch_id)),
                ("ns", serde_json::json!(ns)),
            ],
        )
        .await
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["id"], patch_id);

    // Verify APPLIES_TO relationship
    let file_results = neo4j.execute_query(
        "MATCH (p:Patch {id: $id, namespace: $ns})-[:APPLIES_TO]->(f:File {path: $path}) RETURN f.path as path",
        vec![
            ("id", serde_json::json!(patch_id)),
            ("ns", serde_json::json!(ns)),
            ("path", serde_json::json!("src/neo4j_test_file.rs"))
        ]
    ).await.unwrap();

    assert_eq!(file_results.len(), 1);

    // Verify FOR_TASK relationship
    let task_results = neo4j.execute_query(
        "MATCH (p:Patch {id: $id, namespace: $ns})-[:FOR_TASK]->(t:Task {id: $task_id, namespace: $ns}) RETURN t.id as task_id",
        vec![
            ("id", serde_json::json!(patch_id)),
            ("ns", serde_json::json!(ns)),
            ("task_id", serde_json::json!(888))
        ]
    ).await.unwrap();

    assert_eq!(task_results.len(), 1);
}
