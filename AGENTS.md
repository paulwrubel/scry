# AGENTS.md

`scry` is a client/server task manager: a Rust 2024 Cargo workspace (root `scry` binary plus `crates/scry-core`) with the `scry` CLI and an offline-capable React/TypeScript web UI in `ui/` (a separate npm package). The CLI talks to the REST server over HTTP; only `scry serve` opens PostgreSQL. The browser app keeps an IndexedDB replica and a durable outbox so it can keep working offline. User-facing docs (CLI, API, config, database, web UI, PWA, offline behaviour, auth) live in `README.md`; keep its CLI reference and `install.sh` in sync when flags change.

## Commands

- `just` lists targets. `just validate` is the full local gate: builds the UI, then runs Rust test/check/clippy with default features, binding drift checks, and UI lint/prettier/type-check/Vitest. `check-bindings` depends on `generate-bindings`, which runs `prettier --write` over the whole `ui/` package, so the gate can rewrite working-tree formatting; only `ui/src/generated` is actually drift-checked.
- Rust: `just test` (`cargo-test`), `just check`, `just clippy` (all `--workspace`). One test: `cargo test -p scry <name>` or `cargo test -p scry-core <name>`. Keep clippy clean at `-D warnings`; CI pins Rust 1.98.1 because clippy 1.99 false-positives on `#[async_trait]`.
- CI variants `cargo-test-ci`, `cargo-check-ci`, and `cargo-clippy-ci` add `--no-default-features` and need no UI build. `.github/workflows/ci.yml` is currently `workflow_dispatch` only.
- UI (from `ui/`, or via the `just` wrappers): `npm run build|lint|prettier-check|type-check|test`. One Vitest file: `npm test -- <pattern>`. Node comes from `ui/.nvmrc` (26.x); `just setup-ui-env` installs it through nvm and runs `npm install`.
- Dev stack: `just run` (Docker app + Postgres), `just run-local` (host server against Docker Postgres), `just run-ui` (Docker + Vite on :5173), `just run-postgres`, `just pg-down`, `just pg-reset`, `just view-logs`, `just clean`. The dev API token is `admin` in `scry.dev-secrets.toml`.

## Must-know gotchas

- **Default features need `ui/dist`.** `embed-ui` embeds `ui/dist` with `include_dir!` at compile time, so plain `cargo build/test/check/clippy` fails on a fresh checkout until `just build-ui` (or `just build`) has run. `--no-default-features` builds an API-only binary.
- **The web UI is offline-first.** `ui/src/utils/replica/` owns the IndexedDB database `scry-replica`; confirmed state is disposable and can be re-fetched, but the durable outbox is not and must survive reloads, schema changes, and service-worker updates. The service worker caches only the app shell and is enabled only in production builds (`VitePWA` `devOptions.enabled` is false). Local data is per browser and per origin, so there is no cross-device offline merge.
- **sqlx is compile-time offline.** All SQL lives in `sqlx::query!`/`query_as!` in `src/store/postgres.rs` and resolves against the committed `.sqlx/` cache. The committed `.env` only sets `SQLX_OFFLINE=true`, so normal builds need no database. Editing SQL or adding a migration changes the query hash and breaks the build until `just sqlx-prepare` (needs Docker and sqlx-cli) regenerates `.sqlx/`; commit the migration and cache together. `just sqlx-prepare-check` is the CI guard.
- **Runtime config is not the compile DB.** `database_url` comes only from `scry.toml` / `scry.secret.toml` (or `--config`/`--secrets`), with no DB environment override and no auto-created file; a missing URL is a startup error, and only `postgres://`/`postgresql://` is accepted. Point it at a scratch database when running locally. TLS relies on sqlx's `tls-rustls` feature; do not remove it.
- **Migrations auto-run on every startup.** They are embedded via `sqlx::migrate!("./migrations")`. Add reversible pairs with `sqlx migrate add -r <name>`, fill both files, and keep new migrations additive and backward-compatible. There is no legacy data path: a database from before the UUID/position rewrite must be recreated. The snapshot import format is version 1 with no upgrade path.

## Architecture map

- `src/main.rs` — clap commands and dispatch. `src/client/**` is the HTTP client and never opens the database; `serve` is the only database-backed path; `docs` and `skill` are local.
- `src/service.rs` — `ProjectService`, the only write path. Mutations build `Operation` batches and apply them through `apply_operations` in one transaction (`Atomic` for REST, `PerOperation` for `/sync`) with version, `operation_id` dedupe, and tombstone checks, writing one `changes` row per affected entity. Import also goes through this pipeline via `scry_core::snapshot::plan_import`.
- `src/server/**` — axum router and one handler per endpoint. `src/store.rs` is the `Store` trait; `src/store/postgres.rs` is its sole implementation and the only place with SQL.
- `crates/scry-core/` — domain models, UUID ids and short handles, Rocicorp fractional `Position` keys (stored `TEXT COLLATE "C"`), shared `compare_tasks` ordering, snapshot/import planning, and the client-side replay model in `sync/replay.rs`.
- UI: `ui/src/App.tsx` defines the react-router routes; `ui/src/layouts/` holds the provider stack and authenticated/project layouts; `ui/src/views/` holds the pages. `ui/src/providers/` composes Auth, Replica, Sync, ActiveProject, and InstallPrompt around the non-React stores. `ui/src/utils/replica/` is the IndexedDB replica with a FIFO outbox; `ui/src/utils/sync/` is the non-React sync client that bootstraps, pushes the outbox, applies confirmed changes transactionally, and publishes status; `ui/src/contract/` holds the hand-maintained TypeScript ports. Forms use TanStack Form, board reordering uses `@dnd-kit/react` 0.5.x, and `vite-plugin-pwa` produces the service worker and manifest. The `embed-ui` feature includes `ui/dist` in the binary.
- **Browser auth is an interim `scry_session` cookie.** Auth accepts a bearer token or the signed `scry_session` cookie, which is `HttpOnly`, `SameSite=Strict`, `Path=/`, and lasts 30 days. The signing key is derived from SHA-256 of `api_token`, so rotating the token invalidates all sessions. #109 replaces this with a short-lived access token plus a refresh cookie scoped to `/api/v1/auth`.
- Call-graph detail for the service/store boundary: `docs/service-store-calls.md`.

## Cross-language contract

- Wire types derive `ts_rs::TS` behind the `ts` feature. `just generate-bindings` writes and formats `ui/src/generated/` (machine-owned — never hand-edit); `just check-bindings` fails on drift.
- The TypeScript ports in `ui/src/contract/` (`positions`, `handles`, `task-order`, `op-replay`) mirror Rust and are maintained by hand. There is no shared fixture system: when changing one side, update the other implementation and its tests in the same change.
- `crates/scry-core/src/sync/replay.rs` is the reference client model (confirmed state plus outbox replay); the server never calls it, and `ui/src/contract/op-replay.ts` mirrors it.

## Release

- `just release-patch|minor|major` (append `-and-push`) bumps `Cargo.toml`, runs `cargo check --no-default-features`, commits as `vX.Y.Z`, and tags it.
- Pushing a `v*` tag triggers `.github/workflows/release.yml`, which builds the UI before compiling the binary, publishes `scry-<target>.tar.gz` for four targets, and pushes the Docker image from the `x86_64-unknown-linux-gnu` artifact. The Dockerfile is runtime-only and copies `target/release/scry`.
