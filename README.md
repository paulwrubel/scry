# scry

A task manager for the terminal.

## Overview

scry organizes work into **projects** with customizable **states** (kanban columns). Tasks live within a project and move between states. A built-in "default" project with `todo` / `done` states provides a simple out-of-box todo list experience — no setup required.

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

## Core Concepts

### Projects

A project is a container for related tasks. Each project has its own set of states. The built-in "default" project exists automatically and can be used without creating anything.

### States

States are the columns a task moves through within a project. The "default" project comes with `todo` and `done`. Custom projects can have any number of arbitrarily named states (e.g., "backlog", "in progress", "review", "done"). State names are case-sensitive.

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

# create a project with custom states
scry project create myapp
scry -p myapp project state add "in progress"
scry -p myapp project state add review
scry -p myapp add "design API"
scry -p myapp move 1 "in progress"
```

## CLI Reference

### Global Flag

`-p, --project <name>` — target a specific project, overriding the active project.

### Task Commands

| Command | Description |
|---------|-------------|
| `scry add <description>` | Add a task (defaults to `todo` state) |
| `scry list [--state <name>]` | List tasks grouped by state |
| `scry move <id> <state>` | Move a task to a new state (alias for `update --state`) |
| `scry update <id> --state <state>` | Update task properties |
| `scry show <id>` | Show full task details |
| `scry delete <id>` | Delete a task permanently |

### Project Commands

| Command | Description |
|---------|-------------|
| `scry project list` | List all projects (`*` marks active) |
| `scry project create <name>` | Create a project (auto-switches to it) |
| `scry project delete <name> [-f]` | Delete a project (prompts unless `--force`) |
| `scry project use <name>` | Set the active project |
| `scry project current` | Show the active project |

### State Commands

| Command | Description |
|---------|-------------|
| `scry project state list` | List states for the active project |
| `scry project state add <name>` | Add a new state |
| `scry project state remove <name> [-f]` | Remove a state (`--force` moves tasks) |
| `scry project state rename <old> <new>` | Rename a state |

## Configuration

On first run, scry creates `~/.config/scry/config.toml` with available options. The active project is persisted automatically.

## Database

Tasks, projects, and states are stored in a SQLite database at `$XDG_DATA_HOME/scry/scry.db` (falling back to `~/.local/share/scry/scry.db`). No manual setup or migrations are required.

## Roadmap

- [#5](https://github.com/paulwrubel/scry/issues/5) Rich task fields: due dates, priorities, tags, descriptions
- [#6](https://github.com/paulwrubel/scry/issues/6) Filtering and search for `scry list`
- [#7](https://github.com/paulwrubel/scry/issues/7) Task editing: update title and other fields
- [#8](https://github.com/paulwrubel/scry/issues/8) Terminal states: auto-record completed_at for "done"
- [#9](https://github.com/paulwrubel/scry/issues/9) Manual task reordering within a state
- [#10](https://github.com/paulwrubel/scry/issues/10) Archiving completed tasks
- [#11](https://github.com/paulwrubel/scry/issues/11) JSON export and import
- [#12](https://github.com/paulwrubel/scry/issues/12) Context-aware shell completions
