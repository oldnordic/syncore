#!/bin/bash

# Dual-Backend Parity Test Runner
# This script runs comprehensive parity tests between SQLiteGraph and Neo4j backends

set -e

echo "🚀 Starting SynCore Dual-Backend Parity Test Suite"
echo "=================================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if required tools are available
check_dependencies() {
    print_status "Checking dependencies..."
    
    if ! command -v cargo &> /dev/null; then
        print_error "cargo not found. Please install Rust."
        exit 1
    fi
    
    if ! command -v sqlite3 &> /dev/null; then
        print_warning "sqlite3 not found. Some tests may fail."
    fi
    
    # Check if Neo4j is running (optional)
    if curl -s http://localhost:7474 > /dev/null 2>&1; then
        print_success "Neo4j detected at http://localhost:7474"
        NEO4J_AVAILABLE=true
    else
        print_warning "Neo4j not detected. Neo4j tests will be skipped."
        NEO4J_AVAILABLE=false
    fi
}

# Build the project
build_project() {
    print_status "Building SynCore project..."
    
    if cargo build --release; then
        print_success "Build completed successfully"
    else
        print_error "Build failed"
        exit 1
    fi
}

# Run SQLite backend tests
run_sqlite_tests() {
    print_status "Running SQLite backend parity tests..."
    
    export SYNC_BACKEND=sqlite
    
    # Run individual test files
    test_files=(
        "dual_parity/crud_parity_tests"
        "dual_parity/relationship_parity_tests"
        "dual_parity/pattern_parity_tests"
        "dual_parity/raggraph_parity_tests"
        "dual_parity/error_behavior_parity_tests"
        "dual_parity/ordering_parity_tests"
    )
    
    sqlite_passed=0
    sqlite_total=0
    
    for test_file in "${test_files[@]}"; do
        print_status "Running $test_file..."
        sqlite_total=$((sqlite_total + 1))
        
        if cargo test --test-threads=1 "$test_file" 2>/dev/null; then
            print_success "✓ $test_file passed"
            sqlite_passed=$((sqlite_passed + 1))
        else
            print_error "✗ $test_file failed"
        fi
    done
    
    print_status "SQLite Results: $sqlite_passed/$sqlite_total tests passed"
}

# Run Neo4j backend tests
run_neo4j_tests() {
    if [ "$NEO4J_AVAILABLE" = false ]; then
        print_warning "Skipping Neo4j tests - Neo4j not available"
        return
    fi
    
    print_status "Running Neo4j backend parity tests..."
    
    export SYNC_BACKEND=neo4j
    
    # Run individual test files
    test_files=(
        "dual_parity/crud_parity_tests"
        "dual_parity/relationship_parity_tests"
        "dual_parity/pattern_parity_tests"
        "dual_parity/raggraph_parity_tests"
        "dual_parity/error_behavior_parity_tests"
        "dual_parity/ordering_parity_tests"
    )
    
    neo4j_passed=0
    neo4j_total=0
    
    for test_file in "${test_files[@]}"; do
        print_status "Running $test_file..."
        neo4j_total=$((neo4j_total + 1))
        
        if cargo test --test-threads=1 "$test_file" 2>/dev/null; then
            print_success "✓ $test_file passed"
            neo4j_passed=$((neo4j_passed + 1))
        else
            print_error "✗ $test_file failed"
        fi
    done
    
    print_status "Neo4j Results: $neo4j_passed/$neo4j_total tests passed"
}

# Run comparison tests
run_comparison_tests() {
    if [ "$NEO4J_AVAILABLE" = false ]; then
        print_warning "Skipping comparison tests - Neo4j not available"
        return
    fi
    
    print_status "Running backend comparison tests..."
    
    # This would run tests that compare results between backends
    if cargo test --test-threads=1 "dual_parity::integration_tests::test_backend_comparison" 2>/dev/null; then
        print_success "✓ Backend comparison tests passed"
    else
        print_warning "⚠ Backend comparison tests failed or incomplete"
    fi
}

# Generate test report
generate_report() {
    print_status "Generating test report..."
    
    cat > "dual_parity_test_report_$(date +%Y%m%d_%H%M%S).md" << EOF
# SynCore Dual-Backend Parity Test Report

**Date:** $(date)
**Backend:** SQLiteGraph + Neo4j (if available)

## Test Summary

### SQLite Backend Tests
- Total Tests: $sqlite_total
- Passed: $sqlite_passed
- Failed: $((sqlite_total - sqlite_passed))
- Success Rate: $(( sqlite_passed * 100 / sqlite_total ))%

### Neo4j Backend Tests
- Total Tests: $neo4j_total
- Passed: $neo4j_passed
- Failed: $((neo4j_total - neo4j_passed))
- Success Rate: $(( neo4j_passed * 100 / neo4j_total ))%

## Test Categories

1. **CRUD Parity Tests** - Create, Read, Update, Delete operations
2. **Relationship Parity Tests** - Relationship creation, traversal, deletion
3. **Pattern Parity Tests** - Graph pattern matching and queries
4. **RAGGraph Parity Tests** - Embeddings, tasks, memories operations
5. **Error Behavior Parity Tests** - Error handling and edge cases
6. **Ordering Parity Tests** - Deterministic ordering behavior

## Environment

- Rust: $(rustc --version)
- OS: $(uname -s)
- SQLite: $(sqlite3 --version | head -n1 || echo "Not available")
- Neo4j: $([ "$NEO4J_AVAILABLE" = true ] && echo "Available" || echo "Not available")

## Notes

- Tests run with single thread to ensure deterministic behavior
- Neo4j tests are skipped if Neo4j is not available
- All tests use temporary databases that are cleaned up after execution

EOF

    print_success "Test report generated"
}

# Cleanup function
cleanup() {
    print_status "Cleaning up temporary files..."
    
    # Remove any temporary test databases
    find . -name "*test_parity*.db" -delete 2>/dev/null || true
    find . -name "*test_ordering_parity*.db" -delete 2>/dev/null || true
    
    print_success "Cleanup completed"
}

# Main execution
main() {
    # Trap cleanup on exit
    trap cleanup EXIT
    
    check_dependencies
    build_project
    
    print_status "Starting parity test execution..."
    echo ""
    
    run_sqlite_tests
    echo ""
    
    run_neo4j_tests
    echo ""
    
    run_comparison_tests
    echo ""
    
    generate_report
    
    print_success "Dual-Backend Parity Test Suite completed!"
    echo ""
    print_status "Summary:"
    print_status "- SQLite tests: $sqlite_passed/$sqlite_total passed"
    if [ "$NEO4J_AVAILABLE" = true ]; then
        print_status "- Neo4j tests: $neo4j_passed/$neo4j_total passed"
    else
        print_status "- Neo4j tests: Skipped (Neo4j not available)"
    fi
    echo ""
    print_status "Check the generated report for detailed results."
}

# Run main function
main "$@"