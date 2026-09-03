# AGENTS.md

`scry` is a SQLite-backed terminal task manager: a clap CLI plus a ratatui interactive TUI (launched when run with **no subcommand**). Single binary crate, Rust edition 2024. Currently there are no unit/integration tests — `just test`/`cargo test` only verifies compilation.

## Tooling prerequisites

- The standard workflow uses `just` and sqlx-cli (`cargo install just sqlx-cli`). `just sqlx-prepare`, `sqlx migrate run`, and `cargo sqlx prepare` all need sqlx-cli.
- rust-analyzer is configured to run `cargo clippy` as its check command (`.vscode/settings.json`), and `just clippy` is `cargo clippy -- -D warnings`. Keep clippy clean at `-D warnings`; CI does not run it, so `just validate` is your gate: `test` -> `check` -> `clippy`.

## sqlx compile-time database (the #1 gotcha)

All SQL lives in `sqlx::query!`/`query_as!` macros in `src/store/sqlite.rs` and is checked against a live database at compile time.

- `.env` (committed) sets `DATABASE_URL=sqlite://scry.db`, i.e. the repo-root `scry.db`. That file is gitignored build-time state, not a committed artifact — a fresh clone will **not compile** until it exists.
- Run `just setup-database` after a fresh clone (creates an empty DB and applies migrations), or run any `just` target — `test`, `check`, and `clippy` all depend on `setup-database` first.
- `setup-database` **deletes** `scry.db` and rebuilds it from migrations. Repo-root dev data is wiped by design every time you validate.
- Whenever you edit SQL inside a `query!` macro or add a migration, you must regenerate the committed offline cache: `just sqlx-prepare`. CI (`.github/workflows/release.yml`) builds with `SQLX_OFFLINE=true` against that cache, so stale `.sqlx/*.json` breaks the release build. Commit both the migration and the regenerated cache.
- The runtime database is **not** the compile DB. The app never reads `.env`; it reads the `DATABASE_URL` env var at startup and otherwise falls back to `$XDG_DATA_HOME/scry/scry.db` (`~/.local/share/scry/scry.db`). `cargo run` therefore touches your real user data unless you point it elsewhere, e.g. `DATABASE_URL=sqlite:///tmp/scratch.db cargo run`.

## Migrations

- `migrations/` holds reversible sqlx pairs (`<ts>_<name>.up.sql` / `.down.sql`), matching `sqlx migrate add -r <name>` output. Add new migrations that way, filling in both files.
- Migrations are embedded (`sqlx::migrate!("./migrations")` in `src/store/sqlite.rs`) and auto-run on **every app startup**. Existing user databases upgrade in place, so new migrations must be additive/backward-compatible.
- `build.rs` reruns when `migrations/` changes, so new migrations trigger a rebuild automatically.

## Architecture

- `src/main.rs` — clap command definitions and one-shot handlers; no subcommand means launch the TUI.
- `src/store.rs` — `TaskStore` async trait declaring every DB operation. `SqliteStore` in `src/store/sqlite.rs` is the only impl, and all `query!` calls live there. Add new persistence via trait + impl.
- `src/models.rs` — domain types (`Task`, `Project`, `Status`, `Priority`, …) and ID aliases.
- Row->model mapping is hand-rolled per query via the `*_from_fields` helpers in `sqlite.rs`. Timestamps are RFC3339 TEXT, enums are stored as their `Display`/string names (or numeric for `Priority`) — route all conversions through these helpers rather than deserializing directly.
- `src/tui/` — command-driven ratatui app: `app.rs` (event loop), `command.rs` (internal command enum), `state.rs` (shared state), `component/` (panes; `root.rs` dispatches; `popup/` are overlays; `shared/` are reusable widgets).
- The sqlite pool is `max_connections(1)` with `foreign_keys(true)` — no concurrent writers.

## Release process

- Version bumps are scripted in the justfile: `just release-patch|minor|major` (append `-and-push` to also push). The script edits the `Cargo.toml` version, commits `vX.Y.Z`, and tags it.
- Pushing a `v*` tag triggers `.github/workflows/release.yml`, which cross-compiles and publishes `scry-<target>.tar.gz` tarballs; `install.sh` downloads exactly those artifact names.
- Keep the README CLI reference and `install.sh` in sync when adding/changing commands or flags.
