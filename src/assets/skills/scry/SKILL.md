---
name: scry
description: Manage tasks, todos, projects, and sections with the scry CLI; use to track, move, or inspect tasks.
license: MIT
---

# scry

`scry` is a task manager driven through its CLI. Commands talk to a configured scry server.

## Essentials

- Put the global `--json` flag before the subcommand: `scry --json list`. It prints compact JSON with
  color disabled, so never parse the text output.
- Every command supports `--help`; use it to confirm names, flags, and enum values instead of guessing.
- Under `--json`, failures exit non-zero and print `{"error":{"kind":"<kind>","message":"<message>"}}` to stderr.
  `kind` is a stable snake_case code such as `not_found` or `invalid`.
- Task JSON objects include a full UUID `id`. `show` and `add` also return a display `handle`, but handles
  can change as ids are added, so store and pass `id`.
- Prefer `-p/--project <name>` when targeting a specific project. Otherwise the machine's active project
  applies, falling back to `default`.

## Task commands

| Command                                                                                                                                              | Purpose                    |
| ---------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------- |
| `scry --json add <title> [--description <text>] [--priority <level>] [--tags <a,b>] [--section <name>] [--new-section <name>]`                       | Add a task                 |
| `scry --json list [--section <name>] [--search <text>] [--include-closed]`                                                                           | List tasks                 |
| `scry --json show <id\|handle>`                                                                                                                      | Show a task with its notes |
| `scry --json move <id\|handle> [<section>] [--no-section] [--top\|--first\|--bottom\|--last\|--before <id>\|--after <id>]`                           | Move or place a task       |
| `scry --json update <id\|handle> [--title ...] [--description ...] [--priority ...] [--tags ...] [--section ...] [--new-section ...] [--no-section]` | Update task properties     |
| `scry --json close <id\|handle>...`                                                                                                                  | Close tasks                |
| `scry --json open <id\|handle>...`                                                                                                                   | Reopen tasks               |
| `scry --json duplicate <id\|handle>`                                                                                                                 | Duplicate a task           |
| `scry --json note add <id\|handle> <contents>`                                                                                                       | Add a note to a task       |
| `scry --json delete <id\|handle>`                                                                                                                    | Delete a task permanently  |

Task arguments accept a full UUID or a unique handle prefix of at least 4 hex characters
(case-insensitive, dashes optional at the canonical UUID positions).

Priority is one of `minimal`, `low`, `medium`, `high`, `critical` (aliases `p5`-`p1`). An empty
`--description` or `--tags` on `update` clears the field.

`list` returns a flat JSON array of task objects; use `section_id` to associate tasks with sections. It includes
no notes or handles, and closed tasks are hidden unless you pass `--include-closed` (they follow the open tasks).
The array follows section order but is not grouped.

Sections are optional. `--section` must match an existing section name exactly, while `--new-section` creates
one; without either, the task goes to No section. `--no-section` clears a task's section. `move` appends by
default; add `--top`/`--first`, `--bottom`/`--last`, `--before <id>`, or `--after <id>` to place it instead.
Order is only visible when the project uses `manual` sorting. `close` and `open` never move a task.

## Project and section commands

| Command                                                                                                               | Purpose                                        |
| --------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------- |
| `scry --json project list`                                                                                            | List projects                                  |
| `scry --json project current`                                                                                         | Show the active project                        |
| `scry --json project show [name]`                                                                                     | Show project details                           |
| `scry --json project use <name>`                                                                                      | Set the active project                         |
| `scry --json project create [-t <template>] [--description <text>] <name>`                                            | Create a project                               |
| `scry --json project update [--name <new>] [--description <text>] [--sort <mode>] [--show-priority\|--hide-priority]` | Update project properties                      |
| `scry --json project delete <name> [-f]`                                                                              | Delete a project and its tasks                 |
| `scry --json section list`                                                                                            | List a project's sections                      |
| `scry --json section add <name> [--color <color>]`                                                                    | Add a section                                  |
| `scry --json section move <name> (--top\|--bottom\|--before <name>\|--after <name>)`                                  | Reorder a section                              |
| `scry --json section remove <name>`                                                                                   | Delete a section; its tasks move to No section |
| `scry --json section rename <old> <new>`                                                                              | Rename a section                               |
| `scry --json section merge <source> <target>`                                                                         | Merge source into target                       |
| `scry --json section set-color <name> <color>`                                                                        | Set a section color                            |
| `scry --json section reset-color <name>`                                                                              | Clear a section color                          |

`section move` requires exactly one placement flag: `--top` (alias `--first`), `--bottom` (alias
`--last`), `--before <name>`, or `--after <name>`.

`project create -t <template>` seeds sections and settings: `todolist` has none and uses manual sorting;
`kanban` has `backlog`, `in-progress`, and `review` and uses priority sorting. Without `-t` a project starts
with no sections. `project update --sort` accepts `alphabetical`, `alphabetical-case-insensitive`, `created`,
`manual`, or `priority`. An empty `--description` clears it.

## Snapshots

`scry export` prints a JSON snapshot; put `-p <name>` before `export` for a single project. `scry import`
reads a snapshot from stdin.

## Safety

- Do not run `scry delete`, `scry project delete`, or `scry section remove` without explicit user confirmation.
- Never guess an id for a destructive action; confirm it with `scry --json show <id>` first when in doubt.
- `scry close` keeps the task; `scry delete` removes it permanently.
- Changing the active project affects the user's later commands on this machine; prefer `-p/--project`.
