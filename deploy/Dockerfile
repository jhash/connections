FROM rust:slim AS chef
WORKDIR /build
RUN cargo install cargo-chef --locked

FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /build/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json -p connections-web
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/
COPY migrations/ ./migrations/
COPY .sqlx/ ./.sqlx/
ENV SQLX_OFFLINE=true
RUN cargo build --release -p connections-web

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/connections-web /usr/local/bin/connections-web
COPY archive.json /opt/connections/archive.json
ENV ARCHIVE_PATH=/opt/connections/archive.json
ENV DATABASE_URL=sqlite:///data/games.db?mode=rwc
EXPOSE 3062
ENTRYPOINT ["/usr/local/bin/connections-web"]
