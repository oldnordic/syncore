#!/bin/bash

# Test script for SynCore MCP server interaction
echo "🧠 Testing SynCore MCP Server..."
echo "================================"

# Path to the compiled binary
BINARY="./target/debug/syncore_mcp_stdio"

# Check if binary exists
if [ ! -f "$BINARY" ]; then
    echo "❌ Binary not found. Building..."
    cargo build --bin syncore_mcp_stdio
    if [ $? -ne 0 ]; then
        echo "❌ Build failed"
        exit 1
    fi
fi

echo "✅ Binary found: $BINARY"

# Test 1: Initialize
echo ""
echo "📡 Test 1: Initialize"
echo "----------------------"
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0.0"}}}' | $BINARY

# Test 2: List Tools
echo ""
echo "🔧 Test 2: List Tools"
echo "----------------------"
echo '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' | $BINARY

# Test 3: Store Memory
echo ""
echo "💾 Test 3: Store Memory"
echo "-----------------------"
echo '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"memory.store","arguments":{"key":"test_key","value":"test_value"}}}' | $BINARY

# Test 4: Query Memory
echo ""
echo "🔍 Test 4: Query Memory"
echo "-----------------------"
echo '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"memory.query","arguments":{"key":"test_key"}}}' | $BINARY

# Test 5: Create Task
echo ""
echo "📋 Test 5: Create Task"
echo "----------------------"
echo '{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"task.create","arguments":{"goal":"Test MCP integration","priority":1}}}' | $BINARY

# Test 6: Next Task
echo ""
echo "🎯 Test 6: Next Task"
echo "---------------------"
echo '{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"task.next","arguments":{}}}' | $BINARY

# Test 7: Vector Search
echo ""
echo "🔎 Test 7: Vector Search"
echo "------------------------"
echo '{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"vector.search","arguments":{"query":"test search","k":5}}}' | $BINARY

# Test 8: Logs Tail
echo ""
echo "📊 Test 8: Logs Tail"
echo "---------------------"
echo '{"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":"logs.tail","arguments":{"n":5}}}' | $BINARY

echo ""
echo "✅ MCP Server Tests Complete!"
```
