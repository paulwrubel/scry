# AGENTS.md

`scry` is a terminal task manager with a SQLite or PostgreSQL backend: a clap CLI, a ratatui interactive TUI (launched when run with **no subcommand**), and an in-process MCP server (`scry mcp`). Rust edition 2024. It is a Cargo workspace: the root `scry` binary plus `crates/scry-core` (domain types and the `Store` trait), `crates/scry-sqlite` (`SqliteStore`), and `crates/scry-postgres` (`PostgresStore`). Integration tests live in `tests/cli.rs` and drive the built binary against isolated scratch databases; `just test` runs them (`cargo test --workspace`).

## Tooling prerequisites

- The standard workflow uses `just` and sqlx-cli (`cargo install just sqlx-cli`). The `setup-database*` and `sqlx-prepare*` targets need sqlx-cli; the Postgres targets also need Docker, since they start and stop the dev Postgres defined in `compose.dev.yaml`.
- rust-analyzer is configured to run `cargo clippy` as its check command (`.vscode/settings.json`), and `just clippy` is `cargo clippy --workspace -- -D warnings`. Keep clippy clean at `-D warnings`; CI runs it through `just validate` (`.github/workflows/ci.yml`), so `just validate` is your local gate: `test` -> `check` -> `clippy`.

## sqlx compile-time database (the #1 gotcha)

All SQL lives in `sqlx::query!`/`query_as!` macros in `crates/scry-sqlite/src/sqlite.rs` and `crates/scry-postgres/src/postgres.rs`, and is resolved at compile time against the committed offline cache in each crate's `.sqlx/`.

- `.env` (committed) sets `DATABASE_URL=sqlite://scry.db` and `SQLX_OFFLINE=true`. sqlx reads `.env` at compile time, so ordinary `cargo build`/`check`/`test`/`clippy` resolve queries from `.sqlx/` and do **not** need a live database. A fresh clone therefore compiles without any database.
- Regenerating the caches is what needs a database. `just setup-database` deletes and rebuilds the repo-root `scry.db` from the sqlite migrations. `just sqlx-prepare` regenerates both caches: `sqlx-prepare-sqlite` prepares the SQLite cache against `scry.db` (with `SQLX_OFFLINE=false`), and `sqlx-prepare-postgres` brings up the dev Postgres (`pg-up`), prepares against it, and tears it down.
- After editing SQL in a `query!` macro or adding a migration, run `just sqlx-prepare` (or the backend-specific target). Changing a query changes its hash, so a stale `.sqlx/*.json` breaks the build. Commit the migration and the regenerated cache together.
- `just sqlx-prepare-check` verifies both caches are current; CI runs it. Release CI (`.github/workflows/release.yml`) builds with `SQLX_OFFLINE=true` against the committed cache.
- `test`, `check`, and `clippy` each depend on `setup-database` first, so `just validate` always rebuilds `scry.db`, wiping repo-root dev data by design.
- The runtime database is **not** the compile DB. The app never reads `.env`; it selects a backend from the `DATABASE_URL` env var at startup (`sqlite:` -> `SqliteStore`, `postgres://` / `postgresql://` -> `PostgresStore`, anything else is an error), falling back to `$XDG_DATA_HOME/scry/scry.db` (`~/.local/share/scry/scry.db`) when unset. `cargo run` therefore touches your real user data unless you point it elsewhere, e.g. `DATABASE_URL=sqlite:///tmp/scratch.db cargo run`. The Postgres backend is built with sqlx's `tls-rustls` feature (`crates/scry-postgres/Cargo.toml`), so URLs whose `sslmode` requires TLS connect correctly; removing that feature breaks them with a "built without TLS support" error.

## Migrations

- Each backend owns its migrations: `crates/scry-sqlite/migrations/` and `crates/scry-postgres/migrations/`. They are reversible sqlx pairs (`<ts>_<name>.up.sql` / `.down.sql`) matching `sqlx migrate add -r <name>` output. Add new migrations that way, filling in both files, in the backend(s) they apply to.
- Migrations are embedded (`sqlx::migrate!("./migrations")` in each crate's store) and auto-run on **every app startup** for the selected backend. Existing user databases upgrade in place, so new migrations must be additive/backward-compatible.
- Each store crate has a `build.rs` that reruns when that crate's `migrations/` changes, so new migrations trigger a rebuild automatically.

## Architecture

- `src/main.rs` — clap command definitions, one-shot handlers, and backend dispatch (`connect_store`); no subcommand launches the TUI.
- `src/mcp.rs` plus `src/mcp/` — in-process MCP server exposing task/project/status/note operations as tools, over stdio by default or streamable HTTP with `scry mcp --http <addr>`.
- `crates/scry-core/src/store.rs` — `Store` async trait declaring every DB operation. Two implementors: `SqliteStore` (`crates/scry-sqlite/src/sqlite.rs`) and `PostgresStore` (`crates/scry-postgres/src/postgres.rs`). All `query!` calls live in those two files. Add new persistence via the trait plus each implementor.
- `crates/scry-core/src/models.rs` — domain types (`Task`, `Project`, `Status`, `Priority`, …) and ID aliases. `crates/scry-core/src/backup.rs` defines the JSON backup/import format. `src/service.rs` holds business logic (`ProjectService`), `src/config.rs` handles config and DB URL resolution, and `src/skill.rs` implements the embedded Agent Skill command.
- Row->model mapping is hand-rolled per query via the `*_from_fields` helpers in each store. SQLite stores timestamps as RFC3339 TEXT; enums are stored as their `Display`/string names (or numeric for `Priority`) — route all conversions through these helpers rather than deserializing directly.
- `src/tui/` — command-driven ratatui app: `app.rs` (event loop), `command.rs` (internal command enum), `action.rs` (input actions), `component/` (panes; `root.rs` dispatches; `popup/` are overlays; `shared/` are reusable widgets). Shared app state lives in `src/state.rs`.
- The SQLite pool is `max_connections(1)` with `foreign_keys(true)` — no concurrent SQLite writers. The Postgres pool uses sqlx defaults, so concurrent writers are possible there.

## Release process

- Version bumps are scripted in the justfile: `just release-patch|minor|major` (append `-and-push` to also push). The script edits the `Cargo.toml` version, runs `cargo check`, commits `Cargo.toml` and `Cargo.lock` as `vX.Y.Z`, and tags it.
- Pushing a `v*` tag triggers `.github/workflows/release.yml`, which cross-compiles and publishes `scry-<target>.tar.gz` tarballs; `install.sh` downloads exactly those artifact names.
- Keep the README CLI reference and `install.sh` in sync when adding/changing commands or flags.
