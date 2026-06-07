# syntax=docker/dockerfile:1

# ── Build stage ──────────────────────────────────────────────────────────────
FROM rust:1-slim AS builder
WORKDIR /build
RUN apt-get update \
 && apt-get install -y --no-install-recommends pkg-config libssl-dev \
 && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY .sqlx ./.sqlx
COPY migrations ./migrations
ENV SQLX_OFFLINE=true
RUN cargo build --release

# ── Runtime stage ─────────────────────────────────────────────────────────────
FROM debian:bookworm-slim
RUN apt-get update \
 && apt-get install -y --no-install-recommends git openssh-client ca-certificates \
 && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/connections /usr/local/bin/connections
COPY scripts/run-daily.sh /scripts/run-daily.sh
COPY deploy/entrypoint.sh /entrypoint.sh
RUN chmod +x /scripts/run-daily.sh /entrypoint.sh

WORKDIR /repo
ENV PROJECT_DIR=/repo
ENV BINARY=/usr/local/bin/connections
ENTRYPOINT ["/entrypoint.sh"]
CMD ["/bin/bash", "/scripts/run-daily.sh"]
