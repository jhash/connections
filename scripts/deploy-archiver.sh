#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

echo "Installing cross if needed..."
if ! command -v cross &> /dev/null; then
  cargo install cross --locked
fi

echo "Adding aarch64 target..."
rustup target add aarch64-unknown-linux-gnu

echo "Building aarch64 binary..."
cd "$PROJECT_DIR"
cross build --release -p connections-cli --target aarch64-unknown-linux-gnu

echo "Copying to bin/arm64..."
mkdir -p bin/arm64
cp target/aarch64-unknown-linux-gnu/release/connections bin/arm64/connections

echo "Building and pushing Docker image..."
docker buildx build \
  -f Dockerfile.archiver \
  --platform linux/arm64 \
  -t jhash14/connections-archiver:latest \
  --push .

echo "Done. Deploy on OCI box:"
echo "ssh deploy@<oci-ip> 'docker pull jhash14/connections-archiver:latest && docker service update --image jhash14/connections-archiver:latest connections_archiver'"
