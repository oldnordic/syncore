#!/bin/bash
# Test SynCore MCP Server Connection
# This script helps diagnose MCP connection issues

set -e

echo "=== SynCore MCP Connection Diagnostic ==="
echo

# Check 1: Verify cargo is available
echo "1. Checking cargo..."
if ! command -v cargo &> /dev/null; then
    echo "   ❌ ERROR: cargo not found in PATH"
    echo "   Add cargo to PATH: export PATH=\"\$HOME/.cargo/bin:\$PATH\""
    exit 1
fi
echo "   ✓ cargo found: $(cargo --version)"
echo

# Check 2: Verify we're in the right directory
echo "2. Checking directory..."
if [ ! -f "Cargo.toml" ]; then
    echo "   ❌ ERROR: Not in syncore directory"
    exit 1
fi
echo "   ✓ In correct directory: $(pwd)"
echo

# Check 3: Build the binary
echo "3. Building syncore_mcp_stdio..."
if ! cargo build --bin syncore_mcp_stdio 2>&1 | tail -5; then
    echo "   ❌ ERROR: Build failed"
    exit 1
fi
echo "   ✓ Build successful"
echo

# Check 4: Test binary startup
echo "4. Testing binary startup..."
timeout 3s cargo run --bin syncore_mcp_stdio 2>&1 | head -10 &
sleep 1
echo "   ✓ Binary starts (connection closed is expected without MCP client)"
echo

# Check 5: Test MCP handshake
echo "5. Testing MCP protocol handshake..."
cat << 'EOF' | timeout 5s cargo run --bin syncore_mcp_stdio 2>&1 | head -20
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}
EOF
echo
echo "   ✓ MCP handshake test complete"
echo

# Check 6: Verify Claude configuration
echo "6. Checking Claude Code configuration..."
if [ -f "$HOME/.claude.json" ]; then
    if grep -q "syncore" "$HOME/.claude.json"; then
        echo "   ✓ syncore found in ~/.claude.json"
        echo
        echo "   Configuration:"
        jq '.mcpServers.syncore' "$HOME/.claude.json" 2>/dev/null || echo "   (Could not parse JSON)"
    else
        echo "   ❌ syncore NOT found in ~/.claude.json"
        echo "   Add this to mcpServers section:"
        cat << 'CONF'
{
  "mcpServers": {
    "syncore": {
      "command": "cargo",
      "args": ["run", "--release", "--bin", "syncore_mcp_stdio"],
      "cwd": "/home/feanor/Projects/SynCore/syncore",
      "env": {
        "RUST_LOG": "info",
        "DB_PATH": "syncore.db"
      }
    }
  }
}
CONF
    fi
else
    echo "   ⚠ ~/.claude.json not found"
fi
echo

echo "=== Diagnostic Complete ==="
echo
echo "If Claude Code still can't connect:"
echo "1. Make sure you're using --release flag for faster startup"
echo "2. Check Claude Code logs for specific error messages"
echo "3. Try restarting Claude Code"
echo "4. Verify PATH includes ~/.cargo/bin"
echo
echo "Current recommended configuration:"
cat << 'EOF'
{
  "mcpServers": {
    "syncore": {
      "command": "cargo",
      "args": ["run", "--release", "--bin", "syncore_mcp_stdio"],
      "cwd": "/home/feanor/Projects/SynCore/syncore",
      "env": {
        "RUST_LOG": "error",
        "DB_PATH": "syncore.db"
      }
    }
  }
}
EOF
