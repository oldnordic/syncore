#!/bin/bash

# Test script for SynCore MCP server

echo "🧠 Testing SynCore MCP Server"
echo "==============================="

# Test MCP initialization
echo "1. Testing mcp.initialize..."
echo '{"jsonrpc":"2.0","method":"mcp.initialize","id":1}' | nc 127.0.0.1 8080
echo ""

# Test tool listing
echo "2. Testing mcp.list_tools..."
echo '{"jsonrpc":"2.0","method":"mcp.list_tools","id":2}' | nc 127.0.0.1 8080
echo ""

# Test tool calling (memory.store)
echo "3. Testing mcp.call_tool (memory.store)..."
echo '{"jsonrpc":"2.0","method":"mcp.call_tool","params":{"name":"memory.store","arguments":{"key":"test_key","value":"test_value"}},"id":3}' | nc 127.0.0.1 8080
echo ""

# Test tool calling (task.create)
echo "4. Testing mcp.call_tool (task.create)..."
echo '{"jsonrpc":"2.0","method":"mcp.call_tool","params":{"name":"task.create","arguments":{"goal":"Implement MCP server"}},"id":4}' | nc 127.0.0.1 8080
echo ""

# Test tool calling (task.next)
echo "5. Testing mcp.call_tool (task.next)..."
echo '{"jsonrpc":"2.0","method":"mcp.call_tool","params":{"name":"task.next","arguments":{}},"id":5}' | nc 127.0.0.1 8080
echo ""

# Test server description
echo "6. Testing mcp.describe..."
echo '{"jsonrpc":"2.0","method":"mcp.describe","id":6}' | nc 127.0.0.1 8080
echo ""

echo "✅ MCP testing complete!"