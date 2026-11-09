---
name: scry
description: Manage tasks, todos, and projects with the scry CLI. Use when the user asks to track work, add or complete todos, move tasks between sections such as backlog, in-progress, or review, organize tasks into projects, or inspect existing tasks and notes.
license: MIT
---

# scry

`scry` is a client/server task manager with a CLI and a web UI; this skill drives it through the CLI to manage tasks and projects on the user's behalf.

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
- Enum values accepted by the CLI (priority, sort mode, and color) appear in JSON in the
  same kebab-case spelling, so a value read from JSON can be passed straight back to the
  CLI.
  - Examples: `scry --json list`, `scry --json show 4f3a`, `scry -p myapp --json add "design API"`.
- Every task object in `--json` output includes its full UUID `id`. `show`, `add`, and
  `duplicate` also return a `handle`; store and pass the `id` for reliability, because
  handles can lengthen as more tasks and notes are created.
- `scry --json show <handle>` returns the full task (description, priority, tags,
  `section_id`, state, and timestamps), its notes, and a `handle`. `scry --json list`
  returns a flat array of full task objects (description and `created_at` included) but no
  notes or handles; use `section_id` to associate tasks with sections. Use `show` when you
  need notes or the handle.
- `scry --json list` output is the stable machine format; never parse the human-readable
  text output without the `--json` flag. Closed tasks are hidden unless you pass
  `--include-closed`; when included, they follow the open tasks. The array already follows
  section order, but it is not grouped into per-section objects.

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
| `scry --json move <handle> [<section>] [--no-section] [--top\|--first\|--bottom\|--last\|--before <handle>\|--after <handle>]`                   | Move or place a task             |
| `scry --json update <handle> [--title ...] [--description ...] [--priority ...] [--tags ...] [--section ...] [--new-section ...] [--no-section]` | Update task properties           |
| `scry --json close <handle>...`                                                                                                                  | Close one or more tasks          |
| `scry --json open <handle>...`                                                                                                                   | Reopen one or more closed tasks  |
| `scry --json duplicate <handle>`                                                                                                                 | Duplicate a task                 |
| `scry --json note add <task-handle> <contents>`                                                                                                  | Add a note to a task             |
| `scry --json delete <handle>`                                                                                                                    | Delete a task permanently        |

`<level>` is one of `minimal`, `low`, `medium`, `high`, `critical` (aliases `p5`-`p1`).

Task arguments take a handle: any unique prefix of the task's UUID, at least 4 hex
characters. Uppercase is accepted and normalized to lowercase, and dashes are optional but
must sit at the canonical UUID positions (after 8, 12, 16, or 20 hex characters). A full
UUID always works. `note add` takes a task handle, not a note handle.

Sections are created explicitly: `--section <name>` must name an existing section and fails
with a not-found error naming the section, while `--new-section <name>` (or
`scry section add`) creates one. New tasks go to No section unless one of those flags is
given, and `--no-section` on `move` or `update` moves a task back to No section. `move`
appends by default when given a section or `--no-section`. Add `--top`/`--first`,
`--bottom`/`--last`, `--before <handle>`, or `--after <handle>` to place the task instead.
The placement flags work with a destination section, with `--no-section`, or alone to stay
in the current group; anchors must be another task in that group. The REST equivalent is
`POST /api/v1/tasks/{handle}/move` with `{"placement":"top"}`, `{"placement":"bottom"}`,
`{"placement":"before","anchor_id":"<task-id>"}`, or
`{"placement":"after","anchor_id":"<task-id>"}`. Omit `section_id` to keep the current
section, use `null` for No section, or pass a section id. Positions are always written, but
task order is only visible when the project uses `manual` sorting. `close` and `open`
change only the state; they never change a task's section or position.

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
| `scry --json section move <name> [--top\|--first\|--bottom\|--last\|--before <name>\|--after <name>]`                | Move a section within the project order            |
| `scry --json section remove <name>`                                                                                  | Delete a section, moving its tasks to No section   |
| `scry --json section rename <old> <new>`                                                                             | Rename a section                                   |
| `scry --json section merge <source> <target>`                                                                        | Merge the source into the target                   |
| `scry --json section set-color <name> <color>`                                                                       | Set a section's color                              |
| `scry --json section reset-color <name>`                                                                             | Clear a section's color                            |

`scry section move` reorders sections: `--top` (alias `--first`) places the section first,
`--bottom` (alias `--last`) places it last, and `--before <name>` or `--after <name>` places
it next to another section in the same project. The REST equivalent is
`POST /api/v1/sections/{id}/move` with `{"placement":"top"}`, `{"placement":"bottom"}`,
`{"placement":"before","anchor_id":"<id>"}`, or `{"placement":"after","anchor_id":"<id>"}`.
The task `move` command has the same placement flags; see Core task commands above.

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
