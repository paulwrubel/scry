set shell := ["bash", "-uc"]

# rebuild the sqlite database fresh (used for the sqlite offline query cache)
[group('validate')]
setup-database:
    rm -f scry.db
    touch scry.db
    sqlx migrate run --source crates/scry-sqlite/migrations

# regenerate the sqlite offline query cache by rebuilding the DB fresh
[group('validate')]
sqlx-prepare-sqlite: setup-database
    cd crates/scry-sqlite && SQLX_OFFLINE=false cargo sqlx prepare --database-url sqlite://{{justfile_directory()}}/scry.db

# start the dev postgres and wait for it to be healthy
[group('validate')]
pg-up:
    docker compose -f compose.dev.yaml up -d --wait

# stop the dev postgres (keep its data)
[group('validate')]
pg-down:
    docker compose -f compose.dev.yaml down

# stop the dev postgres and wipe its data volume
[group('validate')]
pg-reset:
    docker compose -f compose.dev.yaml down -v

# run the postgres migrations against the local postgres
[group('validate')]
setup-database-postgres: pg-up
    sqlx migrate run --source crates/scry-postgres/migrations --database-url postgres://postgres:postgres@127.0.0.1:5432/scry

# regenerate the postgres offline query cache against the local postgres
[group('validate')]
sqlx-prepare-postgres: setup-database-postgres
    #!/usr/bin/env bash
    set -euo pipefail
    trap 'cd "{{justfile_directory()}}" && just pg-down || true' EXIT
    cd crates/scry-postgres && SQLX_OFFLINE=false cargo sqlx prepare --database-url postgres://postgres:postgres@127.0.0.1:5432/scry

# regenerate both offline query caches
[group('validate')]
sqlx-prepare: sqlx-prepare-sqlite sqlx-prepare-postgres

# run all validation checks: test, check, clippy
[group('validate')]
validate:  test check clippy

# run the test suite
[group('validate')]
test: setup-database
    cargo test --workspace

# type-check without compiling (fast feedback)
[group('validate')]
check: setup-database
    cargo check --workspace

# lint for common mistakes and style issues
[group('validate')]
clippy: setup-database
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
    rm -f ./scry.db
