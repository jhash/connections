#!/usr/bin/env bash
# Hot-reload dev server — socket persists across recompiles
set -e
cd "$(dirname "$0")/.."
exec systemfd --no-pid -s http::3062 -- cargo watch -x 'run -p connections-web'
