# syntax=docker/dockerfile:1

FROM rust:1.92-bookworm AS builder
WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo build --release \
    && cp /app/target/release/uj-ai-club-backend /app/uj-ai-club-backend

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl3 curl \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --system --gid 10001 app \
    && useradd --system --uid 10001 --gid app --home-dir /app --create-home app

WORKDIR /app

COPY --from=builder /app/uj-ai-club-backend /app/uj-ai-club-backend

RUN mkdir -p /app/uploads \
    && chown -R app:app /app

USER app

EXPOSE 8000

ENV RUST_LOG=info \
    SERVER_ADDRESS=0.0.0.0:8000

HEALTHCHECK --interval=30s --timeout=5s --start-period=20s --retries=3 \
    CMD curl -fsS http://127.0.0.1:8000/health || exit 1

CMD ["/app/uj-ai-club-backend"]
