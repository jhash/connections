#!/usr/bin/env bash
# Run script for connections+ worktrees
# Auto-detects available port and starts dev server
# Shares DATABASE_URL across all instances

set -e

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

# Find available port starting from 3062
find_available_port() {
    local port=3062
    local max_attempts=100
    local attempt=0

    while [ $attempt -lt $max_attempts ]; do
        # Check if port is available (cross-platform compatible)
        if ! nc -z localhost "$port" 2>/dev/null; then
            echo "$port"
            return 0
        fi
        port=$((port + 1))
        attempt=$((attempt + 1))
    done

    echo "Error: Could not find available port after $max_attempts attempts" >&2
    return 1
}

# Find the main repo checkout (shared across all Conductor worktrees)
get_main_repo_root() {
    if [ -n "$CONDUCTOR_ROOT_PATH" ]; then
        echo "$CONDUCTOR_ROOT_PATH"
    else
        # Fallback for non-Conductor checkouts: common git dir's parent is the main worktree
        (cd "$(dirname "$(git rev-parse --git-common-dir)")" && pwd)
    fi
}

# Get or create shared database URL
get_database_url() {
    if [ -n "$DATABASE_URL" ]; then
        echo "$DATABASE_URL"
    else
        # Default to main repo's database, shared across all worktrees
        echo "sqlite://$(get_main_repo_root)/games.db?mode=rwc"
    fi
}

PORT=$(find_available_port)
DB_URL=$(get_database_url)
ARCHIVE_PATH="${ARCHIVE_PATH:-$(get_main_repo_root)/archive.json}"

export DATABASE_URL="$DB_URL"
export ARCHIVE_PATH="$ARCHIVE_PATH"

echo "Starting dev server on port $PORT"
echo "Database: $DB_URL"
echo "Archive: $ARCHIVE_PATH"

# Hot-reload dev server — socket persists across recompiles
exec systemfd --no-pid -s "http::$PORT" -- cargo watch -x 'run -p connections-web'
