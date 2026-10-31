# AGENTS.md

`scry` is a terminal task manager with a PostgreSQL backend: a clap CLI. Rust edition 2024. It is a Cargo workspace: the root `scry` binary and `crates/scry-core` (domain types and the `Store` trait). The tests are unit tests that need no database; `just cargo-test` (alias `just test`) runs them (`cargo test --workspace`).

## Tooling prerequisites

- The standard workflow uses `just` and sqlx-cli (`cargo install just sqlx-cli`). The `sqlx-prepare*` and `run-postgres` targets need sqlx-cli; the Postgres targets also need Docker, since they start and stop the Postgres defined in `compose.yaml`, which runs the app together with its own `postgres:18` service.
- `just run` (the `run-docker` alias) builds and starts the whole Docker stack, app plus database, so no host Postgres is needed; `just view-logs` tails the app container. `just run-local` builds the release binary and runs the REST server on the host against that same Postgres with `--config scry.dev.toml --secrets scry.dev-secrets.toml`, and `just run-postgres` only starts Postgres and applies migrations.
- `just pg-down` keeps the data volume and `just pg-reset` wipes it; `pg-reset` and `clean` take the whole stack down, and `clean` also runs `cargo clean`.
- rust-analyzer is configured to run `cargo clippy` as its check command (`.vscode/settings.json`), and `just cargo-clippy` (alias `just clippy`) is `cargo clippy --workspace -- -D warnings`. Keep clippy clean at `-D warnings`; CI runs it through `just validate` (`.github/workflows/ci.yml`), so `just validate` is your local gate: `cargo-test` -> `cargo-check` -> `cargo-clippy`.

## sqlx compile-time database (the #1 gotcha)

All SQL lives in `sqlx::query!`/`query_as!` macros in `src/store/postgres.rs`, and is resolved at compile time against the committed offline cache in `.sqlx/`.

- `.env` (committed) sets only `SQLX_OFFLINE=true`. sqlx reads `.env` at compile time, so ordinary `cargo build`/`check`/`test`/`clippy` resolve queries from `.sqlx/` and do **not** need a live database. A fresh clone therefore compiles without any database.
- Regenerating the cache is what needs a database. `just run-postgres` starts the dev Postgres and applies the migrations, and `just sqlx-prepare` runs `sqlx-prepare-postgres`, which brings up the dev Postgres (`pg-up`), prepares against it, and tears it down.
- After editing SQL in a `query!` macro or adding a migration, run `just sqlx-prepare`. Changing a query changes its hash, so a stale `.sqlx/*.json` breaks the build. Commit the migration and the regenerated cache together.
- `just sqlx-prepare-check` verifies the cache is current, using the dev Postgres as well; CI runs it. Release CI (`.github/workflows/release.yml`) builds with `SQLX_OFFLINE=true` against the committed cache.
- The runtime database is **not** the compile DB. The app never reads `.env`; it takes `database_url` only from `scry.toml` or `scry.secret.toml` (or the files named by `--config` / `--secrets`), with no environment-variable override and no local-file fallback — a missing URL is a startup configuration error, and only `postgres://` / `postgresql://` URLs are accepted. `cargo run` therefore talks to whichever database the resolved config points at, so point `database_url` at a scratch database when you do not want to touch real user data. The backend is built with sqlx's `tls-rustls` feature (in the root `Cargo.toml`), so URLs whose `sslmode` requires TLS connect correctly; removing that feature breaks them with a "built without TLS support" error.

## Migrations

- `migrations/` holds one fresh reversible pair, `20261031164943_init.up.sql` / `.down.sql`, which creates the sections/lifecycle schema from scratch. There is no legacy migration path and no data migration from the pre-refactor schema; a pre-refactor database must be recreated or restored from a version 2 backup. Add future migrations with `sqlx migrate add -r <name>`, filling in both files.
- Migrations are embedded (`sqlx::migrate!("./migrations")` in `src/store/postgres.rs`) and auto-run on **every app startup**. Existing user databases upgrade in place, so new migrations must be additive/backward-compatible.
- The root crate has a `build.rs` that reruns when `migrations/` changes, so new migrations trigger a rebuild automatically.

## Architecture

- `src/main.rs` — clap command definitions, one-shot handlers, and backend dispatch (`connect_store`); with no subcommand it prints the top-level help.
- `src/service.rs` — `ProjectService` owns the store and is the only write path. It is the operations layer: it resolves section names to ids (failing with a closest-match suggestion), creates sections only when explicitly asked (`--new-section` or `section add`), computes integer task positions, stamps `closed_at` on close and clears the lifecycle on open, and enforces task and section invariants before delegating raw writes to `Store`. Every mutation (CLI, REST handlers, import) goes through it; reads may use `Store` directly. The REST `ScryState` holds the service.
- `crates/scry-core/src/store.rs` — `Store` async trait declaring every DB operation. One implementor: `PostgresStore` (`src/store/postgres.rs`). All `query!` calls live there. It covers task CRUD plus `move_task` (section and position atomically), `set_lifecycle`, and `duplicate_task`; section CRUD plus `delete_section` (tasks fall back to No section) and `merge_sections` (source tasks move to the target); note and project CRUD; snapshots; and import. Add new persistence via the trait plus the implementor.
- `crates/scry-core/src/models.rs` — domain types (`Task`, `Project`, `Section`, `Lifecycle`, `CloseReason`, `Priority`, …) and ID aliases. A task has a nullable `section_id`, an integer `position`, and a `lifecycle` of `Open` or `Closed { closed_at, reason }`. `crates/scry-core/src/backup.rs` defines the JSON backup/import format; it is version 2, with no v1 upgrade support, and import rejects every other version. `src/config.rs` handles config and DB URL resolution, and `src/skill.rs` implements the embedded Agent Skill command.
- Row->model mapping is hand-rolled per query via the `*_from_fields` helpers in the store. Enums are stored as their `Display`/string names (or numeric for `Priority`) — route all conversions through these helpers rather than deserializing directly.
- `src/state.rs` — shared read/query model used by `scry list`/`scry show`: `ProjectState` groups a project snapshot into ordered section groups plus a trailing `No section` group, keeps closed tasks in a separate collection (rendered only by `--include-closed`), applies the project's sorting mode, and supports substring filtering. It has no UI dependencies.
- The Postgres pool uses sqlx defaults, so concurrent writers are possible.

## Release process

- Version bumps are scripted in the justfile: `just release-patch|minor|major` (append `-and-push` to also push). The script edits the `Cargo.toml` version, runs `cargo check`, commits `Cargo.toml` and `Cargo.lock` as `vX.Y.Z`, and tags it.
- Pushing a `v*` tag triggers `.github/workflows/release.yml`, which cross-compiles and publishes `scry-<target>.tar.gz` tarballs; `install.sh` downloads exactly those artifact names.
- Keep the README CLI reference and `install.sh` in sync when adding/changing commands or flags.
