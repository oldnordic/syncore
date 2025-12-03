# SynCore Configuration Guide

## Overview

SynCore uses a hierarchical configuration system that supports:
- **Configuration files**: TOML format with sensible defaults
- **Environment variables**: Override any configuration setting
- **Fallback behavior**: Graceful degradation when services are unavailable

## Configuration Files

### Default Configuration File
The main configuration file is located at `config/syncore.toml`. If the file doesn't exist, SynCore will use built-in defaults.

### Configuration Structure

```toml
[graph]
# Graph backend selection (NEW in Task 4)
backend = "sqlite"  # "sqlite" (default) or "neo4j"
sqlite_db_path = "syncore_code_graph.db"
neo4j_uri = "bolt://127.0.0.1:7687"
neo4j_user = "neo4j"
neo4j_password = ""

[paths]
db_path = "syncore.db"
code_graph_db = "syncore_code_graph.db"
cache_path = "cache"
logs_dir = "logs"

[indexing]
excluded_dirs = ["target", "node_modules", ".git"]
include_extensions = ["rs", "py", "js", "ts"]
max_file_size = 1048576  # 1MB

[embeddings]
model = "semantic"
dimensions = 384
batch_size = 32

[llm]
backend = "gguf_engine"
model = "qwen2.5-mini"
url = "local"
timeout_seconds = 30
```

## Graph Backend Configuration (Task 4)

### SQLiteGraph Backend (Default)
- **Description**: Embedded SQLite database for graph operations
- **Advantages**: No external dependencies, fast setup, reliable
- **Use case**: Development, testing, production without Neo4j
- **Configuration**:
  ```toml
  [graph]
  backend = "sqlite"
  sqlite_db_path = "syncore_code_graph.db"
  ```

### Neo4j Backend
- **Description**: External Neo4j graph database server
- **Advantages**: Advanced graph algorithms, better performance for complex queries
- **Use case**: Large-scale production, advanced graph analytics
- **Configuration**:
  ```toml
  [graph]
  backend = "neo4j"
  neo4j_uri = "bolt://127.0.0.1:7687"
  neo4j_user = "neo4j"
  neo4j_password = "your_password"
  ```

## Environment Variables

### New Environment Variables (Task 4)

```bash
# Graph backend selection
export SYNC_GRAPH_BACKEND="sqlite"  # or "neo4j"

# SQLite configuration
export SYNC_SQLITE_DB_PATH="/path/to/code_graph.db"

# Neo4j configuration
export SYNC_NEO4J_URI="bolt://127.0.0.1:7687"
export SYNC_NEO4J_USER="neo4j"
export SYNC_NEO4J_PASSWORD="your_password"

# Database paths
export DB_PATH="/path/to/syncore.db"
export SYNCORE_CODE_GRAPH_DB="/path/to/code_graph.db"
export SYNCORE_CACHE_PATH="/path/to/cache"
export SYNCORE_LOGS_DIR="/path/to/logs"
```

### Legacy Environment Variables (Supported)

For backward compatibility, SynCore also supports legacy variable names:

```bash
# Legacy graph variables (mapped to new equivalents)
export GRAPH_BACKEND="neo4j"
export GRAPH_PATH="/path/to/graph.db"
export GRAPH_URI="bolt://127.0.0.1:7687"
export GRAPH_USER="neo4j"
export GRAPH_PASS="password"

# Legacy Neo4j variables
export NEO4J_URI="bolt://127.0.0.1:7687"
export NEO4J_USER="neo4j"
export NEO4J_PASS="password"
```

## Configuration Precedence (Task 4B)

Settings are applied in the following order (highest to lowest priority):

1. **Configuration file** (`config/syncore.toml`) - PRIMARY
2. **Environment variables** - OPTIONAL overrides only
3. **Built-in defaults** - LAST RESORT

**Key Changes:**
- Config file is now the primary source of configuration
- Environment variables are optional overrides for individual fields
- No environment variables are required for default behavior
- SQLiteGraph works out-of-the-box via config file or defaults

## Usage Examples

### Basic Usage with Defaults
```bash
# Uses SQLiteGraph backend by default (no config file needed)
./target/release/syncore_mcp_stdio

# Or create config/syncore.toml for explicit control
mkdir -p config
cat > config/syncore.toml << EOF
[graph]
backend = "sqlite"
sqlite_db_path = "syncore_code_graph.db"
EOF
./target/release/syncore_mcp_stdio
```

### Using Neo4j Backend
```bash
# Method 1: Config file (recommended)
cat > config/syncore.toml << EOF
[graph]
backend = "neo4j"
neo4j_uri = "bolt://127.0.0.1:7687"
neo4j_user = "neo4j"
neo4j_password = "my_password"
EOF
./target/release/syncore_mcp_stdio

# Method 2: Environment variable override (optional)
# Assumes config file exists with other settings
export SYNC_GRAPH_BACKEND="neo4j"
export SYNC_NEO4J_PASSWORD="my_password"
./target/release/syncore_mcp_stdio
```

### Custom Database Paths
```bash
# Method 1: Config file (recommended)
cat > config/syncore.toml << EOF
[graph]
backend = "sqlite"
sqlite_db_path = "/data/my_code_graph.db"

[paths]
db_path = "/data/my_syncore.db"
EOF
./target/release/syncore_mcp_stdio

# Method 2: Environment variable override (optional)
export SYNC_SQLITE_DB_PATH="/data/my_code_graph.db"
export DB_PATH="/data/my_syncore.db"
./target/release/syncore_mcp_stdio
```

### Testing Configuration
```rust
use crate::config::SyncoreConfig;

// Load config with environment overrides
let config = SyncoreConfig::load_with_env("config/syncore.toml")?;

// Create test-optimized configuration
let test_config = SyncoreConfig::default_sqlite_test();
```

## Fallback Behavior

SynCore implements graceful fallback behavior:

1. **Invalid backend configuration**: Falls back to SQLiteGraph with default settings
2. **Missing configuration files**: Uses built-in defaults
3. **Connection failures**: Logs warning but continues without graph features
4. **Missing environment variables**: Uses configuration file or defaults

## Troubleshooting

### Common Issues

#### Graph Backend Not Working
```bash
# Check current configuration
echo "Backend: $SYNC_GRAPH_BACKEND"

# Test SQLiteGraph backend
export SYNC_GRAPH_BACKEND="sqlite"
./target/release/syncore_mcp_stdio

# Test Neo4j connectivity
export SYNC_GRAPH_BACKEND="neo4j"
./target/release/syncore_mcp_stdio
```

#### Database Path Issues
```bash
# Check database paths
ls -la $DB_PATH
ls -la $SYNC_SQLITE_DB_PATH

# Use absolute paths
export SYNC_SQLITE_DB_PATH="/full/path/to/syncore_code_graph.db"
```

#### Permission Issues
```bash
# Ensure write permissions to database directories
chmod 755 /path/to/database/dir
chmod 644 /path/to/database/file
```

### Debug Mode
```bash
# Enable debug logging
export RUST_LOG=debug
./target/release/syncore_mcp_stdio
```

## Configuration Validation

SynCore validates configuration at startup:

- **Graph backend**: Must be "sqlite" or "neo4j"
- **Database paths**: Must be accessible and writable
- **Neo4j settings**: URI, user, and password required for Neo4j backend
- **File paths**: Excluded directories and file extensions must be valid

Invalid configurations trigger fallback to safe defaults rather than failures.