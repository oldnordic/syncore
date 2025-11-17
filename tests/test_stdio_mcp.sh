#!/bin/bash

# Test script for SynCore MCP stdio server

echo "🧠 Testing SynCore MCP STDIO Server"
echo "=================================="

# Build the stdio server
echo "📦 Building stdio MCP server..."
cargo build --bin syncore_mcp_stdio

if [ $? -ne 0 ]; then
    echo "❌ Build failed!"
    exit 1
fi

echo "✅ Build successful!"

# Test 1: Initialize
echo ""
echo "1. Testing initialize..."
echo '{"jsonrpc":"2.0","method":"initialize","params":{},"id":1}' | ./target/debug/syncore_mcp_stdio
echo ""

# Test 2: List tools
echo ""
echo "2. Testing tools/list..."
echo '{"jsonrpc":"2.0","method":"tools/list","params":{},"id":2}' | ./target/debug/syncore_mcp_stdio
echo ""

# Test 3: Call memory.store
echo ""
echo "3. Testing tools/call (memory.store)..."
echo '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"memory.store","arguments":{"key":"test_key","value":"test_value"}},"id":3}' | ./target/debug/syncore_mcp_stdio
echo ""

# Test 4: Call task.create
echo ""
echo "4. Testing tools/call (task.create)..."
echo '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"task.create","arguments":{"goal":"Test MCP stdio"}},"id":4}' | ./target/debug/syncore_mcp_stdio
echo ""

# Test 5: Call task.next
echo ""
echo "5. Testing tools/call (task.next)..."
echo '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"task.next","arguments":{}},"id":5}' | ./target/debug/syncore_mcp_stdio
echo ""

# Test 6: Error handling (unknown method)
echo ""
echo "6. Testing error handling (unknown method)..."
echo '{"jsonrpc":"2.0","method":"unknown.method","params":{},"id":6}' | ./target/debug/syncore_mcp_stdio
echo ""

echo "✅ MCP stdio testing complete!"