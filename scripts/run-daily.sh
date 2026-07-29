#!/usr/bin/env bash
set -euo pipefail

export TZ=America/New_York

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# Allow override via env (used by Docker where the script lives outside the repo)
PROJECT_DIR="${PROJECT_DIR:-$(dirname "$SCRIPT_DIR")}"
STAMP_FILE="$PROJECT_DIR/.last-run"
BINARY="${BINARY:-$PROJECT_DIR/target/release/connections}"
ARCHIVE="$PROJECT_DIR/archive.json"

today="$(date +%Y-%m-%d)"
hour="$(date +%H)"

# Only run between 12am-9am Eastern Time (force decimal to avoid octal trap on 08/09)
if (( 10#$hour >= 9 )); then
  exit 0
fi

# Once-per-day gate — failed runs retry on next tick
if [[ -f "$STAMP_FILE" ]] && [[ "$(cat "$STAMP_FILE")" == "$today" ]]; then
  exit 0
fi

cd "$PROJECT_DIR"

# Abort any stuck rebase from prior run
if [[ -d ".git/rebase-merge" ]] || [[ -d ".git/rebase-apply" ]]; then
  git rebase --abort || true
  git reset --hard
fi

"$BINARY" archive --output "$ARCHIVE"

if ! git diff --quiet "$ARCHIVE"; then
  # Backup before writing
  cp "$ARCHIVE" "$ARCHIVE.backup"

  git add "$ARCHIVE"
  git commit -m "chore: archive update $today"

  # Use merge instead of rebase to avoid conflict disputes. If remote is far ahead,
  # just reset and retry next run rather than hanging
  if ! git pull --ff-only origin main 2>/dev/null; then
    git reset --hard HEAD~1
    echo "Skipped push due to non-fast-forward. Archive saved in $ARCHIVE.backup. Will retry next run."
    exit 1
  fi

  git push origin main
fi

echo "$today" > "$STAMP_FILE"
