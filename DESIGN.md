# scry — Design Document

A task manager for the terminal.

## Overview

scry is a CLI task manager that organizes work into **projects** with customizable **states** (kanban columns). Tasks live within a project and move between states. A built-in "default" project with `todo` / `done` states provides a simple out-of-box todo list experience — no setup required.

## Core Concepts

### Projects

A project is a container for related tasks. Each project has its own set of states. The built-in "default" project exists automatically and can be used without creating anything.

### States

States are the columns a task moves through within a project. Every project has at least one state. The "default" project comes with two states: `todo` and `done`. Custom projects can have any number of arbitrarily named states (e.g., "backlog", "in progress", "review", "done"). State names are case-sensitive.

### Active Project

scry uses a kubectl-style context model. `scry project use <name>` sets the active project, which persists across commands and shell sessions. All task commands operate on the active project unless overridden with the global `--project` / `-p` flag.

## CLI Reference

### Global Flags

These flags are available on every command.

| Flag | Description |
|------|-------------|
| `-p, --project <name>` | Target a specific project for this command, overriding the active project |

### Task Commands

#### `scry add <description>`

Add a new task to the active project. The task is created in the `todo` state.

**Arguments:**
| Arg | Description |
|-----|-------------|
| `<description>` | The task title / description |

**Output:**
```
✓ Created task #3 in "default" [todo]: Buy groceries
```

**Edge cases:**
- If no project exists yet, the "default" project is auto-created with `todo` / `done` states
- If the active project has been deleted, fall back to the "default" project

---

#### `scry list`

List tasks in the active project, grouped by state (kanban-column style). States are displayed in the order they were defined.

**Options:**
| Flag | Description |
|------|-------------|
| `--state <name>` | Show only tasks in the given state (can be repeated for multiple states) |

**Output (default project, all states):**
```
project "default" (*)

todo (2):
  #1  ☐ Buy groceries
  #2  ☐ Call dentist

done (1):
  #3  ☑ Ship the package
```

**Output (custom project, all states):**
```
project "myapp" (*)

backlog (2):
  #1  ☐ Add dark mode
  #2  ☐ Write docs

in progress (1):
  #3  ☐ Fix login bug

done (1):
  #4  ☑ Set up CI
```

**Output (filtered to one state, `--state todo`):**
```
project "default" (*)

todo (2):
  #1  ☐ Buy groceries
  #2  ☐ Call dentist
```

**Output (empty project):**
```
project "default" (*)

No tasks.
```

**Edge cases:**
- If no project exists, auto-create the "default" project
- Tasks without a recognizable state are listed under a fallback heading

---

#### `scry move <id> <state>`

Move a task to a new state. This is an alias for `scry update <id> --state <state>`.

**Arguments:**
| Arg | Description |
|-----|-------------|
| `<id>` | The task ID |
| `<state>` | The target state name |

**Output:**
```
✓ Moved task #3 → "done"
```

**Edge cases:**
- If the task ID does not exist in the project: error message with "Task #N not found"
- If the state does not exist in the project: error message listing available states
- If the task is already in the target state: succeed silently (no-op)

---

#### `scry update <id>`

Update task properties. This is the extensible hub for future task modifications.

**Arguments:**
| Arg | Description |
|-----|-------------|
| `<id>` | The task ID |

**Options (current):**
| Flag | Description |
|------|-------------|
| `--state <name>` | Move the task to a new state |

**Note:** Future flags may include `--title`, `--due`, `--priority`, etc. These are documented under Future Roadmap but are not yet implemented.

**Output (state change):**
```
✓ Updated task #3: state → "done"
```

**Edge cases:**
- Same as `scry move` edge cases
- If no flags are provided: print a help summary for the update command

---

#### `scry delete <id>`

Delete a task permanently.

**Arguments:**
| Arg | Description |
|-----|-------------|
| `<id>` | The task ID |

**Output:**
```
✓ Deleted task #3 from "default"
```

**Edge cases:**
- If the task ID does not exist: `✗ Task #3 not found in "default"`

---

#### `scry show <id>`

Show full details for a single task.

**Arguments:**
| Arg | Description |
|-----|-------------|
| `<id>` | The task ID |

**Output:**
```
Task #3
  Project:   default
  State:     todo
  Title:     Buy groceries
  Created:   2026-08-03 14:22:10 UTC
  Completed: —
```

If the task is in a "done" state (or any state configured as a terminal state — future feature), `Completed` shows the timestamp. Otherwise it shows `—`.

---

### Project Commands

#### `scry project list`

List all projects. The active project is marked with `*`.

