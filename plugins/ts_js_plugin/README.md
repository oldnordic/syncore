# TypeScript/JavaScript Plugin for SynCore

This plugin provides TypeScript and JavaScript analysis capabilities for the SynCore code analysis platform.

## Features

- **Static Analysis**: Extract classes, interfaces, functions, variables, and other code constructs
- **Dependency Analysis**: Track imports, exports, and module relationships
- **Diagnostics Integration**: Support for TypeScript Server, ESLint, and Prettier diagnostics
- **Code Graph Generation**: Create entities and edges for code graph construction
- **Project Analysis**: Full project analysis combining indexing and diagnostics

## Supported Tasks

The plugin supports the following tasks:

1. **ts_js_index_directory**: Index TypeScript/JavaScript files in a directory
2. **ts_js_lsp_diagnostics**: Run TypeScript Server diagnostics
3. **ts_js_eslint**: Run ESLint analysis
4. **ts_js_prettier**: Run Prettier formatting checks
5. **ts_js_full_project_analysis**: Complete project analysis combining all features

## Configuration

The plugin accepts configuration through the `ts_js_config` parameter:

```json
{
  "ts_js_config": {
    "tsserver_path": "/path/to/tsserver",
    "eslint_path": "eslint",
    "prettier_path": "prettier",
    "eslint_config": "/path/to/.eslintrc.json"
  }
}
```

### Configuration Options

- `tsserver_path`: Path to TypeScript Server executable (optional)
- `eslint_path`: Path to ESLint executable (defaults to "eslint")
- `prettier_path`: Path to Prettier executable (defaults to "prettier")
- `eslint_config`: Path to ESLint configuration file (optional)

## Usage Examples

### Indexing a Directory

```json
{
  "event": "execute",
  "task": "ts_js_index_directory",
  "params": {
    "root_path": "/path/to/project"
  }
}
```

### Running LSP Diagnostics

```json
{
  "event": "execute",
  "task": "ts_js_lsp_diagnostics",
  "params": {
    "project_root": "/path/to/project",
    "tsserver_path": "/usr/bin/tsserver"
  }
}
```

### Running ESLint

```json
{
  "event": "execute",
  "task": "ts_js_eslint",
  "params": {
    "project_root": "/path/to/project",
    "eslint_path": "eslint",
    "eslint_config": "/path/to/.eslintrc.json"
  }
}
```

### Running Prettier

```json
{
  "event": "execute",
  "task": "ts_js_prettier",
  "params": {
    "project_root": "/path/to/project",
    "prettier_path": "prettier"
  }
}
```

### Full Project Analysis

```json
{
  "event": "execute",
  "task": "ts_js_full_project_analysis",
  "params": {
    "project_root": "/path/to/project",
    "ts_js_config": {
      "eslint_path": "eslint",
      "prettier_path": "prettier"
    }
  }
}
```

## Output Format

### Entities

The plugin extracts the following entity types:

- **Class**: TypeScript/JavaScript classes
- **Interface**: TypeScript interfaces
- **Function**: Function declarations
- **Method**: Class methods
- **Property**: Class properties
- **Variable**: Variable declarations
- **Import**: Imported symbols
- **Export**: Exported symbols

Each entity includes:
- `file_path`: Path to the source file
- `name`: Name of the entity
- `kind`: Type of entity
- `span`: Location in the source file (start/end line and column)
- `signature`: Optional signature information
- `extra`: Optional additional metadata

### Edges

The plugin generates the following edge types:

- **Contains**: Parent-child relationships (e.g., class contains methods)
- **Imports**: Import relationships between modules
- **Exports**: Export relationships

Each edge includes:
- `from`: Source entity identifier
- `to`: Target entity identifier
- `kind`: Type of relationship

### Diagnostics

The plugin provides diagnostics from multiple tools:

- **TypeScript Server**: Type errors, compilation warnings
- **ESLint**: Code quality issues, style violations
- **Prettier**: Formatting issues

Each diagnostic includes:
- `file_path`: Path to the file with the issue
- `line`: Line number
- `column`: Column number
- `severity`: Error level (Error, Warning, Info)
- `code`: Diagnostic code
- `message`: Description of the issue
- `tool`: Source of the diagnostic (tsserver, eslint, prettier)

## Requirements

- Rust 1.70 or later
- Node.js (for external tools)
- TypeScript Server (optional, for LSP diagnostics)
- ESLint (optional, for linting)
- Prettier (optional, for formatting checks)

## Building

```bash
cargo build --release
```

## Testing

Run the test suite:

```bash
cargo test
```

The test suite includes:
- DLR contract compliance tests
- Indexing functionality tests
- Diagnostics parsing tests
- Integration tests with mock projects

## License

This plugin is part of the SynCore project and is licensed under the same terms as the main project.

## Contributing

Please refer to the main SynCore project documentation for contribution guidelines.