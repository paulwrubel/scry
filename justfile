set shell := ["bash", "-uc"]

default:
    @just --list

alias run := run-local

# start the dev postgres and wait for it to be healthy
[group('run')]
pg-up:
    docker compose -f compose.dev.yaml up -d --wait

# stop the dev postgres (keep its data)
[group('run')]
pg-down:
    docker compose -f compose.dev.yaml down

# stop the dev postgres and wipe its data volume
[group('run')]
pg-reset:
    docker compose -f compose.dev.yaml down -v

# start the dev postgres and apply migrations
[group('run')]
run-postgres: pg-up
    sqlx migrate run --source crates/scry-postgres/migrations --database-url postgres://postgres:postgres@127.0.0.1:5432/scry

# run the REST server locally against the dev postgres
[group('run')]
run-local: run-postgres
    cargo run -- --config scry.dev.toml --secrets scry.dev-secrets.toml serve --http 127.0.0.1:8000

# start the MCP server stack in Docker against the host dev postgres
[group('run')]
run-docker: run-postgres
    docker compose up -d --build

# regenerate the postgres offline query cache against the local postgres
[group('validate')]
sqlx-prepare-postgres: run-postgres
    #!/usr/bin/env bash
    set -euo pipefail
    trap 'cd "{{justfile_directory()}}" && just pg-down || true' EXIT
    cd crates/scry-postgres && SQLX_OFFLINE=false cargo sqlx prepare --database-url postgres://postgres:postgres@127.0.0.1:5432/scry

# verify the Postgres offline query cache is up to date (used in CI)
[group('validate')]
sqlx-prepare-check: run-postgres
    #!/usr/bin/env bash
    set -euo pipefail
    trap 'cd "{{justfile_directory()}}" && just pg-down || true' EXIT
    cd crates/scry-postgres && SQLX_OFFLINE=false cargo sqlx prepare --check --database-url postgres://postgres:postgres@127.0.0.1:5432/scry

# run all validation checks: test, check, clippy
[group('validate')]
validate: cargo-test cargo-check cargo-clippy

# run the test suite
alias test := cargo-test
[group('validate')]
cargo-test:
    cargo test --workspace

# type-check without compiling (fast feedback)
alias check := cargo-check
[group('validate')]
cargo-check:
    cargo check --workspace

# lint for common mistakes and style issues
alias clippy := cargo-clippy
[group('validate')]
cargo-clippy:
    cargo clippy --workspace -- -D warnings

# bump patch version (x.y.Z → x.y.Z+1)
[group('release')]
release-patch:
    @just _release patch

# bump minor version (x.Y.z → x.Y+1.0)
[group('release')]
release-minor:
    @just _release minor

# bump major version (X.y.z → X+1.0.0)
[group('release')]
release-major:
    @just _release major

# bump patch version and push to origin
[group('release')]
release-patch-and-push:
    @just _release patch push

# bump minor version and push to origin
[group('release')]
release-minor-and-push:
    @just _release minor push

# bump major version and push to origin
[group('release')]
release-major-and-push:
    @just _release major push

# shared release logic — bump version, update Cargo.toml, commit, tag
_release type do_push="":
    @current=$(grep '^version' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/'); \
    IFS=. read major minor patch <<< "$current"; \
    case "{{type}}" in \
        patch) next="$major.$minor.$((patch + 1))" ;; \
        minor) next="$major.$((minor + 1)).0" ;; \
        major) next="$((major + 1)).0.0" ;; \
        *) echo "Invalid bump type: {{type}}"; exit 1 ;; \
    esac; \
    echo "Releasing $current --> $next"; \
    read -p "Proceed? [y/N] " confirm; \
    case "$confirm" in \
        [yY]*) ;; \
        *) echo "Aborted."; exit 1 ;; \
    esac; \
    sed -i.bak "s/^version = \"$current\"/version = \"$next\"/" Cargo.toml && rm -f Cargo.toml.bak; \
    cargo check; \
    git add Cargo.toml Cargo.lock; \
    git commit -m "v$next"; \
    git tag "v$next"; \
    if [ "{{do_push}}" = "push" ]; then \
        git push origin main "v$next"; \
    else \
        echo ""; \
        echo "Release prepared locally. To publish, run:"; \
        echo "  git push origin main v$next"; \
    fi

# regenerate the Postgres offline query cache
[group('build')]
sqlx-prepare: sqlx-prepare-postgres

# build an optimized release binary
[group('build')]
build:
    cargo build --release

# build and install to ~/.local/bin
[group('build')]
install:
    cargo build --release
    mkdir -p {{"${HOME}"}}/.local/bin
    cp target/release/scry {{"${HOME}"}}/.local/bin/scry

# wipe build artifacts
[group('misc')]
clean:
    cargo clean
