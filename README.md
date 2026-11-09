# scry

A client/server task manager for the terminal and the browser.

scry pairs the `scry` CLI with an embedded web UI. `scry serve` owns a PostgreSQL database and exposes a REST API plus the browser app, and the CLI is an HTTP client that keeps no local copy of your data. The web app is local-first: it keeps a confirmed replica and a durable outbox in the browser, so it keeps working offline. A built-in `default` project means you can start adding tasks immediately.

## Installation

### Shell installer (Linux & macOS)

```sh
curl -fsSL https://raw.githubusercontent.com/paulwrubel/scry/main/install.sh | sh
```

This downloads the latest release and installs to `~/.local/bin`. Make sure that directory is in your `PATH`.

### Manual download

Download the latest binary from the [releases page](https://github.com/paulwrubel/scry/releases/latest).

### Build from source

Building requires [Rust](https://www.rust-lang.org/tools/install), [just](https://github.com/casey/just), and Node 26 installed through nvm (see `ui/.nvmrc`). `just build` installs the UI dependencies, builds the web UI, and embeds it in the binary:

```sh
git clone https://github.com/paulwrubel/scry.git
cd scry
just build
./target/release/scry --help
```

For an API-only binary without the embedded web UI:

```sh
cargo build --release --no-default-features
```

## Quick start

Point the CLI at a running server, then use it like a local task manager:

```sh
export SCRY_SERVER_URL=http://127.0.0.1:8000
# Alternatively: pass --server <url> or set server_url in scry.toml.

scry add "buy groceries"   # prints the new task's handle
scry list
scry close <handle>        # use the handle printed by add

scry project create myapp
scry -p myapp section add "in progress"
scry -p myapp add "design API" --section "in progress"
scry -p myapp list --section "in progress"
```

Need a server? See [Docker](#docker) or [Server and API](#server-and-api).

## Web UI

The default build serves the embedded web app at `/` from the same origin as the API, so opening `http://127.0.0.1:8000/` in a browser is enough to start. Sign in with the server's `api_token`; the browser exchanges it for a short-lived access token and a refresh cookie (see [Browser authentication](#browser-authentication)).

The app is a PWA. Use the browser's install control, or add it to your home screen on mobile, to run it in its own window. Installation requires a secure context: HTTPS, or `localhost`/`127.0.0.1` during local development.

After the first visit the service worker caches the app shell, and the app keeps a confirmed replica plus a durable outbox in IndexedDB. While offline you can create, edit, move, close, and delete tasks; changes collect in the outbox and are pushed and pulled when the connection returns. The server remains the only writer and the authority on conflicts.

Offline data is per browser and per origin: the replica does not follow you to another device, and there is no cross-device offline merge. Clearing site data removes both the replica and the outbox, so unsynced edits are lost. Reloading while offline still shows the app shell, but it asks you to sign in because session status has to be checked with the server. The app does not rely on Background Sync or server-sent events; syncing happens while a tab is open and online.

### Browser authentication

Browser sessions use a short-lived access token plus a refresh credential. Signing in at `POST /api/v1/auth/session` exchanges `api_token` for a one-hour HS256 JWT kept only in JavaScript memory, and a 30-day refresh token in an `HttpOnly`, `SameSite=Lax`, `Path=/api/v1/auth` cookie. The refresh token is SHA-256 hashed in the database, rotated on every use, and reuse revokes the token lineage.

Normal API and sync requests send `Authorization: Bearer <access token>`; the refresh cookie is never accepted by those endpoints. A `401` during sync attempts one refresh and retries the request, preserving the outbox. If the refresh fails, the app returns to login without dropping queued edits. A reload recovers the session through `POST /api/v1/auth/refresh`. The signing key is derived from `api_token`, so rotating it invalidates existing access and refresh tokens. The cookie is marked `Secure` when the request arrives over HTTPS.

This remains a single-user authentication model. GitHub SSO and multi-user project ownership are separate future work.

## Backup and restore

`scry export > backup.json` writes every project, with its sections, tasks, and notes, as JSON; `scry import < backup.json` merges it back. Put `-p <name>` before `export` to back up a single project.

## Core concepts

- **Projects** group related tasks and own optional ordered sections. The built-in `default` project exists automatically.
- **State**: every task is `open` or `closed`. Closing records whether it was `completed` or `cancelled` and when; reopening restores the task where it was.
- **Sections** are optional ordered groups within a project. Tasks without one appear under **No section**. New sections append; drag them in the web UI to reorder.
  - Create sections explicitly with `scry section add <name>` or `--new-section <name>`. `--section <name>` must match an existing name exactly.
  - Sections can carry an optional color. Set one with `scry section add <name> --color <color>` or `scry section set-color <name> <color>`, remove it with `scry section reset-color <name>`, or click the circle in the web UI column header. Colors are kebab-case: `red`, `light-blue`, `dark-gray`, and so on.
  - Deleting a section moves its tasks to No section. Merging moves the source's tasks into the target and deletes the source.
  - Sections have no completion semantics: a section named "Done" does not close anything.
- **Active project** works like a kubectl context. `scry project use <name>` verifies the name with the server and stores the choice per machine in `$XDG_STATE_HOME/scry/state.toml` (or the `state_file` config option), not in the backend. Task commands use it unless you pass `-p/--project`.

## Docker

Each release publishes `ghcr.io/paulwrubel/scry`, tagged with its version and `latest`. The image is runtime-only: the web UI is embedded in the release binary at build time, and the image serves the API and web UI on port 8000. Create a `scry.toml` pointing at your PostgreSQL server:

```toml
database_url = "postgres://user:pass@host:5432/scry"
```

Then run:

```sh
docker run --rm -p 8000:8000 \
  -v "$PWD/scry.toml:/app/scry.toml:ro" \
  ghcr.io/paulwrubel/scry:latest
```

Pin a released version tag in production rather than tracking `latest`. Use `host.docker.internal` in place of `host` to reach a database on the Docker host. GHCR packages are private by default: make the package public or give the host an image pull secret.

## Configuration

scry reads `scry.toml` from the current directory, then `$XDG_CONFIG_HOME/scry/scry.toml`, then the legacy `$XDG_CONFIG_HOME/scry/config.toml`. Pass `--config <path>` to use a specific file. The file is never created automatically.

- `server_url` — the REST server the client talks to.
- `database_url` — the PostgreSQL URL used by `scry serve`; client commands do not use it.
- `state_file` — override the active-project state file location.

The server URL resolves from `--server`, then `SCRY_SERVER_URL`, then `server_url`.

Credentials belong in `scry.secret.toml` (discovered in the same locations, or passed with `--secrets`). It is gitignored and never created automatically. `database_url` there overrides `scry.toml`, and `api_token` overrides `SCRY_API_TOKEN`.

## Database

`scry serve` requires `database_url` in `scry.toml` or `scry.secret.toml`; if neither sets it, the server fails to start. Only `postgres://` and `postgresql://` URLs are accepted, and TLS works through the URL's `sslmode` parameter. Migrations run automatically on startup.

Only `scry serve` connects to PostgreSQL. The CLI sends every command to `server_url` over HTTP.

## Server and API

Start the server with `scry serve --http 127.0.0.1:8000`. The same origin serves the REST API at `/api/v1` and, in the default build, the embedded web UI at `/`. `--http` takes a literal `ip:port`; a hostname such as `localhost` is rejected. Check it with `curl http://127.0.0.1:8000/api/v1`.

Set `api_token` in `scry.secret.toml` (or `SCRY_API_TOKEN`) to require authentication. Non-browser clients send `Authorization: Bearer <token>`; browsers exchange the token at `POST /api/v1/auth/session` for a short-lived access token and a refresh cookie scoped to `/api/v1/auth`. Normal requests then send `Authorization: Bearer <access token>`. With no token, the API is unauthenticated. There is no TLS, so use a TLS-terminating proxy if the server leaves the host.

For scripting, `--json` emits machine-readable output (for example, `scry --json list`). Collection commands emit arrays, single-resource and mutation commands emit bare objects, and errors carry a stable `kind`: `not_found`, `conflict`, `invalid`, `internal`, or `config`.

## Documentation

Run `scry --help` for commands and `scry <command> --help` for a command's flags. `scry docs` prints this README from the installed binary.
