use std::env;

fn main() {
    // Initialize logger
    env_logger::init();
    
    // Import the function
    use syncore::mcp::tool_metadata::list_all_metadata;
    
    let tools = list_all_metadata();
    println!("Total tools in metadata: {}", tools.len());
    
    for tool in tools {
        println!("- {}", tool.name);
    }
}