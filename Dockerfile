# syntax=docker/dockerfile:1

# ── Build stage ──────────────────────────────────────────────────────────────
FROM rust:1-slim AS builder
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

# ── Runtime stage ─────────────────────────────────────────────────────────────
FROM debian:bookworm-slim
RUN apt-get update \
 && apt-get install -y --no-install-recommends git openssh-client ca-certificates \
 && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/connections /usr/local/bin/connections
COPY scripts/run-daily.sh /scripts/run-daily.sh
RUN chmod +x /scripts/run-daily.sh

# Mount the cloned repo at /repo (includes archive.json, .git, .last-run).
# Mount SSH keys at /root/.ssh (read-only).
#
# Example daily docker run (cron or cloud scheduler):
#   docker run --rm \
#     -v /path/to/repo:/repo \
#     -v ~/.ssh:/root/.ssh:ro \
#     -e PROJECT_DIR=/repo \
#     connections-archive
#
WORKDIR /repo
ENV PROJECT_DIR=/repo
ENTRYPOINT ["/bin/bash", "/scripts/run-daily.sh"]
