#!/usr/bin/env bash
# Archive script for connections+ worktrees
# Placeholder for archive operations

set -e

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

echo "Archive check at: $REPO_ROOT"

# Verify archive.json exists and is readable
if [ ! -f "$REPO_ROOT/archive.json" ]; then
    echo "Warning: archive.json not found at $REPO_ROOT"
    exit 1
fi

echo "✓ Archive verified: $(wc -c < "$REPO_ROOT/archive.json") bytes"
