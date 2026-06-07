#!/usr/bin/env bash
# Regenerate .sqlx/ query cache after adding or changing sqlx::query! macros.
# Must be re-run and cache committed whenever queries change.
set -e
cd "$(dirname "$0")/.."

DB_PATH="games.db"

DATABASE_URL="sqlite://${DB_PATH}?mode=rwc" cargo sqlx database create
DATABASE_URL="sqlite://${DB_PATH}?mode=rwc" cargo sqlx migrate run
DATABASE_URL="sqlite://${DB_PATH}?mode=rwc" cargo sqlx prepare --workspace

echo "Done. Commit .sqlx/ directory."
