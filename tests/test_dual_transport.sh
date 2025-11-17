#!/bin/bash
set -e

echo "Testing Syncore dual-transport MCP server..."

# Clean up old instances
pkill -f syncore_mcp_stdio 2>/dev/null || true
sleep 1

# Start server with stdin from /dev/null to prevent immediate exit
echo "Starting server..."
cd /home/feanor/Projects/SynCore/syncore
HTTP_PORT=3001 ./target/debug/syncore_mcp_stdio < /dev/null 2>&1 &
PID=$!

sleep 3

echo "Server PID: $PID"
echo ""

# Check if process is still running
if ! kill -0 $PID 2>/dev/null; then
    echo "ERROR: Server process died"
    exit 1
fi

# Check HTTP port
echo "Checking HTTP port 3001..."
if lsof -i :3001 2>/dev/null | grep -q LISTEN; then
    echo "✓ HTTP/SSE server is listening on port 3001"
else
    echo "✗ HTTP/SSE server NOT listening"
fi

# Try to connect to SSE endpoint
echo ""
echo "Testing SSE endpoint..."
RESPONSE=$(curl -s -m 5 http://127.0.0.1:3001/sse 2>&1 | head -5)
echo "Response: $RESPONSE"

# Cleanup
echo ""
echo "Cleaning up..."
kill $PID 2>/dev/null || true
wait $PID 2>/dev/null || true

echo "Test complete"
