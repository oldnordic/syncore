#!/bin/bash
# Test the syncore MCP binary directly

BINARY="/home/feanor/Projects/SynCore/syncore/target/release/syncore_mcp_stdio"

echo "Testing SynCore MCP stdio binary..."
echo "====================================="
echo

# Test 1: Check if binary exists
if [ ! -f "$BINARY" ]; then
    echo "❌ Binary not found at: $BINARY"
    exit 1
fi
echo "✅ Binary exists"

# Test 2: Check if binary is executable
if [ ! -x "$BINARY" ]; then
    echo "❌ Binary is not executable"
    exit 1
fi
echo "✅ Binary is executable"

# Test 3: Test MCP handshake
echo
echo "Testing MCP protocol handshake..."
echo "---------------------------------"

# Send initialize request and initialized notification
(
    echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}'
    sleep 0.2
    echo '{"jsonrpc":"2.0","method":"notifications/initialized"}'
    sleep 0.2
    echo '{"jsonrpc":"2.0","id":2,"method":"tools/list"}'
    sleep 1
) | timeout 3s "$BINARY" 2>/dev/null | {
    # Read all responses
    RESPONSES=$(cat)

    # Check if we got tools list
    if echo "$RESPONSES" | grep -q '"tools"'; then
        echo "✅ MCP handshake successful"
        echo "✅ Tools list received"
        echo
        echo "Available tools:"
        echo "$RESPONSES" | jq -r '.result.tools[]?.name // empty' 2>/dev/null | sed 's/^/  - /'
        exit 0
    else
        echo "❌ MCP handshake failed"
        echo "Response:"
        echo "$RESPONSES"
        exit 1
    fi
}

EXIT_CODE=$?
echo
if [ $EXIT_CODE -eq 0 ]; then
    echo "✅ All tests passed - Binary is ready for Claude Code!"
else
    echo "❌ Tests failed"
fi
exit $EXIT_CODE
