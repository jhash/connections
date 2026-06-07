#!/usr/bin/env bash
# Production server — build release binary then run it
set -e
cd "$(dirname "$0")/.."
cargo build --release -p connections-web
exec ./target/release/connections-web
