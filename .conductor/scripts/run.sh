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

# Get or create shared database URL
get_database_url() {
    if [ -n "$DATABASE_URL" ]; then
        echo "$DATABASE_URL"
    else
        # Default to workspace root database
        echo "sqlite://$REPO_ROOT/games.db?mode=rwc"
    fi
}

PORT=$(find_available_port)
DB_URL=$(get_database_url)
ARCHIVE_PATH="${ARCHIVE_PATH:-$REPO_ROOT/archive.json}"

export DATABASE_URL="$DB_URL"
export ARCHIVE_PATH="$ARCHIVE_PATH"

echo "Starting dev server on port $PORT"
echo "Database: $DB_URL"
echo "Archive: $ARCHIVE_PATH"

# Hot-reload dev server — socket persists across recompiles
exec systemfd --no-pid -s "http::$PORT" -- cargo watch -x 'run -p connections-web'
