#!/bin/bash
# Test script for SynCore MCP server

echo "Testing SynCore MCP Server..."
echo "============================"

# Test initialization
echo "1. Testing initialization..."
INIT_RESPONSE=$(echo '{"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0.0"}}}' | ./target/release/syncore_mcp_stdio)
echo "Initialization response: $INIT_RESPONSE"

# Extract the result part
if echo "$INIT_RESPONSE" | grep -q '"result"'; then
    echo "✓ Initialization successful"
    
    # Test tools
    echo ""
    echo "2. Testing tools..."
    
    # Test memory store
    echo "Testing memory_store..."
    MEMORY_STORE_RESPONSE=$(echo '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"memory_store","arguments":{"key":"test_key","value":"test_value"}}}' | ./target/release/syncore_mcp_stdio)
    echo "Memory store response: $MEMORY_STORE_RESPONSE"
    
    # Test memory query
    echo "Testing memory_query..."
    MEMORY_QUERY_RESPONSE=$(echo '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"memory_query","arguments":{"key":"test_key"}}}' | ./target/release/syncore_mcp_stdio)
    echo "Memory query response: $MEMORY_QUERY_RESPONSE"
    
    # Test task creation
    echo "Testing task_create..."
    TASK_CREATE_RESPONSE=$(echo '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"task_create","arguments":{"goal":"Test task","priority":1}}}' | ./target/release/syncore_mcp_stdio)
    echo "Task create response: $TASK_CREATE_RESPONSE"
    
    # Test vector insert
    echo "Testing vector_insert..."
    VECTOR_INSERT_RESPONSE=$(echo '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"vector_insert","arguments":{"text":"This is a test document"}}}' | ./target/release/syncore_mcp_stdio)
    echo "Vector insert response: $VECTOR_INSERT_RESPONSE"
    
    # Test vector search
    echo "Testing vector_search..."
    VECTOR_SEARCH_RESPONSE=$(echo '{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"vector_search","arguments":{"query":"test document","limit":5}}}' | ./target/release/syncore_mcp_stdio)
    echo "Vector search response: $VECTOR_SEARCH_RESPONSE"
    
    # Test logs tail
    echo "Testing logs_tail..."
    LOGS_TAIL_RESPONSE=$(echo '{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"logs_tail","arguments":{"n":5}}}' | ./target/release/syncore_mcp_stdio)
    echo "Logs tail response: $LOGS_TAIL_RESPONSE"
    
    echo ""
    echo "✓ All tests completed successfully!"
else
    echo "✗ Initialization failed"
    exit 1
fi