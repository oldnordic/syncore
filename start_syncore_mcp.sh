#!/bin/bash
cd "/home/feanor/Projects/SynCore/syncore"
export RUST_LOG=info
export DB_PATH=syncore.db
exec cargo run --bin syncore_mcp_stdio
