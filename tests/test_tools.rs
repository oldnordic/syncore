use syncore::mcp::tool_metadata::list_all_metadata;

fn main() {
    let tools = list_all_metadata();
    println!("Total tools in registry: {}", tools.len());
    for tool in tools {
        println!("- {}", tool.name);
    }
}
