# AGENTS.md

`scry` is a terminal task manager with a PostgreSQL backend: a clap CLI and an in-process MCP server (`scry mcp`). Rust edition 2024. It is a Cargo workspace: the root `scry` binary plus `crates/scry-core` (domain types and the `Store` trait) and `crates/scry-postgres` (`PostgresStore`). The tests are unit tests that need no database; `just test` runs them (`cargo test --workspace`).

## Tooling prerequisites

- The standard workflow uses `just` and sqlx-cli (`cargo install just sqlx-cli`). The `sqlx-prepare*` and `setup-database*` targets need sqlx-cli; the Postgres targets also need Docker, since they start and stop the dev Postgres defined in `compose.dev.yaml`.
- rust-analyzer is configured to run `cargo clippy` as its check command (`.vscode/settings.json`), and `just clippy` is `cargo clippy --workspace -- -D warnings`. Keep clippy clean at `-D warnings`; CI runs it through `just validate` (`.github/workflows/ci.yml`), so `just validate` is your local gate: `test` -> `check` -> `clippy`.

## sqlx compile-time database (the #1 gotcha)

All SQL lives in `sqlx::query!`/`query_as!` macros in `crates/scry-postgres/src/postgres.rs`, and is resolved at compile time against the committed offline cache in `crates/scry-postgres/.sqlx/`.

- `.env` (committed) sets only `SQLX_OFFLINE=true`. sqlx reads `.env` at compile time, so ordinary `cargo build`/`check`/`test`/`clippy` resolve queries from `.sqlx/` and do **not** need a live database. A fresh clone therefore compiles without any database.
- Regenerating the cache is what needs a database. `just setup-database-postgres` applies the migrations to the dev Postgres, and `just sqlx-prepare` runs `sqlx-prepare-postgres`, which brings up the dev Postgres (`pg-up`), prepares against it, and tears it down.
- After editing SQL in a `query!` macro or adding a migration, run `just sqlx-prepare`. Changing a query changes its hash, so a stale `.sqlx/*.json` breaks the build. Commit the migration and the regenerated cache together.
- `just sqlx-prepare-check` verifies the cache is current, using the dev Postgres as well; CI runs it. Release CI (`.github/workflows/release.yml`) builds with `SQLX_OFFLINE=true` against the committed cache.
- The runtime database is **not** the compile DB. The app never reads `.env`; it takes `database_url` only from `scry.toml` or `scry.secret.toml`, with no environment-variable override and no local-file fallback — a missing URL is a startup configuration error, and only `postgres://` / `postgresql://` URLs are accepted. `cargo run` therefore talks to whichever database the resolved config points at, so point `database_url` at a scratch database when you do not want to touch real user data. The backend is built with sqlx's `tls-rustls` feature (`crates/scry-postgres/Cargo.toml`), so URLs whose `sslmode` requires TLS connect correctly; removing that feature breaks them with a "built without TLS support" error.

## Migrations

- `crates/scry-postgres/migrations/` holds the migrations. They are reversible sqlx pairs (`<ts>_<name>.up.sql` / `.down.sql`) matching `sqlx migrate add -r <name>` output. Add new migrations that way, filling in both files.
- Migrations are embedded (`sqlx::migrate!("./migrations")` in the store) and auto-run on **every app startup**. Existing user databases upgrade in place, so new migrations must be additive/backward-compatible.
- The crate has a `build.rs` that reruns when its `migrations/` changes, so new migrations trigger a rebuild automatically.

## Architecture

- `src/main.rs` — clap command definitions, one-shot handlers, and backend dispatch (`connect_store`); with no subcommand it prints the top-level help.
- `src/mcp.rs` plus `src/mcp/` — in-process MCP server exposing task/project/status/note operations as tools, over stdio by default or streamable HTTP with `scry mcp --http <addr>`.
- `crates/scry-core/src/store.rs` — `Store` async trait declaring every DB operation. One implementor: `PostgresStore` (`crates/scry-postgres/src/postgres.rs`). All `query!` calls live there. Add new persistence via the trait plus the implementor.
- `crates/scry-core/src/models.rs` — domain types (`Task`, `Project`, `Status`, `Priority`, …) and ID aliases. `crates/scry-core/src/backup.rs` defines the JSON backup/import format. `src/service.rs` holds business logic (`ProjectService`), `src/config.rs` handles config and DB URL resolution, and `src/skill.rs` implements the embedded Agent Skill command.
- Row->model mapping is hand-rolled per query via the `*_from_fields` helpers in the store. Enums are stored as their `Display`/string names (or numeric for `Priority`) — route all conversions through these helpers rather than deserializing directly.
- `src/state.rs` — shared read/query model used by `scry list`/`scry show` and MCP `task_list`/`task_show`: `ProjectState` groups a project snapshot into statuses with tasks and notes, applies the project's sorting mode, and supports substring filtering. It has no UI dependencies.
- The Postgres pool uses sqlx defaults, so concurrent writers are possible.

## Release process

- Version bumps are scripted in the justfile: `just release-patch|minor|major` (append `-and-push` to also push). The script edits the `Cargo.toml` version, runs `cargo check`, commits `Cargo.toml` and `Cargo.lock` as `vX.Y.Z`, and tags it.
- Pushing a `v*` tag triggers `.github/workflows/release.yml`, which cross-compiles and publishes `scry-<target>.tar.gz` tarballs; `install.sh` downloads exactly those artifact names.
- Keep the README CLI reference and `install.sh` in sync when adding/changing commands or flags.
