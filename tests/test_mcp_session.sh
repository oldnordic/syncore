#!/bin/bash
# Single session test for SynCore MCP server

echo "Single Session SynCore MCP Server Test"
echo "======================================"

# Create a temporary file for the conversation
TEMP_FILE=$(mktemp)

# Build the conversation
echo '{"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0.0"}}}' > $TEMP_FILE
echo '{"jsonrpc":"2.0","method":"notifications/initialized"}' >> $TEMP_FILE
echo '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"memory_store","arguments":{"key":"test_key","value":"test_value"}}}' >> $TEMP_FILE
echo '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"memory_query","arguments":{"key":"test_key"}}}' >> $TEMP_FILE
echo '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"task_create","arguments":{"goal":"Test task","priority":1}}}' >> $TEMP_FILE
echo '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"vector_insert","arguments":{"text":"This is a test document"}}}' >> $TEMP_FILE
echo '{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"vector_search","arguments":{"query":"test document","limit":5}}}' >> $TEMP_FILE
echo '{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"logs_tail","arguments":{"n":5}}}' >> $TEMP_FILE

# Run the conversation
echo "Running MCP conversation..."
cat $TEMP_FILE | ./target/release/syncore_mcp_stdio

# Clean up
rm -f $TEMP_FILE

echo ""
echo "Test completed."