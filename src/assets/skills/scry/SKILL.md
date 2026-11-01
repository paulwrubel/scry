---
name: scry
description: Manage tasks, todos, and projects with the scry terminal task manager CLI. Use when the user asks to track work, add or complete todos, move tasks between sections such as backlog, in-progress, or review, organize tasks into projects, or inspect existing tasks and notes.
license: MIT
---

# scry

`scry` is a task manager for the terminal. Drive it through its CLI to manage tasks and projects on the user's behalf.

## When to use this skill

Use it when the user asks to:

- Track work or manage a todo list.
- Add, update, close, reopen, or delete tasks.
- Move tasks between sections (for example backlog, in-progress, or review).
- Organize work into projects, or inspect projects and their sections.
- Read a task's details or notes.

## Discover the exact surface

`scry` is built with `clap`, so every command and subcommand supports `--help`. You can
append `--help` to essentially any command to get accurate, current usage, its arguments,
and the accepted values of any enum option (shown as `[possible values: ...]`). For
example: `scry --help`, `scry add --help`, or `scry section set-color --help`.
Prefer this over guessing command names, flags, or values.

## Output for agents

- Always pass `--json` to task, project, and section commands. It returns compact,
  machine-readable JSON and disables color automatically, so never parse the
  human-readable text output.
- Enum values in JSON are kebab-case and match the CLI's accepted arguments, so a value
  read from JSON can be passed straight back to the CLI.
  - Examples: `scry --json list`, `scry --json show 4f3a`, `scry -p myapp --json add "design API"`.
- `--json` always emits full UUIDs, never handles. Store and pass full UUIDs for
  reliability: use the `id` field returned by `scry --json add` rather than a handle,
  because handles can lengthen as more tasks and notes are created.
- `scry --json show <handle>` includes full detail (description, priority, tags, timestamps,
  section, state, and notes), while `scry --json list` omits descriptions, notes, and
  timestamps. Use `scry --json show` when full detail is needed.
- `scry --json list` output is the stable machine format; never parse the human-readable
  text output without the `--json` flag. Closed tasks are hidden unless you pass
  `--include-closed`.

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
- The built-in `default` project exists automatically with no sections; tasks go to
  **No section** until you add sections. No setup is required.

## Core task commands

Every example below includes the global `--json` flag, which must come before the subcommand.

| Command                                                                                                                                          | Purpose                          |
| ------------------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------- |
| `scry --json add <title> [--description <text>] [--priority <level>] [--tags <a,b>] [--section <name>] [--new-section <name>]`                   | Add a task                       |
| `scry --json list [--section <name>] [--search <text>] [--include-closed]`                                                                       | List tasks grouped by section    |
| `scry --json show <handle>`                                                                                                                      | Show full task details and notes |
| `scry --json move <handle> [<section>] [--no-section]`                                                                                           | Move a task to a section         |
| `scry --json update <handle> [--title ...] [--description ...] [--priority ...] [--tags ...] [--section ...] [--new-section ...] [--no-section]` | Update task properties           |
| `scry --json close <handle>`                                                                                                                     | Close a task                     |
| `scry --json open <handle>`                                                                                                                      | Reopen a closed task             |
| `scry --json duplicate <handle>`                                                                                                                 | Duplicate a task                 |
| `scry --json note add <task-handle> <contents>`                                                                                                  | Add a note to a task             |
| `scry --json delete <handle>`                                                                                                                    | Delete a task permanently        |

`<level>` is one of `minimal`, `low`, `medium`, `high`, `critical` (aliases `p5`-`p1`).

Task and note arguments take a handle: the shortest unique prefix of that task's or note's UUID, at least 4 lowercase hex characters. Dashes are optional and, when present, must sit at the canonical UUID positions (after 8, 12, 16, or 20 hex characters). A full UUID always works.

Sections are created explicitly: `--section <name>` must name an existing section and fails
with the closest match suggested, while `--new-section <name>` (or `scry section add`)
creates one. New tasks go to No section unless one of those flags is given, and `--no-section`
on `move` or `update` moves a task back to No section. `close` and `open` change
only the state; they never change a task's section or position.

## Project and section commands

Every example below includes the global `--json` flag, which must come before the subcommand.

| Command                                                                                                              | Purpose                                            |
| -------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------- |
| `scry --json project list`                                                                                           | List projects (`*` marks the active one)           |
| `scry --json project current`                                                                                        | Show the active project                            |
| `scry --json project show [name]`                                                                                    | Show a project's details including its description |
| `scry --json project use <name>`                                                                                     | Set the active project                             |
| `scry --json project create [-t <template>] [--description <text>] <name>`                                           | Create a project                                   |
| `scry --json project update [--name <new>] [--description <text>] [--sort <mode>] [--show-priority/--hide-priority]` | Update project properties                          |
| `scry --json project delete <name> [-f]`                                                                             | Delete a project and all its tasks                 |
| `scry --json section list`                                                                                           | List a project's sections                          |
| `scry --json section add <name> [--color <color>]`                                                                   | Add a section                                      |
| `scry --json section remove <name>`                                                                                  | Delete a section, moving its tasks to No section   |
| `scry --json section rename <old> <new>`                                                                             | Rename a section                                   |
| `scry --json section merge <source> <target>`                                                                        | Merge the source into the target                   |
| `scry --json section move-up <name>`                                                                                 | Move a section up in the ordering                  |
| `scry --json section move-down <name>`                                                                               | Move a section down in the ordering                |
| `scry --json section set-color <name> <color>`                                                                       | Set a section's color                              |
| `scry --json section reset-color <name>`                                                                             | Clear a section's color                            |

Project templates seed a project's sections and settings via
`scry --json project create -t <template> <name>`:

- `todolist` - no sections; manual sorting.
- `kanban` - sections `backlog`, `in-progress`, and `review`; priority sorting.

Project descriptions are freeform text; set one at create time with `--description`, change
it with `scry --json project update --description <text>`, and clear it by passing an empty
string. The same project update command also renames the project with `--name`, changes the
sort mode with `--sort`, and shows or hides priority with `--show-priority` or
`--hide-priority`. The sort modes are `alphabetical`, `alphabetical-case-insensitive`,
`created` (creation order, oldest first; `id` is accepted as an alias), `manual`, and
`priority`.

Creating a project without `-t` leaves it with no sections. Tasks still work and land in No
section; add sections later with `scry --json section add <name>`.

## Safety

- Do **not** run destructive commands (`scry delete`, `scry project delete`,
  `scry section remove`) without explicit user confirmation.
- Never guess a handle for a destructive action; confirm it with `scry show <handle>` first
  when there is any doubt.
- `scry close` closes a task but keeps it; `scry delete` removes it permanently.
- The active project is local to this machine, but changing it still alters the user's
  later commands; prefer `-p/--project` when operating on a specific project.
