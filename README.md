# scry

A task manager for the terminal.

scry is client/server: `scry serve` owns a PostgreSQL database and exposes a REST API plus an embedded web UI, while the `scry` CLI is an HTTP client that keeps no local copy of your data. A built-in `default` project means you can start adding tasks immediately.

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

## Backup and restore

`scry export > backup.json` writes every project, with its sections, tasks, and notes, as JSON; `scry import < backup.json` merges it back. Put `-p <name>` before `export` to back up a single project.

## Core concepts

- **Projects** group related tasks and own optional ordered sections. The built-in `default` project exists automatically.
- **State**: every task is `open` or `closed`. Closing records whether it was `completed` or `cancelled` and when; reopening restores the task where it was.
- **Sections** are optional ordered groups within a project. Tasks without one appear under **No section**. New sections append and cannot be reordered.
  - Create sections explicitly with `scry section add <name>` or `--new-section <name>`. `--section <name>` must match an existing name exactly.
  - Deleting a section moves its tasks to No section. Merging moves the source's tasks into the target and deletes the source.
  - Sections have no completion semantics: a section named "Done" does not close anything.
- **Active project** works like a kubectl context. `scry project use <name>` verifies the name with the server and stores the choice per machine in `$XDG_STATE_HOME/scry/state.toml` (or the `state_file` config option), not in the backend. Task commands use it unless you pass `-p/--project`.

## Docker

Each release publishes `ghcr.io/paulwrubel/scry`, tagged with its version and `latest`. The runtime-only image serves the API and web UI on port 8000. Create a `scry.toml` pointing at your PostgreSQL server:

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

Start the server with `scry serve --http 127.0.0.1:8000`. It serves the REST API at `/api/v1` and, in the default build, the embedded web UI at `/`. `--http` takes a literal `ip:port`; a hostname such as `localhost` is rejected. Check it with `curl http://127.0.0.1:8000/api/v1`.

Set `api_token` in `scry.secret.toml` (or `SCRY_API_TOKEN`) to require authentication. Non-browser clients send `Authorization: Bearer <token>`; browsers exchange the token at `POST /api/v1/auth/session` for a `scry_session` cookie. With no token, the API is unauthenticated. There is no TLS, so use a TLS-terminating proxy if the server leaves the host.

For scripting, `--json` emits machine-readable output (for example, `scry --json list`). Collection commands emit arrays, single-resource and mutation commands emit bare objects, and errors carry a stable `kind`: `not_found`, `conflict`, `invalid`, `internal`, or `config`.

## Documentation

Run `scry --help` for commands and `scry <command> --help` for a command's flags. `scry docs` prints this README from the installed binary.
