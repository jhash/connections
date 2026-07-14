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

# Only run between 12am-9am Eastern Time
if (( hour >= 9 )); then
  exit 0
fi

# Once-per-day gate — failed runs retry on next tick
if [[ -f "$STAMP_FILE" ]] && [[ "$(cat "$STAMP_FILE")" == "$today" ]]; then
  exit 0
fi

cd "$PROJECT_DIR"

"$BINARY" archive --output "$ARCHIVE"

if ! git diff --quiet "$ARCHIVE"; then
  git add "$ARCHIVE"
  git commit -m "chore: archive update $today"
  git pull --rebase origin main
  git push origin main
fi

echo "$today" > "$STAMP_FILE"