**Output:**
```
  default (*)
  myapp
  side-project
```

**Edge cases:**
- If no projects exist: `No projects. Run 'scry project create <name>' to create one.`

---

#### `scry project create <name>`

Create a new project with default states (`todo`, `done`). The new project becomes the active project.

**Arguments:**
| Arg | Description |
|-----|-------------|
| `<name>` | The project name (must be unique) |

**Output:**
```
✓ Created project "myapp" with states: todo, done
→ Using project "myapp"
```

**Edge cases:**
- If a project with the same name already exists: `✗ Project "myapp" already exists`
- Project names must be non-empty and may not contain only whitespace

---

#### `scry project delete <name>`

Delete a project and all its tasks. Prompts for confirmation unless `--force` is provided. If the deleted project was the active project, the active project is reset to "default".

**Arguments:**
| Arg | Description |
|-----|-------------|
| `<name>` | The project name |

**Options:**
| Flag | Description |
|------|-------------|
| `--force, -f` | Skip confirmation prompt |

**Output:**
```
⚠ Delete project "myapp" and all 12 tasks? [y/N]: y
✓ Deleted project "myapp"
→ Using project "default"
```

With `--force`:
```
✓ Deleted project "myapp" (12 tasks)
→ Using project "default"
```

**Edge cases:**
- Cannot delete the "default" project: `✗ Cannot delete the default project`
- If the project does not exist: `✗ Project "myapp" not found`

---

#### `scry project use <name>`

Set the active project. Persisted across sessions.

**Arguments:**
| Arg | Description |
|-----|-------------|
| `<name>` | The project name |

**Output:**
```
✓ Using project "myapp"
```

**Edge cases:**
- If the project does not exist: `✗ Project "myapp" not found`

---

#### `scry project current`

Show the currently active project.

**Output:**
```
myapp
```

No edge cases — the "default" project always exists.

---

### State Commands

State commands operate on the active project by default (or the project specified via `-p`).

#### `scry project state list`

List all states for the active project, in their defined order.

**Output:**
```
States for "myapp":
  todo
  in progress
  review
  done
```

---

#### `scry project state add <name>`

Add a new state to the active project. The new state is appended after existing states.

**Arguments:**
| Arg | Description |
|-----|-------------|
| `<name>` | The state name (must be unique within the project) |

**Output:**
```
✓ Added state "review" to project "myapp"
```

**Edge cases:**
- If the state already exists in the project: `✗ State "review" already exists in "myapp"`
- State names must be non-empty

---

#### `scry project state remove <name>`

Remove a state from the active project. Fails with an error if tasks exist in that state (use `--force` to move those tasks to the first remaining state).

**Arguments:**
| Arg | Description |
|-----|-------------|
| `<name>` | The state name |

**Options:**
| Flag | Description |
|------|-------------|
| `--force, -f` | Move tasks in the removed state to the first state in the project |

**Output (no tasks in state):**
```
✓ Removed state "review" from project "myapp"
```

**Output (with `--force`, tasks moved):**
```
✓ Removed state "review" from project "myapp" (3 tasks moved to "todo")
```

**Edge cases:**
- Cannot remove the last remaining state: `✗ Cannot remove the last state of a project`
- If tasks exist in the state and `--force` is not used: `✗ State "review" has 3 tasks. Use --force to move them to "todo".`

## Error Conventions

All errors are printed to stderr and prefixed with `✗`. Success messages are printed to stdout and prefixed with `✓`. Warnings and prompts use `⚠`.

Task IDs are per-project (each project has its own auto-incrementing ID sequence). A task ID alone is not globally unique — it must be combined with the project context.

## Configuration

The active project setting is persisted across sessions. The exact storage mechanism is an implementation detail (database record, config file, etc.).

## Database

Tasks, projects, and states are stored in a SQLite database located at `$XDG_DATA_HOME/scry/scry.db` (falling back to `~/.local/share/scry/scry.db`). The database schema is managed automatically — no manual setup or migrations are required.

## Future Roadmap

Features under consideration for future versions:

- **Rich task fields:** due dates, priorities, tags/labels, long-form descriptions
- **Filtering and search:** `scry list --tag bug --priority high --due this-week`
- **Task editing:** `scry update <id> --title "new title" --due friday`
- **Terminal states:** mark certain states as "done" so completed_at is auto-recorded
- **Reordering:** manual task ordering within a state
- **Archiving:** archive completed tasks instead of deleting
- **JSON export/import:** `scry export` / `scry import` for backup and portability
- **Context-aware completion:** shell completions for project names, state names, and task IDs
