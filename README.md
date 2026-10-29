# scry

A task manager for the terminal.

## Overview

scry organizes work into **projects** with customizable **statuses** (kanban columns). Tasks live within a project and move between statuses. A built-in "default" project with `todo` / `done` statuses provides a simple out-of-box todo list experience — no setup required.

## Installation

### Shell installer (Linux & macOS)

```sh
curl -fsSL https://raw.githubusercontent.com/paulwrubel/scry/main/install.sh | sh
```

This downloads the latest release and installs to `~/.local/bin`. Make sure `~/.local/bin` is in your `PATH`.

### Manual download

Download the latest binary from the [Releases page](https://github.com/paulwrubel/scry/releases/latest).

### Build from source

Requires [Rust](https://www.rust-lang.org/tools/install).

```sh
git clone https://github.com/paulwrubel/scry.git
cd scry
cargo build --release
./target/release/scry --help
```

## Docker

Prebuilt images are published to GitHub Container Registry on each release as
`ghcr.io/paulwrubel/scry` (see [Published images](#published-images)). The image's default
command serves the REST API on port 8000; the MCP surfaces are one command override away. To
build an image from source instead, use:

```sh
docker build -t scry .
```

### Published images

Each release publishes the image tagged with its version (e.g.
`ghcr.io/paulwrubel/scry:0.22.0`) and with `latest`; pin a specific version rather than
tracking `latest`. Pull it and run the image with its default command, which serves the REST
API on port 8000:

```sh
docker pull ghcr.io/paulwrubel/scry:0.22.0
docker run --rm -p 8000:8000 -v "$PWD/scry.docker.toml:/app/scry.toml:ro" ghcr.io/paulwrubel/scry:0.22.0
```

Passing a command overrides the image default. The same tag also serves the stdio MCP server
or the MCP server over HTTP:

```sh
docker run -i --rm -v "$PWD/scry.docker.toml:/app/scry.toml:ro" ghcr.io/paulwrubel/scry:0.22.0 mcp
docker run --rm -p 8000:8000 -v "$PWD/scry.docker.toml:/app/scry.toml:ro" ghcr.io/paulwrubel/scry:0.22.0 mcp --http 0.0.0.0:8000
```

See [Running a server](#running-a-server) for the host-side REST API setup and the
[MCP Server](#mcp-server) section for the tools.

- GitHub Container Registry packages are private by default: for an unauthenticated pull (e.g.
  from a k3s node), switch the package to public in the repository's package settings, or give
  the cluster an image pull secret.

### stdio

The client launches the container as a child process; its stdin/stdout carry the protocol.

```sh
docker run -i --rm -v "$PWD/scry.docker.toml:/app/scry.toml:ro" scry mcp
```

`-i` keeps stdin open (the stdio transport requires it); do not add `-t`, which can corrupt the framing. A client config looks like:

```json
{
  "mcpServers": {
    "scry": {
      "command": "docker",
      "args": ["run", "-i", "--rm", "-v", "./scry.docker.toml:/app/scry.toml:ro", "scry", "mcp"]
    }
  }
}
```

### HTTP

```sh
SCRY_MCP_TOKEN=$(openssl rand -hex 32) docker compose up -d
```

This starts the server on `http://127.0.0.1:8000/mcp`, published on host loopback only. The container reads its config from `/app/scry.toml`, which compose mounts from `scry.docker.toml`; that config's `database_url` points at an external PostgreSQL server. A client config looks like:

```json
{
  "mcpServers": {
    "scry": {
      "type": "http",
      "url": "http://127.0.0.1:8000/mcp",
      "headers": { "Authorization": "Bearer <token>" }
    }
  }
}
```

The `headers` entry is only needed when `SCRY_MCP_TOKEN` is set; without it the endpoint is unauthenticated.

### Notes

- A reachable PostgreSQL server configured via `scry.toml` is required.
- The HTTP transport has no TLS; put it behind a TLS-terminating proxy or a private network if it leaves the host.
- Client config key names vary between harnesses.

## Core Concepts

### Projects

A project is a container for related tasks. Each project has its own set of statuses. The built-in "default" project exists automatically and can be used without creating anything.

### Statuses

Statuses are the columns a task moves through within a project. The "default" project comes with `todo` and `done`. Custom projects can have any number of arbitrarily named statuses (e.g., "backlog", "in progress", "review", "done"). Status names are case-sensitive.

### Active Project

scry uses a kubectl-style context model. `scry project use <name>` sets the active project, which persists across sessions. The active project is stored locally per machine, in `$XDG_STATE_HOME/scry/state.toml` (falling back to `~/.local/state/scry/state.toml`), overridable with the `state_file` config option, rather than in the backend, so each device that talks to a shared server keeps its own. All task commands operate on the active project unless overridden with `--project` / `-p`.

## Quick Start

```sh
# add a task
scry add "buy groceries"

# list tasks (kanban columns)
scry list

# move a task to done
scry move 1 done

# create a project with custom statuses
scry project create myapp
scry -p myapp project status add "in progress"
scry -p myapp project status add review
scry -p myapp add "design API"
scry -p myapp move 1 "in progress"
```

## CLI Reference

### Global Flags

`-p, --project <name>` — target a specific project, overriding the active project.

`--color <auto|always|never>` — when to colorize output. `auto` (the default) colorizes only when stdout is a terminal and `NO_COLOR` is unset.

`--json` — emit machine-readable JSON instead of human-formatted text. Applies only to one-shot subcommands; passing `--json` with no subcommand or with `mcp`, `serve`, `docs`, or `skill` is an error. See [JSON Output](#json-output).

`--config <path>` — read configuration from this file instead of discovering `./scry.toml`, `$XDG_CONFIG_HOME/scry/scry.toml`, or the legacy config name.

`--secrets <path>` — read secrets from this file instead of discovering `./scry.secret.toml` or `$XDG_CONFIG_HOME/scry/scry.secret.toml`.

Running `scry` with no subcommand prints the top-level help and exits.

### Task Commands

| Command                                                                                                                      | Description                                                                     |
| ---------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| `scry add <title> [--description <text>] [--priority <level>] [--tags <comma,separated>] [--status <name>]`                  | Add a task (defaults to the project's entry status, otherwise the first status) |
| `scry list [--status <name>] [--search <text>]`                                                                              | List tasks grouped by status; `--search` matches titles and tags                |
| `scry move <id> <status>`                                                                                                    | Move a task to a new status (alias for `update --status`)                       |
| `scry update <id> [--title <text>] [--description <text>] [--priority <level>] [--tags <comma,separated>] [--status <name>]` | Update task properties; an empty `--description` or `--tags` clears it          |
| `scry duplicate <id>`                                                                                                        | Duplicate a task into the same status at the same position                      |
| `scry show <id>`                                                                                                             | Show full task details, including description, priority, tags, and notes        |
| `scry delete <id>`                                                                                                           | Delete a task permanently                                                       |
| `scry note add <task-id> <contents>`                                                                                         | Add a note to a task                                                            |

`<level>` is one of `minimal`, `low`, `medium`, `high`, `critical` (aliases `p5`–`p1`).

### Project Commands

| Command                                                                                                                                                        | Description                                                       |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------- |
| `scry project list`                                                                                                                                            | List all projects (`*` marks the active one)                      |
| `scry project current`                                                                                                                                         | Show the active project                                           |
| `scry project show [name]`                                                                                                                                     | Show a project's details, including its description               |
| `scry project use <name>`                                                                                                                                      | Set the active project                                            |
| `scry project create [-t <template>] [--description <text>] <name>`                                                                                            | Create a project (does not switch to it — run `scry project use`) |
| `scry project update [--name <new>] [--description <text>] [--entry-status <status>] [--clear-entry-status] [--sort <mode>] [--show-priority/--hide-priority]` | Update project properties; targets `-p` or the active project     |
| `scry project delete <name> [-f]`                                                                                                                              | Delete a project (prompts unless `--force`)                       |

`scry project update` applies multiple flags in one call; an empty `--description` clears the description, and `--clear-entry-status` clears the entry status.

`<mode>` is one of `alphabetical`, `alphabetical-case-insensitive`, `id`, `manual`, `priority`.

### Status Commands

| Command                                        | Description                               |
| ---------------------------------------------- | ----------------------------------------- |
| `scry project status list`                     | List statuses for the active project      |
| `scry project status add <name>`               | Add a new status                          |
| `scry project status remove <name>`            | Remove a status (refuses if it has tasks) |
| `scry project status rename <old> <new>`       | Rename a status                           |
| `scry project status set-color <name> <color>` | Set a status's color                      |
| `scry project status reset-color <name>`       | Clear a status's color                    |
| `scry project status set-style <name> <style>` | Set a status's style                      |
| `scry project status move-up <name>`           | Move a status up in the ordering          |
| `scry project status move-down <name>`         | Move a status down in the ordering        |

`<color>` is one of `black`, `red`, `green`, `yellow`, `blue`, `magenta`, `cyan`, `gray`, `dark-gray`, `light-red`, `light-green`, `light-yellow`, `light-blue`, `light-magenta`, `light-cyan`, `white`. `<style>` is one of `none`, `unchecked`, `checked`, `strikethrough`, `hidden`.

### Export & Import

Back up and restore projects as a single JSON document. `export` always writes to stdout and `import` always reads from stdin, so both compose with shell redirection.

| Command                                   | Description                                                                                |
| ----------------------------------------- | ------------------------------------------------------------------------------------------ |
| `scry export`                             | Write all projects (with statuses, tasks, and notes) as JSON to stdout                     |
| `scry -p <name> export`                   | Write a single project as JSON to stdout (`-p` is the global flag, placed before `export`) |
| `scry import`                             | Read a JSON backup from stdin and add its projects (fails if any name already exists)      |
| `scry import --replace-existing-projects` | Add new projects, replacing existing projects whose names collide                          |
| `scry import --skip-existing-projects`    | Add new projects, keeping existing projects whose names collide                            |

A plain `scry import` is strict: it adds the incoming projects but fails, writing nothing, if any name already exists. Because scry always ships a built-in `default` project, a full backup (which contains `default`) conflicts on a plain import — restoring one therefore requires `scry import --replace-existing-projects`. `--replace-existing-projects` adds new projects and replaces any existing project whose name collides, while `--skip-existing-projects` adds new projects and keeps the existing ones on collision; the two flags are mutually exclusive. `scry -p <name> export` is handy for sharing a single project that doesn't collide.

The typical round trip pipes one into the other:

```sh
scry export > scry-backup.json
scry import --replace-existing-projects < scry-backup.json
```

The document is a single object, `{ "version": 1, "app_version": "0.20.0", "projects": [ ... ] }`. `version` is the format version checked on import and `app_version` is informational, recording the binary that produced the backup. Each project carries its own fields (`id`, `name`, `description`, `entry_status_id`, `task_sorting_mode`, `show_priority`, `created_at`) alongside nested `statuses`, `tasks`, and `notes` arrays. Field names are snake_case, enum values are kebab-case strings — for example `"priority": "high"` and `"style": "checked"` — matching the values the CLI accepts and the `--json` output, and timestamps are RFC3339 strings.

Import assigns fresh ids and rewrites the related `project_id`, `status_id`, and `task_id` references while preserving the other fields and timestamps. The `version` field is validated, and a backup from an unsupported version is rejected. Under `--json`, `scry import` prints a report object with the integer fields `projects`, `statuses`, `tasks`, and `notes`.

### Documentation

Print the full scry README, which is embedded in the binary at build time, to stdout as raw
Markdown. It works offline with no network access, is suitable for piping to a pager or a Markdown
renderer, and never reads or writes the database.

| Command     | Description                        |
| ----------- | ---------------------------------- |
| `scry docs` | Print the bundled README to stdout |

### Skill Commands

Install an Agent Skills `SKILL.md` so coding agents (Claude Code, opencode, and any harness
that reads the open-standard path) can discover and drive the scry CLI.

| Command                                         | Description                                                     |
| ----------------------------------------------- | --------------------------------------------------------------- |
| `scry skill install --harness <name> [--force]` | Install at a harness's standard user-global location            |
| `scry skill install --dir <path> [--force]`     | Install into a custom skills directory (`<path>/scry/SKILL.md`) |
| `scry skill uninstall --harness <name>`         | Remove a harness's user-global skill                            |
| `scry skill uninstall --dir <path>`             | Remove a skill from a custom skills directory                   |
| `scry skill print`                              | Print the skill to stdout without writing anything              |

Exactly one of `--harness` or `--dir` is required. `--force` overwrites an existing skill file.

`<name>` is one of `claude-code`, `opencode`, `agents`, each of which has a standard
user-global location:

| `--harness`   | Location                                  |
| ------------- | ----------------------------------------- |
| `claude-code` | `~/.claude/skills/scry/SKILL.md`          |
| `opencode`    | `~/.config/opencode/skills/scry/SKILL.md` |
| `agents`      | `~/.agents/skills/scry/SKILL.md`          |

`--dir` is harness-agnostic: scry writes `<path>/scry/SKILL.md`, so it works with any harness or
location. For a repo-local install that travels with the repository, point it at the harness's
project directory, for example `scry skill install --dir .claude/skills`.

On the `--harness` paths, `~/.config` honors `$XDG_CONFIG_HOME` when it is set. opencode also reads
skills from `.claude/skills/` and `.agents/skills/`, so installing for `claude-code` or `agents`
covers opencode too. Do not install twice: skill names must be unique across discovery locations.

### MCP Server

Serve scry over the [Model Context Protocol](https://modelcontextprotocol.io) so MCP clients
(Claude Code, opencode, and others) can drive it through tools instead of the CLI.

| Command                  | Description                                                 |
| ------------------------ | ----------------------------------------------------------- |
| `scry mcp`               | Serve over stdio (the client launches scry as a subprocess) |
| `scry mcp --http <addr>` | Serve over streamable HTTP at `http://<addr>/mcp`           |

Without `--http`, the server speaks the stdio transport: stdin/stdout carry the protocol and
logs go to stderr. With `--http`, scry serves at `http://<addr>/mcp`. Set `SCRY_MCP_TOKEN` — or
`mcp_token` in `scry.secret.toml` — to require an `Authorization: Bearer <token>` header on
every HTTP request; requests are answered statelessly with JSON.

The tools mirror the CLI: tasks, notes, projects, statuses, and project settings. Results are
structured JSON, and errors are returned as tool results carrying a stable `kind` (`not_found`,
`conflict`, `invalid`, or `internal`). See [Docker](#docker) for container usage and client
configuration examples.

### REST API

Serve scry's operations as a versioned JSON REST API. This is a server surface: the CLI
still talks to its configured database directly (see [Database](#database)).

| Command                    | Description                                  |
| -------------------------- | -------------------------------------------- |
| `scry serve --http <addr>` | Serve the REST API at `http://<addr>/api/v1` |

#### Running a server

`scry serve` reads `database_url` only from `scry.toml` or `scry.secret.toml`, so it serves
whichever database that machine is configured with. On the host that holds the data, point
`scry.toml` at the database:

```toml
database_url = "postgres://user:pass@localhost/scry"
```

Then start the server:

```sh
# optional; omit the token for an unauthenticated server
printf 'api_token = "%s"\n' "$(openssl rand -hex 32)" > scry.secret.toml
scry serve --http 0.0.0.0:8000
```

The token lives in `scry.secret.toml` (see [Configuration](#configuration)); exporting
`SCRY_API_TOKEN` still works.

`--http` takes a literal `ip:port`; a hostname such as `localhost` is rejected. Use
`127.0.0.1:8000` to accept only local connections, or `0.0.0.0:8000` to accept any.
Confirm the server is up:

```sh
curl http://127.0.0.1:8000/api/v1
# {"name":"scry","version":"0.21.2"}
```

The container image runs it the same way:

```sh
docker run -d -p 8000:8000 -v "$PWD/scry.docker.toml:/app/scry.toml:ro" scry serve --http 0.0.0.0:8000
```

Setting `api_token` in `scry.secret.toml` (or `SCRY_API_TOKEN`) requires an
`Authorization: Bearer <token>` header on every request
(compared in constant time). Responses are JSON; errors carry the same stable `kind`
vocabulary as `--json` and MCP (`not_found`, `conflict`, `invalid`, `internal`, `config`)
and map to the matching HTTP status (404, 409, 400, 500). There is no TLS, so put the
server behind a TLS-terminating proxy or a private network if it leaves the host.

Endpoints, all under `/api/v1`:

| Method                   | Path                                     | Purpose                             |
| ------------------------ | ---------------------------------------- | ----------------------------------- |
| `GET`                    | `/`                                      | Server info                         |
| `GET` / `POST`           | `/projects`                              | List / create projects              |
| `GET` / `DELETE`         | `/projects/by-name/{name}`               | Fetch / delete a project by name    |
| `GET` / `PUT` / `DELETE` | `/projects/{id}`                         | Fetch / update / delete a project   |
| `GET` / `POST`           | `/projects/{id}/statuses`                | List / create statuses              |
| `GET`                    | `/projects/{id}/statuses/by-name/{name}` | Fetch a status by name              |
| `GET` / `POST`           | `/projects/{id}/tasks`                   | List / create a project's tasks     |
| `GET`                    | `/projects/{id}/notes`                   | List a project's notes              |
| `GET`                    | `/projects/{id}/snapshot`                | Fetch a project snapshot            |
| `GET` / `PUT` / `DELETE` | `/statuses/{id}`                         | Fetch / update / delete a status    |
| `POST`                   | `/statuses/{id}/move`                    | Reorder a status within its project |
| `GET`                    | `/statuses/{id}/tasks`                   | List a status's tasks               |
| `GET` / `PUT` / `DELETE` | `/tasks/{id}`                            | Fetch / update / delete a task      |
| `POST`                   | `/tasks/{id}/move`                       | Move a task (auto-positions it)     |
| `GET` / `POST`           | `/tasks/{id}/notes`                      | List / create a task's notes        |
| `GET` / `PUT` / `DELETE` | `/notes/{id}`                            | Fetch / update / delete a note      |
| `POST`                   | `/import`                                | Apply an import plan                |

### JSON Output

The global `--json` flag makes one-shot subcommands emit compact JSON instead of human-formatted text:

```sh
scry --json list
scry --json show 42
scry -p myapp --json add "design API"
```

Collection commands emit a top-level JSON array; single-resource and mutation commands emit a bare model object, with no wrapping key. Every mutation returns the affected model — including deletions, which return the object that was removed — and `show` additionally includes the task's `notes`. Field names are snake_case, while enum values are kebab-case and match the CLI's accepted arguments, so a value read from JSON can be passed straight back to the CLI.

`--json` applies only to one-shot subcommands; passing it with no subcommand or with `mcp`, `serve`, `docs`, or `skill` is an error. Color is always disabled under `--json`, stdout carries only the JSON document, and every failing command exits non-zero.

On failure, a structured error document is written to stderr:

```json
{"error": {"kind": "not_found", "message": "Task 42 not found in \"default\""}}
```

`kind` is one of `not_found`, `conflict`, `invalid`, `internal`, or `config`.

Because the output is the bare data, some context that the human output carries is not repeated: `list` does not include the project or its status definitions, and `project list` does not mark the active project. Consumers that need those can call `scry --json project current` and `scry --json project status list`.

## Configuration

scry looks for `scry.toml` in the current directory first, then `$XDG_CONFIG_HOME/scry/scry.toml` (falling back to `~/.config/scry/scry.toml`), and finally the legacy `$XDG_CONFIG_HOME/scry/config.toml`. The file is never created automatically — create one manually only if you want to override the defaults. Pass `--config <path>` to load a specific file instead of discovering one.

Available options:

- `database_url` — the PostgreSQL connection URL, required: `database_url = "postgres://user:pass@localhost/scry"` (see [Database](#database)).
- `state_file` — override the location of the client-local state file that records the active project. A leading `~` expands to `$HOME`; a relative path resolves against the current directory. If unset, scry uses `$XDG_STATE_HOME/scry/state.toml`, falling back to `~/.local/state/scry/state.toml` (see [Active Project](#active-project)).

The active project is **not** stored here or in the database — it lives in the XDG state directory, per machine (see [Active Project](#active-project)).

Credentials belong in a separate `scry.secret.toml`, discovered the same way (`./scry.secret.toml`, then `$XDG_CONFIG_HOME/scry/scry.secret.toml`), or loaded explicitly with `--secrets <path>`. It is never created automatically and is listed in `.gitignore`, so keep it out of version control. Its values take precedence over both `scry.toml` and the environment variables:

- `database_url` — override `scry.toml`'s `database_url`.
- `api_token` — the REST API bearer token, overriding `SCRY_API_TOKEN`.
- `mcp_token` — the MCP bearer token, overriding `SCRY_MCP_TOKEN`.

## Database

scry stores its data in PostgreSQL. `database_url` must be set in `scry.toml` or `scry.secret.toml`; if neither sets it, startup fails with a configuration error:

```toml
database_url = "postgres://user:pass@localhost/scry"
```

The URL syntax is `postgres://user:pass@host/db` (or `postgresql://…`). TLS is supported via the URL's `sslmode` parameter (e.g. `?sslmode=require`), which remote and managed servers commonly require.

Migrations run automatically on startup, so no manual setup is needed.

The CLI connects to PostgreSQL directly and does not proxy through the REST server.

For local development against Postgres, the repo ships `compose.dev.yaml`, the committed `scry.dev.toml` and `scry.dev-secrets.toml`, and justfile helpers. `just run-local` starts a dev Postgres (`postgres:18`), applies the migrations, and runs the REST server with `--config scry.dev.toml --secrets scry.dev-secrets.toml`; `just run-postgres` only starts Postgres and applies migrations. `just pg-down` stops Postgres while keeping its data, and `just pg-reset` also wipes the data volume. `just run-docker` starts the MCP server stack in Docker against the host dev Postgres. After changing SQL inside a `sqlx::query!` macro, regenerate the committed offline query cache with `just sqlx-prepare`, which rebuilds only the Postgres cache.
