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

An image is provided for running the MCP server in a container.

```sh
docker build -t scry .
```

### stdio

The client launches the container as a child process; its stdin/stdout carry the protocol.

```sh
docker run -i --rm -v scry-data:/data scry mcp
```

`-i` keeps stdin open (the stdio transport requires it); do not add `-t`, which can corrupt the framing. A client config looks like:

```json
{
  "mcpServers": {
    "scry": {
      "command": "docker",
      "args": ["run", "-i", "--rm", "-v", "scry-data:/data", "scry", "mcp"]
    }
  }
}
```

### HTTP

```sh
SCRY_MCP_TOKEN=$(openssl rand -hex 32) docker compose up -d
```

This starts the server on `http://127.0.0.1:8000/mcp`, published on host loopback only, with the database on the `scry-data` volume. The container reads its config from `/app/scry.toml`, which compose mounts from `scry.docker.toml`. A client config looks like:

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

- The container runs as UID/GID `1000`; for a bind-mounted database owned by a different UID, run with `--user "$(id -u):$(id -g)"`.
- SQLite is single-writer: do not run the container and a host `scry` or TUI against the same database at once.
- The HTTP transport has no TLS; put it behind a TLS-terminating proxy or a private network if it leaves the host.
- Client config key names vary between harnesses.

## Core Concepts

### Projects

A project is a container for related tasks. Each project has its own set of statuses. The built-in "default" project exists automatically and can be used without creating anything.

### Statuses

Statuses are the columns a task moves through within a project. The "default" project comes with `todo` and `done`. Custom projects can have any number of arbitrarily named statuses (e.g., "backlog", "in progress", "review", "done"). Status names are case-sensitive.

### Active Project

scry uses a kubectl-style context model. `scry project use <name>` sets the active project, which persists across sessions. All task commands operate on the active project unless overridden with `--project` / `-p`.

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

`--json` — emit machine-readable JSON instead of human-formatted text. Applies only to one-shot subcommands; passing `--json` with no subcommand (the TUI) or with `mcp` is an error. See [JSON Output](#json-output).

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

| Command                                      | Description                                                       |
| -------------------------------------------- | ----------------------------------------------------------------- |
| `scry project list`                          | List all projects (`*` marks the active one)                      |
| `scry project current`                       | Show the active project                                           |
| `scry project use <name>`                    | Set the active project                                            |
| `scry project create [-t <template>] <name>` | Create a project (does not switch to it — run `scry project use`) |
| `scry project rename <old> <new>`            | Rename a project                                                  |
| `scry project delete <name> [-f]`            | Delete a project (prompts unless `--force`)                       |
| `scry project set-entry-status <name>`       | Set the entry status new tasks default to                         |
| `scry project reset-entry-status`            | Clear the entry status, so new tasks use the first status         |
| `scry project set-sort <mode>`               | Set the task sorting mode                                         |
| `scry project show-priority`                 | Show the priority level in task listings                          |
| `scry project hide-priority`                 | Hide the priority level in task listings                          |

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
logs go to stderr. With `--http`, the address decides exposure — binding a non-loopback
address such as `0.0.0.0:8000` makes the server reachable from other hosts, so scry warns on
startup. Set `SCRY_MCP_TOKEN` to require an `Authorization: Bearer <token>` header on every
HTTP request; requests are answered statelessly with JSON.

The tools mirror the CLI: tasks, notes, projects, statuses, and project settings. Results are
structured JSON, and errors are returned as tool results carrying a stable `kind` (`not_found`,
`conflict`, `invalid`, or `internal`). See [Docker](#docker) for container usage and client
configuration examples.

### JSON Output

The global `--json` flag makes one-shot subcommands emit compact JSON instead of human-formatted text:

```sh
scry --json list
scry --json show 42
scry -p myapp --json add "design API"
```

Collection commands emit a top-level JSON array; single-resource and mutation commands emit a bare model object, with no wrapping key. Every mutation returns the affected model — including deletions, which return the object that was removed — and `show` additionally includes the task's `notes`. Field names are snake_case, while enum values are kebab-case and match the CLI's accepted arguments, so a value read from JSON can be passed straight back to the CLI.

`--json` applies only to one-shot subcommands; passing it with no subcommand (which would launch the TUI) or with `mcp` is an error. Color is always disabled under `--json`, stdout carries only the JSON document, and every failing command exits non-zero.

On failure, a structured error document is written to stderr:

```json
{"error": {"kind": "not_found", "message": "Task 42 not found in \"default\""}}
```

`kind` is one of `not_found`, `conflict`, `invalid`, `internal`, or `config`.

Because the output is the bare data, some context that the human output carries is not repeated: `list` does not include the project or its status definitions, and `project list` does not mark the active project. Consumers that need those can call `scry --json project current` and `scry --json project status list`.

## Configuration

scry looks for `scry.toml` in the current directory first, then `$XDG_CONFIG_HOME/scry/scry.toml` (falling back to `~/.config/scry/scry.toml`), and finally the legacy `$XDG_CONFIG_HOME/scry/config.toml`. The file is never created automatically — create one manually only if you want to override the defaults.

Available options:

- `database_url` — set the database URL, e.g. `database_url = "postgres://user:pass@localhost/scry"`. If unset, scry uses the `DATABASE_URL` environment variable, falling back to a SQLite database (see [Database](#database)).

The active project is persisted automatically.

## Database

Tasks, projects, and statuses are stored in a SQLite database at `$XDG_DATA_HOME/scry/scry.db` (falling back to `~/.local/share/scry/scry.db`). No manual setup or migrations are required.
