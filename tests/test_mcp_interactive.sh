#!/bin/bash
# Interactive test for SynCore MCP server showing tool responses

echo "Interactive SynCore MCP Server Test"
echo "===================================="

# Function to send a single request and get response
send_request() {
    local request="$1"
    echo "Sending: $request"
    echo "Response:"
    echo "$request" | ./target/release/syncore_mcp_stdio
    echo "----------------------------------------"
}

# Test initialization
echo "1. Testing initialization..."
INIT_REQUEST='{"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0.0"}}}'
send_request "$INIT_REQUEST"

echo ""
echo "2. Testing initialized notification..."
INIT_NOTIFICATION='{"jsonrpc":"2.0","method":"notifications/initialized"}'
send_request "$INIT_NOTIFICATION"

echo ""
echo "3. Testing memory_store..."
MEMORY_STORE_REQUEST='{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"memory_store","arguments":{"key":"test_key","value":"test_value"}}}'
send_request "$MEMORY_STORE_REQUEST"

echo ""
echo "4. Testing memory_query..."
MEMORY_QUERY_REQUEST='{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"memory_query","arguments":{"key":"test_key"}}}'
send_request "$MEMORY_QUERY_REQUEST"

echo ""
echo "5. Testing task_create..."
TASK_CREATE_REQUEST='{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"task_create","arguments":{"goal":"Test task from interactive test","priority":1}}}'
send_request "$TASK_CREATE_REQUEST"

echo ""
echo "6. Testing vector_insert..."
VECTOR_INSERT_REQUEST='{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"vector_insert","arguments":{"text":"This is a test document for interactive testing"}}}'
send_request "$VECTOR_INSERT_REQUEST"

echo ""
echo "7. Testing vector_search..."
VECTOR_SEARCH_REQUEST='{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"vector_search","arguments":{"query":"test document","limit":5}}}'
send_request "$VECTOR_SEARCH_REQUEST"

echo ""
echo "8. Testing logs_tail..."
LOGS_TAIL_REQUEST='{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"logs_tail","arguments":{"n":5}}}'
send_request "$LOGS_TAIL_REQUEST"

echo ""
echo "All tests completed successfully!"