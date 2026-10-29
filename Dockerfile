# syntax=docker/dockerfile:1

# Build the release binary. The Postgres TLS stack still compiles C, so the
# builder needs a C toolchain and the runtime needs nothing beyond glibc.
ARG RUST_VERSION=1.95
FROM rust:${RUST_VERSION}-slim-bookworm AS builder

RUN apt-get update \
    && apt-get install -y --no-install-recommends build-essential pkg-config \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# The committed Postgres .sqlx offline cache lets the sqlx macros compile
# without a live database.
ENV SQLX_OFFLINE=true

COPY . .
RUN cargo build --release

FROM debian:bookworm-slim AS runtime

# Run as a non-root user. Native-root TLS validates against the operating
# system trust store, so the image needs ca-certificates present or every
# outbound HTTPS request fails.
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates passwd \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --uid 1000 scry

# `scry` reads `./scry.toml` from its working directory, so compose mounts the
# config there (see compose.yaml).
WORKDIR /app

COPY --from=builder /app/target/release/scry /usr/local/bin/scry

USER scry
EXPOSE 8000

ENTRYPOINT ["scry"]
# Defaults to the REST API on 0.0.0.0:8000.
CMD ["serve", "--http", "0.0.0.0:8000"]
