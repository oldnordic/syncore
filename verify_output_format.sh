#!/bin/bash
# Verify ripgrep output format difference

echo "=== Format 1: Search a specific FILE ==="
rg --line-number "RipgrepSearcher" /home/feanor/Projects/SynCore/syncore/src/parser.rs | head -3

echo ""
echo "=== Format 2: Search a DIRECTORY ==="
rg --line-number "RipgrepSearcher" /home/feanor/Projects/SynCore/syncore/src/ | head -3

echo ""
echo "=== Format 3: File search with --with-filename (force file path) ==="
rg --line-number --with-filename "RipgrepSearcher" /home/feanor/Projects/SynCore/syncore/src/parser.rs | head -3
