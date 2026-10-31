# scry

A task manager for the terminal.

## Overview

scry organizes work into **projects**. Every task is either **open** or **closed**; a closed task records why it closed (`completed` or `cancelled`) and when. Tasks may also belong to an optional, ordered **section** within their project, and tasks with no section appear under **No section**. A built-in "default" project with no sections provides a simple out-of-box todo list experience — no setup required.

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
command serves the REST API on port 8000. To build an image from source instead, use:

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

See [Running a server](#running-a-server) for the host-side REST API setup.

- GitHub Container Registry packages are private by default: for an unauthenticated pull (e.g.
  from a k3s node), switch the package to public in the repository's package settings, or give
  the cluster an image pull secret.

### Notes

- A reachable PostgreSQL server configured via `scry.toml` is required when you run the published image, or an image you built yourself, against a database of your choosing.
- `just run` builds the image and starts it together with its own Postgres container, so no host database is needed; see [Database](#database) for that workflow and the other local helpers.
- The REST API has no TLS; put it behind a TLS-terminating proxy or a private network if it leaves the host.

## Core Concepts

### Projects

A project is a container for related tasks. Each project has its own optional, ordered sections. The built-in "default" project exists automatically and can be used without creating anything.

### Lifecycle

Every task is open or closed. `scry close <handle> --reason completed` closes a task as `completed`, `scry close <handle> --reason cancelled` closes it as `cancelled`, and `scry open <handle>` returns it to open. Closing records `closed_at`; opening clears both the timestamp and the reason. Close and open do not change a task's section or position, so reopening restores it where it was.

### Sections

Sections are optional, ordered groupings within a project. A task has zero or one section. Tasks without a section are grouped under **No section**.

Sections are created explicitly, either with `scry section add <name>` or with `--new-section <name>` when adding or updating a task. `--section <name>` must name an existing section; an unknown name fails with the closest match suggested. Section names are trimmed and case-insensitively unique per project. There is no default or entry section: new tasks go to No section unless a section is specified, and `--no-section` on `move` or `update` moves a task back to No section.

Sections carry no completion semantics: a section named "Done" does not close anything. Deleting a section moves its tasks to No section; merging two sections moves the source's tasks into the target and deletes the source. Empty sections are allowed.

### Active Project

scry uses a kubectl-style context model. `scry project use <name>` sets the active project, which persists across sessions. The active project is stored locally per machine, in `$XDG_STATE_HOME/scry/state.toml` (falling back to `~/.local/state/scry/state.toml`), overridable with the `state_file` config option, rather than in the backend, so each device that talks to a shared server keeps its own. All task commands operate on the active project unless overridden with `--project` / `-p`.

## Quick Start

```sh
# add a task
scry add "buy groceries"

# list tasks grouped by section
scry list

# close a task as completed (or cancel it with --reason cancelled)
scry close 4f3a --reason completed

# create a project with sections
scry project create myapp
scry -p myapp section add "in progress"
scry -p myapp section add review
scry -p myapp add "design API" --section "in progress"
scry -p myapp add "write docs" --new-section docs
scry -p myapp move 4f3a review
scry -p myapp list --section review
```

## CLI Reference

### Global Flags

`-p, --project <name>` — target a specific project, overriding the active project.

`--color <auto|always|never>` — when to colorize output. `auto` (the default) colorizes only when stdout is a terminal and `NO_COLOR` is unset.

`--json` — emit machine-readable JSON instead of human-formatted text. Applies only to one-shot subcommands; passing `--json` with no subcommand or with `serve`, `docs`, or `skill` is an error. See [JSON Output](#json-output).

`--config <path>` — read configuration from this file instead of discovering `./scry.toml`, `$XDG_CONFIG_HOME/scry/scry.toml`, or the legacy config name.

`--secrets <path>` — read secrets from this file instead of discovering `./scry.secret.toml` or `$XDG_CONFIG_HOME/scry/scry.secret.toml`.

Running `scry` with no subcommand prints the top-level help and exits.

### Task Commands

| Command                                                                                                                                                                 | Description                                                                         |
| ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| `scry add <title> [--description <text>] [--priority <level>] [--tags <comma,separated>] [--section <name>] [--new-section <name>]`                                     | Add a task (defaults to No section)                                                 |
| `scry list [--section <name>] [--search <text>] [--include-closed]`                                                                                                     | List open tasks grouped by section; `--include-closed` adds a trailing Closed group |
| `scry move <handle> [<section>] [--no-section]`                                                                                                                         | Move a task to a section, or to No section (appends to the end of the group)        |
| `scry update <handle> [--title <text>] [--description <text>] [--priority <level>] [--tags <comma,separated>] [--section <name>] [--new-section <name>] [--no-section]` | Update task properties; an empty `--description` or `--tags` clears it              |
| `scry close <handle> --reason <completed \| cancelled>`                                                                                                                 | Close a task with a reason                                                          |
| `scry open <handle>`                                                                                                                                                    | Reopen a closed task                                                                |
| `scry duplicate <handle>`                                                                                                                                               | Duplicate a task into the same section, immediately after the source                |
| `scry show <handle>`                                                                                                                                                    | Show full task details, including lifecycle, section, priority, tags, and notes     |
| `scry delete <handle>`                                                                                                                                                  | Delete a task permanently                                                           |
| `scry note add <task-handle> <contents>`                                                                                                                                | Add a note to a task                                                                |

Task and note arguments accept a full UUID or a unique short prefix of at least 4 lowercase hex characters. Dashes are optional and must sit at canonical UUID positions (after 8, 12, 16, or 20 hex characters). Human output prints the shortest unique prefix, while `--json` always emits full UUIDs; scripts should prefer those, because a handle can lengthen as more entities are created.

`<level>` is one of `minimal`, `low`, `medium`, `high`, `critical` (aliases `p5`–`p1`).

With `--include-closed`, `scry list` renders closed tasks in a trailing **Closed** group, marking completed tasks `[x]` and cancelled tasks `[~]`. Without it, closed tasks are hidden. An unknown `--section` name fails with a suggestion naming the closest existing section.

### Project Commands

| Command                                                                                                       | Description                                                       |
| ------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------- |
| `scry project list`                                                                                           | List all projects (`*` marks the active one)                      |
| `scry project current`                                                                                        | Show the active project                                           |
| `scry project show [name]`                                                                                    | Show a project's details, including its description               |
| `scry project use <name>`                                                                                     | Set the active project                                            |
| `scry project create [-t <template>] [--description <text>] <name>`                                           | Create a project (does not switch to it — run `scry project use`) |
| `scry project update [--name <new>] [--description <text>] [--sort <mode>] [--show-priority/--hide-priority]` | Update project properties; targets `-p` or the active project     |
| `scry project delete <name> [-f]`                                                                             | Resolve name to id, then delete (prompts unless `--force`)        |

`scry project update` applies multiple flags in one call; an empty `--description` clears the description.

`<mode>` is one of `alphabetical`, `alphabetical-case-insensitive`, `created`, `manual`, `priority` (`id` is accepted as an alias).

`scry project create -t <template>` seeds a project's sections and settings. The `todolist` template has zero sections and manual sorting; the `kanban` template has `backlog`, `in-progress`, and `review` sections and priority sorting. Creating a project without a template leaves it with no sections; tasks still work and land in No section.

### Section Commands

Sections belong to the active project, or to `-p` when given.

| Command                                     | Description                                                   |
| ------------------------------------------- | ------------------------------------------------------------- |
| `scry section list`                         | List sections for the project                                 |
| `scry section add <name> [--color <color>]` | Add a new section, optionally with a color                    |
| `scry section remove <name>`                | Delete a section, moving its tasks to No section              |
| `scry section rename <old> <new>`           | Rename a section                                              |
| `scry section merge <source> <target>`      | Move the source's tasks into the target and delete the source |
| `scry section move-up <name>`               | Move a section up in the ordering                             |
| `scry section move-down <name>`             | Move a section down in the ordering                           |
| `scry section set-color <name> <color>`     | Set a section's color                                         |
| `scry section reset-color <name>`           | Clear a section's color                                       |

`<color>` is one of `black`, `red`, `green`, `yellow`, `blue`, `magenta`, `cyan`, `gray`, `dark-gray`, `light-red`, `light-green`, `light-yellow`, `light-blue`, `light-magenta`, `light-cyan`, `white`.

### Export & Import

Back up and restore projects as a single JSON document. `export` always writes to stdout and `import` always reads from stdin, so both compose with shell redirection.

| Command                                   | Description                                                                                |
| ----------------------------------------- | ------------------------------------------------------------------------------------------ |
| `scry export`                             | Write all projects (with sections, tasks, and notes) as JSON to stdout                     |
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

The document is a single object, `{ "version": 3, "app_version": "0.24.0", "projects": [ ... ] }`. `version` is the format version checked on import and `app_version` is informational, recording the binary that produced the backup. Each project carries its own fields (`id`, `name`, `description`, `task_sorting_mode`, `show_priority`, `created_at`) alongside nested `sections`, `tasks`, and `notes` arrays. Every id (`id`, `project_id`, `section_id`, `task_id`) is a UUID string. Every task carries a nullable `section_id` and a `lifecycle` object: `{"state": "open"}` for an open task, or `{"state": "closed", "closed_at": "...", "reason": "completed"}` (or `"reason": "cancelled"`) for a closed one. Field names are snake_case, enum values are kebab-case strings — for example `"priority": "high"` — matching the values the CLI accepts and the `--json` output, and timestamps are RFC3339 strings.

Import preserves the UUIDs carried by the backup and inserts or updates by id, keeping the other fields and timestamps. Only backup format version 3 is accepted: version 2 and earlier backups (written before UUID identity) are rejected and must be migrated separately; upgrading them is tracked in #88. Under `--json`, `scry import` prints a report object with the integer fields `projects`, `sections`, `tasks`, and `notes`.

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
vocabulary as `--json` (`not_found`, `conflict`, `invalid`, `internal`, `config`)
and map to the matching HTTP status (404, 409, 400, 500). There is no TLS, so put the
server behind a TLS-terminating proxy or a private network if it leaves the host.

Endpoints, all under `/api/v1`:

| Method                   | Path                                     | Purpose                                |
| ------------------------ | ---------------------------------------- | -------------------------------------- |
| `GET`                    | `/`                                      | Server info                            |
| `GET` / `POST`           | `/projects`                              | List / create projects                 |
| `GET`                    | `/projects/by-name/{name}`               | Fetch a project by name                |
| `GET` / `PUT` / `DELETE` | `/projects/{id}`                         | Fetch / update / delete a project      |
| `GET` / `POST`           | `/projects/{id}/sections`                | List / create sections                 |
| `GET`                    | `/projects/{id}/sections/by-name/{name}` | Fetch a section by name                |
| `GET` / `POST`           | `/projects/{id}/tasks`                   | List / create a project's tasks        |
| `GET`                    | `/projects/{id}/notes`                   | List a project's notes                 |
| `GET`                    | `/projects/{id}/snapshot`                | Fetch a project snapshot               |
| `GET` / `PUT` / `DELETE` | `/sections/{id}`                         | Fetch / update / delete a section      |
| `POST`                   | `/sections/{id}/move`                    | Reorder a section within its project   |
| `POST`                   | `/sections/{id}/merge`                   | Merge a section into another           |
| `GET`                    | `/sections/{id}/tasks`                   | List a section's tasks                 |
| `GET` / `PUT` / `DELETE` | `/tasks/{id}`                            | Fetch / update / delete a task         |
| `POST`                   | `/tasks/{id}/move`                       | Move a task to a section at a position |
| `POST`                   | `/tasks/{id}/close`                      | Close a task with a reason             |
| `POST`                   | `/tasks/{id}/open`                       | Reopen a closed task                   |
| `GET` / `POST`           | `/tasks/{id}/notes`                      | List / create a task's notes           |
| `GET` / `PUT` / `DELETE` | `/notes/{id}`                            | Fetch / update / delete a note         |
| `POST`                   | `/import`                                | Apply an import plan                   |

Path and body ids are full UUIDs only; short prefixes are a CLI convenience and are rejected by the API.

`POST /tasks/{id}/move` takes a body of `{ "section_id": <id-or-null>, "position": <integer> }`, where an omitted `position` appends to the destination and a null `section_id` moves the task to No section. `POST /tasks/{id}/close` requires a body of `{ "reason": "completed" }` or `{ "reason": "cancelled" }`. `POST /sections/{id}/move` takes `{ "position": <integer> }`, and `POST /sections/{id}/merge` takes `{ "target_id": <section-id> }`.

### JSON Output

The global `--json` flag makes one-shot subcommands emit compact JSON instead of human-formatted text:

```sh
scry --json list
scry --json show 4f3a
scry -p myapp --json add "design API"
```

Collection commands emit a top-level JSON array; single-resource and mutation commands emit a bare model object, with no wrapping key. Every mutation returns the affected model — including deletions, which return the object that was removed — and `show` additionally includes the task's `notes`. Task objects carry a nullable `section_id` and a `lifecycle` object, which is `{"state": "open"}` or `{"state": "closed", "closed_at": "...", "reason": "completed" | "cancelled"}`. Every `id` field is a full lowercase hyphenated UUID string, never a handle. Field names are snake_case, while enum values are kebab-case and match the CLI's accepted arguments, so a value read from JSON can be passed straight back to the CLI.

`--json` applies only to one-shot subcommands; passing it with no subcommand or with `serve`, `docs`, or `skill` is an error. Color is always disabled under `--json`, stdout carries only the JSON document, and every failing command exits non-zero.

On failure, a structured error document is written to stderr:

```json
{"error": {"kind": "not_found", "message": "No task matches short id \"4f3a\""}}
```

`kind` is one of `not_found`, `conflict`, `invalid`, `internal`, or `config`.

Because the output is the bare data, some context that the human output carries is not repeated: `list` does not include the project or its section definitions, and `project list` does not mark the active project. Consumers that need those can call `scry --json project current` and `scry --json section list`.

## Configuration

scry looks for `scry.toml` in the current directory first, then `$XDG_CONFIG_HOME/scry/scry.toml` (falling back to `~/.config/scry/scry.toml`), and finally the legacy `$XDG_CONFIG_HOME/scry/config.toml`. The file is never created automatically — create one manually only if you want to override the defaults. Pass `--config <path>` to load a specific file instead of discovering one.

Available options:

- `database_url` — the PostgreSQL connection URL, required: `database_url = "postgres://user:pass@localhost/scry"` (see [Database](#database)).
- `state_file` — override the location of the client-local state file that records the active project. A leading `~` expands to `$HOME`; a relative path resolves against the current directory. If unset, scry uses `$XDG_STATE_HOME/scry/state.toml`, falling back to `~/.local/state/scry/state.toml` (see [Active Project](#active-project)).

The active project is **not** stored here or in the database — it lives in the XDG state directory, per machine (see [Active Project](#active-project)).

Credentials belong in a separate `scry.secret.toml`, discovered the same way (`./scry.secret.toml`, then `$XDG_CONFIG_HOME/scry/scry.secret.toml`), or loaded explicitly with `--secrets <path>`. It is never created automatically and is listed in `.gitignore`, so keep it out of version control. Its values take precedence over both `scry.toml` and the environment variables:

- `database_url` — override `scry.toml`'s `database_url`.
- `api_token` — the REST API bearer token, overriding `SCRY_API_TOKEN`.

## Database

scry stores its data in PostgreSQL. `database_url` must be set in `scry.toml` or `scry.secret.toml`; if neither sets it, startup fails with a configuration error:

```toml
database_url = "postgres://user:pass@localhost/scry"
```

The URL syntax is `postgres://user:pass@host/db` (or `postgresql://…`). TLS is supported via the URL's `sslmode` parameter (e.g. `?sslmode=require`), which remote and managed servers commonly require.

Migrations run automatically on startup, so no manual setup is needed.

The CLI connects to PostgreSQL directly and does not proxy through the REST server.

For local development against Postgres, the repo ships a single `compose.yaml` that brings up the app and a `postgres:18` database together, the committed `scry.dev.toml` and `scry.dev-secrets.toml`, and `scry.compose.toml`, the config the container mounts as `/app/scry.toml`.

`just run` builds the image and starts the whole stack in Docker with no host Postgres required, and `just view-logs` tails the app container. `just run-local` builds the release binary and runs the REST server on the host against the same Postgres, while `just run-postgres` starts only Postgres and applies the migrations. `just pg-down` stops Postgres while keeping its data, `just pg-reset` tears the stack down and wipes the data volume, and `just clean` runs `cargo clean` and `docker compose down`.

`scry.docker.toml` stays the config for running the published image against a database of your choosing. After changing SQL inside a `sqlx::query!` macro, regenerate the committed offline query cache with `just sqlx-prepare`, which rebuilds only the Postgres cache.
