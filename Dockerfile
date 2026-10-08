# syntax=docker/dockerfile:1

# Build the release binary. SQLite is compiled into the binary (sqlx's
# `sqlite-bundled` feature), so the builder needs a C toolchain and the runtime
# needs nothing beyond glibc.
ARG RUST_VERSION=1.95
FROM rust:${RUST_VERSION}-slim-bookworm AS builder

RUN apt-get update \
    && apt-get install -y --no-install-recommends build-essential pkg-config \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# The committed .sqlx offline cache lets the sqlx macros compile without a
# live database.
ENV SQLX_OFFLINE=true

COPY . .
RUN cargo build --release

FROM debian:bookworm-slim AS runtime

# Run as a non-root user with a writable data directory. For a bind-mounted
# database whose owner differs, run the container with
# `--user "$(id -u):$(id -g)"`.
RUN apt-get update \
    && apt-get install -y --no-install-recommends passwd \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --uid 1000 scry \
    && mkdir -p /data \
    && chown scry:scry /data

# `scry` reads `./scry.toml` from its working directory, so compose mounts the
# config there (see compose.yaml).
WORKDIR /app

COPY --from=builder /app/target/release/scry /usr/local/bin/scry

USER scry
ENV DATABASE_URL=sqlite:///data/scry.db
VOLUME ["/data"]

ENTRYPOINT ["scry"]
# Defaults to the stdio transport; override for HTTP, e.g.
#   command: ["mcp", "--http", "0.0.0.0:8000"]
CMD ["mcp"]
