# Dev2Step — Technical Documentation

> Architecture reference, data-flow diagrams, module descriptions, and extension guide.

---

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [Data Flow](#2-data-flow)
3. [Module Reference](#3-module-reference)
4. [Database Schema](#4-database-schema)
5. [todo.txt Format Specification](#5-todotxt-format-specification)
6. [Tauri IPC Commands](#6-tauri-ipc-commands)
7. [Frontend Architecture](#7-frontend-architecture)
8. [File Watcher & Debounce](#8-file-watcher--debounce)
9. [Path Resolution Algorithm](#9-path-resolution-algorithm)
10. [CLI Reference](#10-cli-reference)
11. [Extending Dev2Step](#11-extending-dev2step)

---

## 1. Architecture Overview

Dev2Step uses a **text-first reactive sync** model:

```
┌──────────────────────────────────────────────────┐
│                  todo.txt  (SSOT)                │
│          (plain text, human-editable)            │
└───────────────────┬──────────────────────────────┘
                    │  read / write
         ┌──────────▼───────────┐
         │   sync.rs            │  load_file_and_sync()
         │   (Sync Engine)      │  write_tasks_to_file()
         └──────┬───────────────┘
                │  upsert / delete
     ┌──────────▼──────────┐
     │   dev2step.db        │  SQLite (cache + history)
     │   (rusqlite)         │
     └──────────────────────┘
                │
      ┌─────────▼──────────┐          ┌──────────────────┐
      │  Tauri Commands     │◄────────►│  Frontend UI     │
      │  (main.rs)          │  IPC     │  (TypeScript)    │
      └─────────────────────┘          └──────────────────┘
                                              ▲
                                    todo-updated event
                                              │
                              ┌───────────────┴────────┐
                              │  watcher.rs             │
                              │  (notify file watcher)  │
                              └─────────────────────────┘
```

**Key principle:** `todo.txt` is the Single Source of Truth. SQLite is a derived cache and should be considered disposable — deleting `dev2step.db` causes a clean rebuild on next launch.

---

## 2. Data Flow

### 2a. App startup

```
main() 
  └─ cli::handle_cli()          # exits early if CLI args present
  └─ resolve_paths()            # locate todo.txt and dev2step.db
  └─ init_db(&db_path)          # CREATE TABLE IF NOT EXISTS
  └─ load_file_and_sync()       # parse todo.txt → upsert SQLite
  └─ start_file_watcher()       # spawn notify thread
  └─ tauri::Builder::run()      # enter GUI event loop
```

### 2b. External edit (user edits `todo.txt` in another editor)

```
notify event (Modify)
  └─ LAST_WRITE_TIME check      # skip if self-written within 500 ms
  └─ sleep(150 ms)              # debounce
  └─ load_file_and_sync()       # re-parse → upsert SQLite
  └─ app.emit("todo-updated")   # push Task[] to frontend via IPC
```

### 2c. UI action (e.g. move card to Progress)

```
Frontend invoke("update_task_status", {id, status})
  └─ LAST_WRITE_TIME = now      # arm debounce guard
  └─ load_file_and_sync()       # get current state
  └─ mutate task in memory
  └─ write_tasks_to_file()      # write back to todo.txt
  └─ load_file_and_sync()       # re-sync → returns updated Task[]
  └─ return Task[] to frontend
```

---

## 3. Module Reference

### `src-tauri/src/todo_parser.rs`

Parses and formats the extended todo.txt format.

| Symbol | Signature | Description |
|---|---|---|
| `DevTask` | `struct` | Full task model — id, priority, status, dates, project, parent |
| `TaskStatus` | `enum` | `Icebox \| Todo \| Progress \| Done` |
| `parse_line` | `fn(line: &str, line_num: usize) -> Result<DevTask, String>` | Parse one todo.txt line |
| `format_task` | `fn(task: &DevTask) -> String` | Serialize a `DevTask` back to todo.txt line |

### `src-tauri/src/path_resolver.rs`

| Symbol | Signature | Description |
|---|---|---|
| `resolve_paths` | `fn() -> (PathBuf, PathBuf)` | Returns `(todo_path, db_path)` |

Walks up from `cwd` looking for `.git/` or `todo.txt`. Falls back to `~/.dev2step/`.

### `src-tauri/src/db.rs`

| Symbol | Signature | Description |
|---|---|---|
| `init_db` | `fn(db_path: &Path) -> Result<Connection, String>` | Open/create SQLite DB and run schema migrations |
| `sync_tasks_to_db` | `fn(conn: &Connection, tasks: &[DevTask]) -> Result<(), String>` | Full upsert: inserts/updates all tasks, deletes removed ones, logs status transitions |

### `src-tauri/src/sync.rs`

| Symbol | Signature | Description |
|---|---|---|
| `load_file_and_sync` | `fn(todo_path: &Path, conn: &Connection) -> Result<Vec<DevTask>, String>` | Read `todo.txt`, assign missing IDs, rewrite if needed, sync to DB |
| `write_tasks_to_file` | `fn(todo_path: &Path, tasks: &[DevTask]) -> Result<(), String>` | Serialize `Vec<DevTask>` back to `todo.txt` |

### `src-tauri/src/watcher.rs`

| Symbol | Signature | Description |
|---|---|---|
| `LAST_WRITE_TIME` | `Mutex<Option<Instant>>` | Guards against self-write loops |
| `start_file_watcher` | `fn<R: Runtime>(app_handle, todo_path, db_path)` | Spawns a `notify` watcher on the parent directory of `todo.txt` |

### `src-tauri/src/cli.rs`

| Symbol | Signature | Description |
|---|---|---|
| `handle_cli` | `fn() -> Result<bool, String>` | Returns `Ok(true)` if a CLI subcommand was handled (main skips GUI), `Ok(false)` to launch GUI |

### `src-tauri/src/main.rs`

Defines `AppState`, all `#[tauri::command]` handlers, and the `main()` entry point.

---

## 4. Database Schema

```sql
CREATE TABLE projects (
    id   TEXT PRIMARY KEY,       -- same as project tag, e.g. "Backend"
    name TEXT NOT NULL,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE tasks (
    id              INTEGER PRIMARY KEY,
    priority        TEXT,              -- single char, e.g. "A"
    is_completed    INTEGER DEFAULT 0, -- 0 | 1
    completion_date TEXT,              -- YYYY-MM-DD or NULL
    creation_date   TEXT,              -- YYYY-MM-DD
    description     TEXT NOT NULL,
    project_id      TEXT REFERENCES projects(id) ON DELETE SET NULL,
    status          TEXT,              -- "icebox" | "todo" | "progress" | "done"
    due_date        TEXT,              -- YYYY-MM-DD or NULL
    parent_id       INTEGER REFERENCES tasks(id) DEFERRABLE INITIALLY DEFERRED ON DELETE SET NULL,
    line_number     INTEGER NOT NULL
);

CREATE TABLE state_history (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id     INTEGER NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    from_status TEXT,    -- NULL for initial insertion
    to_status   TEXT,
    changed_at  TEXT DEFAULT CURRENT_TIMESTAMP
);
```

> **Note:** `PRAGMA foreign_keys = ON` is set on every connection. The `parent_id` self-reference uses `DEFERRABLE INITIALLY DEFERRED` to allow parent and child rows to be inserted within the same transaction regardless of order.

---

## 5. todo.txt Format Specification

Dev2Step extends the [todo.txt standard](http://todotxt.org/) with four extra key-value pairs.

### Grammar

```
line     ::= [completed] [priority] [comp_date] [cre_date] description tags
completed::= "x "
priority ::= "(" UPPERCASE_LETTER ") "
comp_date::= YYYY-MM-DD " "    (only valid when `completed` is present)
cre_date ::= YYYY-MM-DD " "
tags     ::= ("+project" | "status:STATUS" | "due:DATE" | "id:INT" | "parent:INT")*
STATUS   ::= icebox | todo | progress | done
```

### Examples

```
# Active task with priority, project, and status
(A) 2026-06-17 Implement OAuth +Backend status:progress due:2026-06-30 id:1

# Completed task
x 2026-06-18 2026-06-15 Write unit tests +Backend status:done id:2

# Sub-task linked to task #1
2026-06-17 Add refresh token logic +Backend status:icebox id:3 parent:1
```

### ID assignment

If a line is missing `id:N`, Dev2Step assigns the next available integer on first load and **rewrites the file** to persist the ID. This ensures IDs are stable once assigned.

---

## 6. Tauri IPC Commands

All commands are invoked from the frontend via `invoke(commandName, args)` and return a `Promise`.

### `get_tasks() → Task[]`
Returns all tasks by loading and syncing `todo.txt`.

### `add_task(description, project?, priority?, status) → Task[]`
Creates a new task and returns the updated task list.

| Param | Type | Notes |
|---|---|---|
| `description` | `string` | Task text |
| `project` | `string \| null` | Project tag (without `+`) |
| `priority` | `string \| null` | Single uppercase letter |
| `status` | `string` | One of `icebox`, `todo`, `progress`, `done` |

### `update_task_status(id, status) → Task[]`
Moves a task to a new status. Automatically sets `is_completed` and `completion_date` when transitioning to `done`, and clears them otherwise.

### `delete_task(id) → Task[]`
Removes a task by ID and returns the updated list.

### `get_velocity_metrics() → Metrics`

```typescript
interface Metrics {
  status_counts: [string, number][];  // e.g. [["todo", 3], ["progress", 1]]
  avg_cycle_days: number;             // avg days from "progress" → "done"
}
```

### IPC Event: `todo-updated`

Emitted by the file watcher when `todo.txt` is modified externally. Payload is `Task[]`.

```typescript
await listen<Task[]>("todo-updated", (event) => {
  updateDOM(event.payload);
});
```

---

## 7. Frontend Architecture

The entire frontend is a single-module TypeScript file with no framework dependency.

```
src/main.ts
│
├── init()                  DOMContentLoaded entry; sets up listeners + IPC event
├── refreshState()          invoke("get_tasks") → updateDOM
├── updateDOM(tasks)        allTasks = tasks → renderBoard + updateMetrics + refreshFocus
│
├── renderBoard()           Clears columns, renders task cards, enforces WIP limits
├── setupCardClicks()       Click-to-focus card selection
├── refreshFocus()          Applies .focused class + scrolls to active card
│
├── updateMetrics()         invoke("get_velocity_metrics") → update sidebar DOM
├── setupEventListeners()   Omnibar, drag-and-drop, keyboard shortcuts
│
├── handleAddTask()         Parses omnibar inline tags → invoke("add_task")
├── transitionTask()        invoke("update_task_status", {id, status})
└── escapeHtml()            XSS-safe DOM text insertion
```

### Keyboard focus model

`focusedIndex` tracks the active task within the flat `visibleTasks` array (ordered as tasks appear in the rendered board, left-to-right, top-to-bottom within each column). Arrow keys and Tab cycle through this array; number keys `1–4` act on the focused task.

---

## 8. File Watcher & Debounce

The watcher uses `notify::RecommendedWatcher` on the **parent directory** of `todo.txt` (non-recursive) to detect `Modify` events.

### Self-write loop prevention

Without a guard, writing `todo.txt` from the app would trigger the watcher, which would re-read the file and emit `todo-updated`, creating an infinite loop.

**Solution:** `LAST_WRITE_TIME` (`Mutex<Option<Instant>>`) is set to `Instant::now()` immediately before any `write_tasks_to_file()` call from Tauri commands. The watcher skips the event if `LAST_WRITE_TIME` is within **500 ms** of the event.

```
Tauri command writes todo.txt
  └─ LAST_WRITE_TIME = now      ← guard armed
  └─ write_tasks_to_file()

  ... 50 ms later ...

notify fires Modify event
  └─ elapsed since LAST_WRITE_TIME < 500 ms → SKIP
```

### Debounce

A `sleep(150 ms)` inside the notify callback ensures that rapid successive saves (e.g., editor auto-save + format) are collapsed into a single sync cycle.

---

## 9. Path Resolution Algorithm

```
current_dir = env::current_dir()
cursor = current_dir

loop:
  if cursor/todo.txt exists OR cursor/.git exists:
    return (cursor/todo.txt, cursor/dev2step.db)
  cursor = cursor.parent()
  if no parent: break

# Fallback
return (~/.dev2step/todo.txt, ~/.dev2step/dev2step.db)
```

This means you can run `dev2step` from any subdirectory of a project and it will locate the correct files, just like `git` resolves the repository root.

---

## 10. CLI Reference

```
dev2step <SUBCOMMAND>

SUBCOMMANDS:
  status                Show all tasks grouped by status column
  add <DESCRIPTION>     Add a new task
    --project <NAME>    Assign a project tag
    --priority <CHAR>   Set priority letter (A–Z)
    --todo              Start in Todo column (default: Icebox)
    --progress          Start in Progress column
  transition            Move a task to a different status
    --id <ID>           Task ID
    --to <STATUS>       icebox | todo | progress | done
  complete --id <ID>    Mark a task as completed (Done)
  remove   --id <ID>    Permanently delete a task
```

Exit codes follow POSIX conventions: `0` on success, non-zero on error. `--help` and `--version` print to stdout and exit `0`.

---

## 11. Extending Dev2Step

### Adding a new Tauri command

1. Write a `#[tauri::command]` async function in `src-tauri/src/main.rs`
2. Add it to the `tauri::generate_handler![]` macro call
3. Call it from the frontend with `invoke("your_command_name", args)`

### Adding a new todo.txt tag

1. Add a field to `DevTask` in `todo_parser.rs`
2. Parse it in the `for word in parts` loop of `parse_line()`
3. Serialize it in `format_task()`
4. Add the column to the `tasks` table schema in `db.rs` → `init_db()`
5. Upsert/read it in `sync_tasks_to_db()`

### Adding a new board column (status)

1. Add the variant to `TaskStatus` in `todo_parser.rs` and update `from_str()` / `to_str()`
2. Add the column `<div>` to `index.html`
3. Add it to the `cols` map in `renderBoard()` in `main.ts`
4. Update CLI `status` display and `transition` validation in `cli.rs`
