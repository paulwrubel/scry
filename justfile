set shell := ["bash", "-uc"]

# initialize the compile-time SQLite database for sqlx query! macros
[group('validate')]
setup-db:
    sqlite3 scry.db "CREATE TABLE IF NOT EXISTS tasks (id INTEGER PRIMARY KEY AUTOINCREMENT, title TEXT NOT NULL, description TEXT, is_complete INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL, completed_at TEXT)"

# run all validation checks: test, check, clippy
[group('validate')]
validate: test check clippy

# run the test suite
[group('validate')]
test:
    cargo test

# type-check without compiling (fast feedback)
[group('validate')]
check:
    cargo check

# lint for common mistakes and style issues
[group('validate')]
clippy:
    cargo clippy -- -D warnings

# shared release logic — bump version, update Cargo.toml, commit, tag
_release type:
    @current=$(grep '^version' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/'); \
    IFS=. read major minor patch <<< "$current"; \
    case "{{type}}" in \
        patch) next="$major.$minor.$((patch + 1))" ;; \
        minor) next="$major.$((minor + 1)).0" ;; \
        major) next="$((major + 1)).0.0" ;; \
        *) echo "Invalid bump type: {{type}}"; exit 1 ;; \
    esac; \
    echo "Releasing $current → $next"; \
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
    echo ""; \
    echo "Release prepared locally. To publish, run:"; \
    echo "  git push origin main v$next"

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
