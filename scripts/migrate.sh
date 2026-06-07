#!/usr/bin/env bash
# Run pending sqlx migrations against the local dev database.
set -e
cd "$(dirname "$0")/.."

DB_PATH="games.db"

DATABASE_URL="sqlite://${DB_PATH}?mode=rwc" cargo sqlx database create
DATABASE_URL="sqlite://${DB_PATH}?mode=rwc" cargo sqlx migrate run

echo "Migrations applied."
