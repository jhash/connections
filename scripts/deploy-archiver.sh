#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

echo "Building aarch64 binary via Docker..."
mkdir -p "$PROJECT_DIR/bin/arm64"
docker run --rm \
  -v "$PROJECT_DIR:/workspace" \
  -w /workspace \
  rust:latest \
  bash -c 'rustup target add aarch64-unknown-linux-gnu && cargo build --release -p connections-cli --target aarch64-unknown-linux-gnu'

echo "Copying to bin/arm64..."
cp "$PROJECT_DIR/target/aarch64-unknown-linux-gnu/release/connections" "$PROJECT_DIR/bin/arm64/connections"

echo "Building and pushing Docker image..."
docker buildx build \
  -f Dockerfile.archiver \
  --platform linux/arm64 \
  -t jhash14/connections-archiver:latest \
  --push .

echo "Deploying to OCI box..."
OCI_IP=$(oci-ip)
ssh deploy@"$OCI_IP" 'docker pull jhash14/connections-archiver:latest && docker service update --image jhash14/connections-archiver:latest connections_archiver'

echo "Done."
