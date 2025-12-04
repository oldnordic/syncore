use anyhow::Result;
use serde_json::json;
use syncore::mcp_tools::translator::{translate_llm_output, TargetSchema};

fn main() -> Result<()> {
    // Test 1: Missing required fields error format
    let incomplete_input = json!({
        "parent_tasks": [{
            "id": "1.0",
            "title": "Test Task"
        }]
    }).to_string();

    let result = translate_llm_output(&incomplete_input, TargetSchema::TaskBreakdown)?;
    println!("=== Missing fields test ===");
    println!("Error: {:?}", result.get("error"));
    println!("Missing fields: {:?}", result.get("missing_fields"));

    // Test 2: PriorityResult missing priorities
    let missing_priorities_input = json!({
        "results": [{
            "task_id": "1.0",
            "priority": "High"
        }]
    }).to_string();

    let result2 = translate_llm_output(&missing_priorities_input, TargetSchema::PriorityResult)?;
    println!("=== PriorityResult test ===");
    println!("Error: {:?}", result2.get("error"));
    println!("Missing fields: {:?}", result2.get("missing_fields"));

    Ok(())
}