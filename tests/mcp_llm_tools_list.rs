//! Test that LLM MCP tools are properly registered

use syncore::mcp::protocol::list_tools;

#[tokio::test]
async fn test_llm_tools_in_list() {
    let tools = list_tools().await;

    // Find our tools
    let health_tool = tools.iter().find(|t| t.name == "llm.health");
    let metrics_tool = tools.iter().find(|t| t.name == "llm.metrics");

    // Verify tools exist
    assert!(health_tool.is_some(), "llm.health tool not found in list");
    assert!(metrics_tool.is_some(), "llm.metrics tool not found in list");

    // Verify tool details
    let health = health_tool.unwrap();
    assert_eq!(health.name, "llm.health");
    assert!(health.description.contains("health"));
    assert_eq!(health.input_schema, "schemas/llm_health.json");

    let metrics = metrics_tool.unwrap();
    assert_eq!(metrics.name, "llm.metrics");
    assert!(metrics.description.contains("metrics"));
    assert_eq!(metrics.input_schema, "schemas/llm_metrics.json");
}
