set shell := ["bash", "-uc"]

default:
    @just --list

alias run := run-docker

# build and start the app and database stack in Docker
[group('run')]
run-docker: build
    docker compose up -d --build

# build the release binary and run the REST server against the dev postgres
[group('run')]
run-local: build run-postgres
    ./target/release/scry --config scry.dev.toml --secrets scry.dev-secrets.toml serve --http 127.0.0.1:8000

# start the dev postgres and apply migrations
[group('run')]
run-postgres: pg-up
    sqlx migrate run --source migrations --database-url postgres://postgres:postgres@127.0.0.1:5432/scry

# start only the dev postgres and wait for it to be healthy
[group('run')]
pg-up:
    docker compose up -d --wait postgres

# stop the dev postgres (keep its data)
[group('run')]
pg-down:
    docker compose stop postgres

# stop the whole stack and wipe the postgres data volume
[group('run')]
pg-reset:
    docker compose down -v

# tail the app container logs
[group('run')]
view-logs:
    docker compose logs -f scry

# run the ui dev server against the docker stack
[group('run')]
run-ui: run-docker setup-ui-env
    cd ui && . "$HOME/.nvm/nvm.sh" && nvm use && npm run dev

# regenerate the Postgres offline query cache
[group('build')]
sqlx-prepare: sqlx-prepare-postgres

# build an optimized release binary
[group('build')]
build: build-ui
    cargo build --release

# build and install to ~/.local/bin
[group('build')]
install: build
    mkdir -p {{"${HOME}"}}/.local/bin
    cp target/release/scry {{"${HOME}"}}/.local/bin/scry

# build the ui
[group('build')]
build-ui: setup-ui-env
    cd ui && . "$HOME/.nvm/nvm.sh" && nvm use && npm run build

# run all validation checks: Rust, generated output, fixtures, and the UI suite
[group('validate')]
validate: build-ui cargo-test cargo-check cargo-clippy check-bindings check-fixtures
    cd ui && . "$HOME/.nvm/nvm.sh" && nvm use \
    && npm run lint \
    && npm run prettier-check \
    && npm run type-check \
    && npm test

# run the test suite
alias test := cargo-test
[group('validate')]
cargo-test:
    cargo test --workspace

# run the test suite without default features (CI)
[group('validate')]
cargo-test-ci:
    cargo test --workspace --no-default-features

# type-check without compiling (fast feedback)
alias check := cargo-check
[group('validate')]
cargo-check:
    cargo check --workspace

# type-check without default features (CI)
[group('validate')]
cargo-check-ci:
    cargo check --workspace --no-default-features

# lint for common mistakes and style issues
alias clippy := cargo-clippy
[group('validate')]
cargo-clippy:
    cargo clippy --workspace -- -D warnings

# lint without default features (CI)
[group('validate')]
cargo-clippy-ci:
    cargo clippy --workspace --no-default-features -- -D warnings

# lint the ui
[group('validate')]
lint-ui: setup-ui-env
    cd ui && . "$HOME/.nvm/nvm.sh" && nvm use && npm run lint

# check ui formatting
[group('validate')]
prettier-check-ui: setup-ui-env
    cd ui && . "$HOME/.nvm/nvm.sh" && nvm use && npm run prettier-check

# type-check the ui
[group('validate')]
typecheck-ui: setup-ui-env
    cd ui && . "$HOME/.nvm/nvm.sh" && nvm use && npm run type-check

# run the ui test suite
[group('validate')]
test-ui: setup-ui-env
    cd ui && . "$HOME/.nvm/nvm.sh" && nvm use && npm test

# regenerate the postgres offline query cache against the local postgres
[group('validate')]
sqlx-prepare-postgres: run-postgres
    #!/usr/bin/env bash
    set -euo pipefail
    trap 'cd "{{justfile_directory()}}" && just pg-down || true' EXIT
    SQLX_OFFLINE=false cargo sqlx prepare --database-url postgres://postgres:postgres@127.0.0.1:5432/scry -- --no-default-features

# verify the Postgres offline query cache is up to date (used in CI)
[group('validate')]
sqlx-prepare-check: run-postgres
    #!/usr/bin/env bash
    set -euo pipefail
    trap 'cd "{{justfile_directory()}}" && just pg-down || true' EXIT
    SQLX_OFFLINE=false cargo sqlx prepare --check --database-url postgres://postgres:postgres@127.0.0.1:5432/scry -- --no-default-features

# regenerate the TypeScript bindings for the sync wire types
[group('validate')]
generate-bindings:
    cargo test -p scry --no-default-features --features ts export_bindings

# verify the generated TypeScript bindings are committed and up to date
[group('validate')]
check-bindings: generate-bindings
    #!/usr/bin/env bash
    set -euo pipefail
    if [[ -n "$(git status --porcelain -- ui/src/generated)" ]]; then
        echo "TypeScript bindings are out of date; run 'just generate-bindings' and commit the result" >&2
        git status --porcelain -- ui/src/generated >&2
        exit 1
    fi

# regenerate the shared cross-language fixtures
[group('validate')]
generate-fixtures:
    cargo run -p scry-core --example generate_fixtures

# verify the shared cross-language fixtures are committed and up to date
[group('validate')]
check-fixtures: generate-fixtures
    #!/usr/bin/env bash
    set -euo pipefail
    if [[ -n "$(git status --porcelain -- fixtures)" ]]; then
        echo "fixtures are out of date; run 'just generate-fixtures' and commit the result" >&2
        git status --porcelain -- fixtures >&2
        exit 1
    fi

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
    cargo check --no-default-features; \
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

# install the ui dependencies
[group('misc')]
setup-ui-env:
    cd ui && . "$HOME/.nvm/nvm.sh" && nvm install && nvm use && npm install

# format the ui
[group('misc')]
prettier-fix-ui:
    cd ui && . "$HOME/.nvm/nvm.sh" && nvm use && npm run prettier-fix

# wipe build artifacts and stop the Docker stack
[group('misc')]
clean:
    cargo clean
    -rm -r ./ui/dist
    docker compose down
