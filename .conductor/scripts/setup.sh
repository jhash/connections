#!/usr/bin/env bash
# Setup script for connections+ worktrees
# Runs database migrations and verifies dependencies

set -e

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

echo "Setting up worktree at: $REPO_ROOT"

# Ensure migrations run on startup (cargo will handle this)
# Just verify Rust toolchain exists
if ! command -v cargo &> /dev/null; then
    echo "Error: Rust toolchain not found. Install from https://rustup.rs/"
    exit 1
fi

# Verify sqlx-cli for migrations (used in build)
if ! command -v sqlx &> /dev/null; then
    echo "Warning: sqlx-cli not found. Installing..."
    cargo install sqlx-cli --no-default-features --features sqlite
fi

echo "✓ Setup complete. Database migrations will run on first 'run' command."
