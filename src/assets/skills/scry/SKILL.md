---
name: scry
description: Manage tasks, todos, and projects with the scry terminal task manager CLI. Use when the user asks to track work, add or complete todos, move tasks through a workflow such as backlog, in-progress, or done, organize tasks into projects, or inspect existing tasks and notes.
license: MIT
---

# scry

`scry` is a task manager for the terminal. Drive it through its CLI to manage tasks and projects on the user's behalf.

## When to use this skill

Use it when the user asks to:

- Track work or manage a todo list.
- Add, update, complete, or delete tasks.
- Move tasks through a workflow (for example backlog -> in progress -> done).
- Organize work into projects, or inspect projects and their statuses.
- Read a task's details or notes.

## Discover the exact surface

`scry` is built with `clap`, so every command and subcommand supports `--help`. You can
append `--help` to essentially any command to get accurate, current usage, its arguments,
and the accepted values of any enum option (shown as `[possible values: ...]`). For
example: `scry --help`, `scry add --help`, or `scry project status set-style --help`.
Prefer this over guessing command names, flags, or values.

## Output for agents

- Always pass `--json` to task, project, and status commands. It returns compact,
  machine-readable JSON and disables color automatically, so never parse the
  human-readable text output.
- Enum values in JSON are kebab-case and match the CLI's accepted arguments, so a value
  read from JSON can be passed straight back to the CLI.
  - Examples: `scry --json list`, `scry --json show 42`, `scry -p myapp --json add "design API"`.
- `scry --json add` returns the new task including its `id`; read `id` from the JSON to
  chain follow-up calls.
- `scry --json show <id>` includes full detail (description, priority, tags, timestamps,
  and notes), while `scry --json list` omits descriptions, notes, and timestamps. Use
  `scry --json show` when full detail is needed.
- `scry --json list` output is the stable machine format; never parse the human-readable
  `list` columns without the `--json` flag.

## Active project

scry uses a kubectl-style active project. `scry project use <name>` sets it, and it
persists across sessions in a file on **this machine** (not in the database or on a
shared server, so each machine keeps its own). Every task command operates on the active
project unless overridden.

- Prefer `-p/--project <name>` when operating on a specific project, so you don't depend
  on the machine's current active project.
- If no active project is set — or the one it names no longer exists — commands fall back
  to the `default` project. A task command with no resolvable active project fails with an
  actionable error; set one with `scry project use <name>`.
- The built-in `default` project exists automatically with `todo` and `done` statuses;
  no setup is required.

## Core task commands

Every example below includes the global `--json` flag, which must come before the subcommand.

| Command                                                                                                  | Purpose                          |
| -------------------------------------------------------------------------------------------------------- | -------------------------------- |
| `scry --json add <title> [--description <text>] [--priority <level>] [--tags <a,b>] [--status <name>]`   | Add a task                       |
| `scry --json list [--status <name>] [--search <text>]`                                                   | List tasks grouped by status     |
| `scry --json show <id>`                                                                                  | Show full task details and notes |
| `scry --json move <id> <status>`                                                                         | Move a task to a status          |
| `scry --json update <id> [--title ...] [--description ...] [--priority ...] [--tags ...] [--status ...]` | Update task properties           |
| `scry --json duplicate <id>`                                                                             | Duplicate a task                 |
| `scry --json note add <task-id> <contents>`                                                              | Add a note to a task             |
| `scry --json delete <id>`                                                                                | Delete a task                    |

`<level>` is one of `minimal`, `low`, `medium`, `high`, `critical` (aliases `p5`-`p1`).

## Project and status commands

Every example below includes the global `--json` flag, which must come before the subcommand.

| Command                                                                                                                                                               | Purpose                                            |
| --------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------- |
| `scry --json project list`                                                                                                                                            | List projects (`*` marks the active one)           |
| `scry --json project current`                                                                                                                                         | Show the active project                            |
| `scry --json project show [name]`                                                                                                                                     | Show a project's details including its description |
| `scry --json project use <name>`                                                                                                                                      | Set the active project                             |
| `scry --json project create [-t <template>] [--description <text>] <name>`                                                                                            | Create a project                                   |
| `scry --json project update [--name <new>] [--description <text>] [--entry-status <status>] [--clear-entry-status] [--sort <mode>] [--show-priority/--hide-priority]` | Update project properties                          |
| `scry --json project delete <name> [-f]`                                                                                                                              | Delete a project and all its tasks                 |
| `scry --json project status list`                                                                                                                                     | List a project's statuses                          |
| `scry --json project status add <name>`                                                                                                                               | Add a status                                       |
| `scry --json project status remove <name>`                                                                                                                            | Remove an empty status                             |
| `scry --json project status rename <old> <new>`                                                                                                                       | Rename a status                                    |
| `scry --json project status move-up <name>`                                                                                                                           | Move a status up in the ordering                   |
| `scry --json project status move-down <name>`                                                                                                                         | Move a status down in the ordering                 |
| `scry --json project status set-style <name> <style>`                                                                                                                 | Set a status's style                               |

Project templates seed a project's statuses and settings via
`scry --json project create -t <template> <name>`:

- `todolist` - statuses `todo` and `done`; entry status `todo`.
- `kanban` - statuses `in-progress`, `backlog`, and `done`; entry status `backlog`; priority sorting.

Project descriptions are freeform text; set one at create time with `--description`, change it with
`scry --json project update --description <text>`, and clear it by passing an empty string. The same
project update command also renames the project with `--name`, sets or clears the entry status with
`--entry-status` or `--clear-entry-status`, changes the sort mode with `--sort`, and shows or hides
priority with `--show-priority` or `--hide-priority`.

Creating a project without `-t` leaves it with no statuses, so tasks cannot be added until you run
`scry --json project status add <name>`.

## Safety

- Do **not** run destructive commands (`scry delete`, `scry project delete`,
  `scry project status remove`) without explicit user confirmation.
- Never guess an ID for a destructive action; confirm it with `scry show <id>` first
  when there is any doubt.
- The active project is local to this machine, but changing it still alters the user's
  later commands; prefer `-p/--project` when operating on a specific project.
