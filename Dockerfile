# syntax=docker/dockerfile:1

# The binaries are built on the host with `just build`, which also builds the
# web UI and embeds it into `scry-api` at compile time. `just run-docker` builds
# them first; plain `docker build .` requires `target/release/scry-api` to exist.
FROM debian:bookworm-slim

# Run as a non-root user. Native-root TLS validates against the operating
# system trust store, so the image needs ca-certificates present or every
# outbound HTTPS request fails.
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates passwd \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --uid 1000 scry

# `scry-api` reads `./scry.toml` from its working directory, so compose mounts
# the config there (see compose.yaml).
WORKDIR /app

COPY target/release/scry-api /usr/local/bin/scry-api

USER scry
EXPOSE 8000

ENTRYPOINT ["scry-api"]
# Defaults to the REST API on 0.0.0.0:8000.
CMD ["--http", "0.0.0.0:8000"]
