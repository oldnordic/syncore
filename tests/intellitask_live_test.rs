/*
/// Live integration test for IntelliTask with real Ollama instance
///
/// NOTE: This test requires Ollama to be running locally
/// Run: ollama serve
/// Pull model: ollama pull qwen2.5-coder:3B
///
/// To run this test: cargo test --test intellitask_live_test -- --nocapture --ignored
use anyhow::Result;
use syncore::intellitask::IntelliTask;
use syncore::ollama::{OllamaClient, OllamaConfig};

#[test]
#[ignore] // Ignore by default since it requires Ollama to be running
fn test_intellitask_with_real_ollama() -> Result<()> {
    println!("\n=== Testing IntelliTask with Real Ollama ===");

    // Check if Ollama is available
    let config = OllamaConfig::default();
    println!("Using Ollama CLI with model: {}", config.model);
    println!("Timeout: {}s", config.timeout_secs);

    let client = OllamaClient::new(config.clone())?;

    // Create IntelliTask instance (it creates its own runtime)
    let intellitask = IntelliTask::new(client);
    println!("✅ IntelliTask instance created");

    // Test PRD parsing
    let test_prd = r#"\
# Feature: User Authentication System

## Overview
Implement a secure user authentication system with JWT tokens, password hashing, and session management.

## Requirements
1. User registration with email validation
2. Secure password storage using bcrypt
3. JWT token generation and validation
4. Session management with Redis
5. Password reset functionality
6. Role-based access control (RBAC)

## Technical Details
- Use Rust with Actix-web framework
- PostgreSQL for user storage
- Redis for session management
- JWT for stateless authentication
- bcrypt for password hashing

## Acceptance Criteria
- Users can register with email and password
- Passwords are never stored in plaintext
- JWT tokens expire after 1 hour
- Sessions can be revoked
- Password reset via email link
- Admin users have elevated permissions
"#;

    println!("\n=== Generating Task Breakdown from PRD ===");
    let breakdown = intellitask.generate_tasks_from_prd(test_prd);

    match breakdown {
        Ok(breakdown) => {
            println!("✅ Task breakdown generated successfully!");
            println!("\nPRD Title: {}", breakdown.prd_title);
            println!("Overall Complexity: {:?}", breakdown.estimated_complexity);
            println!("Number of Parent Tasks: {}", breakdown.parent_tasks.len());
            println!("\nParent Tasks:");

            for (i, task) in breakdown.parent_tasks.iter().enumerate() {
                println!("\n{}. {} (ID: {})", i + 1, task.title, task.id);
                println!("   Description: {}", task.description);
                println!("   Complexity: {:?}", task.complexity);
                println!("   Estimated Hours: {}", task.estimated_hours);
                println!("   Dependencies: {:?}", task.dependencies);
                println!(
                    "   Subtasks: {} (to be generated later)",
                    task.subtasks.len()
                );
            }

            println!("\n\nRelevant Files ({})", breakdown.relevant_files.len());
            for file in &breakdown.relevant_files {
                println!("  - {} ({:?}): {}", file.path, file.action, file.purpose);
            }

            // Verify structure
            assert!(
                !breakdown.parent_tasks.is_empty(),
                "Should generate parent tasks"
            );
            assert!(
                !breakdown.relevant_files.is_empty(),
                "Should identify relevant files"
            );

            println!("\n✅ IntelliTask is fully functional with real Ollama!");
        }
        Err(e) => {
            println!("❌ Task breakdown generation failed: {}", e);
            return Err(e);
        }
    }

    Ok(())
}

#[test]
#[ignore] // Ignore by default since it requires Ollama
fn test_intellitask_subtask_generation() -> Result<()> {
    println!("\n=== Testing IntelliTask Subtask Generation ===");

    let config = OllamaConfig::default();
    let client = OllamaClient::new(config)?;
    let intellitask = IntelliTask::new(client);

    // Create a parent task
    let parent_task = syncore::intellitask::ParentTask {
        id: "1.0".to_string(),
        title: "Implement User Registration".to_string(),
        description: "Create user registration endpoint with email validation and password hashing"
            .to_string(),
        subtasks: vec![],
        dependencies: vec![],
        complexity: syncore::intellitask::Complexity::Moderate,
        estimated_hours: 8.0,
    };

    println!("Parent Task: {}", parent_task.title);
    println!("Generating subtasks...");

    let subtasks = intellitask.generate_subtasks(&parent_task, "No additional context");

    match subtasks {
        Ok(subtasks) => {
            println!("✅ Generated {} subtasks", subtasks.len());
            for (i, subtask) in subtasks.iter().enumerate() {
                println!("\n{}. Subtask {}", i + 1, subtask.id);
                println!("   Description: {}", subtask.description);
                println!("   Complexity: {:?}", subtask.complexity);
                println!("   Estimated Hours: {}", subtask.estimated_hours);
                println!("   Dependencies: {:?}", subtask.dependencies);
                println!("   Files to Modify: {:?}", subtask.files_to_modify);
                println!("   Acceptance Criteria:");
                for criterion in &subtask.acceptance_criteria {
                    println!("     - {}", criterion);
                }
            }

            assert!(!subtasks.is_empty(), "Should generate subtasks");
            println!("\n✅ Subtask generation working correctly!");
        }
        Err(e) => {
            println!("❌ Subtask generation failed: {}", e);
            return Err(e);
        }
    }

    Ok(())
}

#[test]
#[ignore] // Ignore by default since it requires Ollama
fn test_intellitask_task_prioritization() -> Result<()> {
    println!("\n=== Testing IntelliTask Task Prioritization ===");

    let config = OllamaConfig::default();
    let client = OllamaClient::new(config)?;
    let intellitask = IntelliTask::new(client);

    // Create test tasks
    let tasks = vec![
        syncore::intellitask::ParentTask {
            id: "1.0".to_string(),
            title: "Setup Database Schema".to_string(),
            description: "Create initial database tables and indexes".to_string(),
            subtasks: vec![],
            dependencies: vec![],
            complexity: syncore::intellitask::Complexity::Simple,
            estimated_hours: 4.0,
        },
        syncore::intellitask::ParentTask {
            id: "2.0".to_string(),
            title: "Implement Authentication".to_string(),
            description: "JWT token generation and validation".to_string(),
            subtasks: vec![],
            dependencies: vec!["1.0".to_string()],
            complexity: syncore::intellitask::Complexity::Moderate,
            estimated_hours: 8.0,
        },
        syncore::intellitask::ParentTask {
            id: "3.0".to_string(),
            title: "Create User Dashboard".to_string(),
            description: "Frontend dashboard for authenticated users".to_string(),
            subtasks: vec![],
            dependencies: vec!["2.0".to_string()],
            complexity: syncore::intellitask::Complexity::Complex,
            estimated_hours: 16.0,
        },
    ];

    let business_context = "E-commerce platform MVP - prioritize core authentication features";

    println!("Analyzing {} tasks for prioritization...", tasks.len());

    let prioritized = intellitask.prioritize_tasks(&tasks, business_context);

    match prioritized {
        Ok(priorities) => {
            println!("✅ Task prioritization completed");
            for (task_id, priority) in &priorities {
                println!("  Task {} -> Priority: {:?}", task_id, priority);
            }

            assert!(!priorities.is_empty(), "Should return priorities");
            println!("\n✅ Task prioritization working correctly!");
        }
        Err(e) => {
            println!("❌ Task prioritization failed: {}", e);
            return Err(e);
        }
    }

    Ok(())
}

#[test]
#[ignore] // CLI mode error simulation requires ollama to be unavailable
fn test_intellitask_without_ollama_shows_clear_error() -> Result<()> {
    println!("\n=== Testing IntelliTask Error Handling Without Ollama ===");

    // Use config with short timeout
    let mut config = OllamaConfig::default();
    config.timeout_secs = 1; // Quick timeout

    let client = OllamaClient::new(config)?;
    let intellitask = IntelliTask::new(client);

    let test_prd = "Simple test PRD";

    let result = intellitask.generate_tasks_from_prd(test_prd);

    match result {
        Ok(_) => {
            println!("❌ Expected error but got success - this shouldn't happen!");
            panic!("Expected connection error");
        }
        Err(e) => {
            let error_msg = e.to_string();
            println!("✅ Got expected error: {}", error_msg);
            assert!(
                error_msg.contains("error")
                    || error_msg.contains("connection")
                    || error_msg.contains("failed"),
                "Error message should indicate connection failure"
            );
            println!("✅ IntelliTask properly reports when Ollama is not available");
        }
    }

    Ok(())
}
*/
