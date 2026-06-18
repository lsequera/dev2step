# Dev2Step Design Specification

*   **Date:** 2026-06-17
*   **Version:** v1.0
*   **Status:** Approved
*   **Target Stack:** Rust, Tauri v2, SQLite, Vanilla TypeScript, CSS, Vite

---

## 1. Architectural Philosophy & Core Concepts

Dev2Step is a high-performance, local-first "Scrumban" task board optimized for solo developers. It runs as a Tauri desktop app and integrates a command-line interface (CLI) to ensure zero friction in task logging and updates.

### 1.1 Core Principles
*   **Zero Friction:** Transitioning state or logging a task takes less than 2 seconds via CLI, global hotkeys, or direct file edits.
*   **Local-First / Plain-Text:** Data is stored in a standard `todo.txt` file as the Single Source of Truth (SSOT).
*   **Developer Dashboard:** A companion GUI built with Tauri displays a 4-column board (Icebox, Todo, In Progress, Done) with analytics metrics.

---

## 2. Data Architecture

### 2.1 Extended `todo.txt` Format Specification
Dev2Step extends the `todo.txt` standard with specific metadata tags. A complete task line is structured as:
```text
[completed_flag] [priority] [completion_date] [creation_date] Description +project status:STATUS due:YYYY-MM-DD id:ID parent:ID
```

*   `x `: Task completed indicator.
*   `(A) `: Priority character mapping (A, B, C).
*   `2026-06-17`: Date strings formatted as `YYYY-MM-DD`.
*   `+project_name`: Project tag (starts with `+`).
*   `status:<icebox|todo|progress|done>`: State machine column indicator.
*   `id:<integer>`: Unique identifier for synchronization.
*   `parent:<integer>`: Sub-task association ID.

### 2.2 Relational SQLite Schema (`dev2step.db`)
SQLite serves as a query cache and logs status transition history.

```sql
CREATE TABLE IF NOT EXISTS projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS tasks (
    id INTEGER PRIMARY KEY,
    priority TEXT CHECK(priority IN ('A', 'B', 'C', NULL)),
    is_completed INTEGER DEFAULT 0 CHECK(is_completed IN (0, 1)),
    completion_date TEXT,
    creation_date TEXT,
    description TEXT NOT NULL,
    project_id TEXT,
    status TEXT CHECK(status IN ('icebox', 'todo', 'progress', 'done')) DEFAULT 'icebox',
    due_date TEXT,
    parent_id INTEGER,
    line_number INTEGER NOT NULL,
    FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE SET NULL,
    FOREIGN KEY(parent_id) REFERENCES tasks(id) ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS state_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id INTEGER NOT NULL,
    from_status TEXT,
    to_status TEXT,
    changed_at TEXT DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(task_id) REFERENCES tasks(id) ON DELETE CASCADE
);
```

---

## 3. Rust Backend & Sync Engine

### 3.1 Path Resolution
1.  **Walk Up Repository:** Walk up parent directories starting from the current working directory to find an existing `todo.txt` or `.git` directory.
2.  **Global Fallback:** Fallback to `~/.dev2step/todo.txt` and `~/.dev2step/dev2step.db`.

### 3.2 Bidirectional Sync Logic (Approach A)
*   **Startup/Manual Sync:** The Rust backend reads `todo.txt` line-by-line, parses each task, and reconciles the records in the SQLite database (Inserting new, updating modified, and deleting removed tasks).
*   **File Watcher:** The Rust `notify` crate monitors `todo.txt` for external edits. When modified, changes are debounced (100ms) and mapped to the database. Updates to task statuses trigger logs in `state_history`.
*   **Self-Write Bypass:** File-writing calls originating from Dev2Step's own CLI or GUI set an atomic boolean flag (`ignoring_watcher = true`) to prevent circular triggers.

### 3.3 Command-Line Interface (`dev2step`)
*   `dev2step status`: Prints board state to stdout using formatted ASCII.
*   `dev2step add "<desc>"`: Appends task with incremental ID. Supports `--project`, `--priority`, `--todo`, `--progress`.
*   `dev2step transition --id <id> --to <status>`: Rewrites the specified line in `todo.txt` with the new status.
*   `dev2step complete --id <id>`: Prefixes with `x` and marks status as `done`.
*   `dev2step remove --id <id>`: Deletes the line from `todo.txt`.

---

## 4. Frontend UI/UX Design

### 4.1 Aesthetics
*   **Theme:** Glassmorphic slate dark mode (`#0a0b10` background, `#161825` panels, `#f1f3f9` text).
*   **WIP Indicators:** Header highlighting triggers soft warning glow (amber HSL) when WIP limits (Todo: 5, In Progress: 2) are exceeded.
*   **Breathing Animation:** Active tasks in the "In Progress" column pulse with a soft purple shadow.

### 4.2 Keyboard Controls
*   `N` or `/`: Opens Omnibar (with smart badges for `+project`, `(A)` priority, etc.).
*   `Arrow Keys` / `Tab`: Focus navigation.
*   `1`, `2`, `3`, `4`: Moves selected task to columns 1-4.
*   `Ctrl + M`: Toggles the analytics metrics slide-out panel.
