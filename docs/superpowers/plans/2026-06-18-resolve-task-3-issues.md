# Task 3 Review Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Resolve database transaction, foreign key constraint, and pragma issues in `src-tauri/src/db.rs`.

**Architecture:** 
1. Enable SQLite foreign key support immediately after opening the connection in `init_db`.
2. Defer task parent-child self-reference constraint validation to commit time by adding `DEFERRABLE INITIALLY DEFERRED` to the parent_id foreign key constraint.
3. Refactor `sync_tasks_to_db` to use a rusqlite transaction wrapper (`conn.transaction()`) instead of manual SQL transaction control, ensuring auto-rollback on function failures.

**Tech Stack:** Rust (Tauri, rusqlite)

## Global Constraints
* Rust Edition: Rust 2021
* rusqlite version: 0.31.0 (with bundled feature)

---

## Tasks

### Task 1: Refactor DB Initialization
**Files:**
* Modify: `src-tauri/src/db.rs`

**Interfaces:**
* Produces: `init_db` that enforces foreign keys and uses deferred constraints for parent task relationships.

- [ ] **Step 1: Modify `init_db` to execute PRAGMA and defer parent_id foreign key**
    Edit `src-tauri/src/db.rs`:
    * Add `conn.execute("PRAGMA foreign_keys = ON;", []).map_err(|e| e.to_string())?;` directly after opening the connection.
    * Modify the `FOREIGN KEY(parent_id) REFERENCES tasks(id)` definition in the table creation SQL to include `DEFERRABLE INITIALLY DEFERRED`.
    
    Expected modification in `init_db`:
    ```rust
    pub fn init_db(db_path: &Path) -> Result<Connection, String> {
        let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
        
        conn.execute("PRAGMA foreign_keys = ON;", []).map_err(|e| e.to_string())?;
        
        conn.execute(
            "CREATE TABLE IF NOT EXISTS projects (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP
            );",
            [],
        ).map_err(|e| e.to_string())?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS tasks (
                id INTEGER PRIMARY KEY,
                priority TEXT,
                is_completed INTEGER DEFAULT 0,
                completion_date TEXT,
                creation_date TEXT,
                description TEXT NOT NULL,
                project_id TEXT,
                status TEXT,
                due_date TEXT,
                parent_id INTEGER,
                line_number INTEGER NOT NULL,
                FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE SET NULL,
                FOREIGN KEY(parent_id) REFERENCES tasks(id) ON DELETE SET NULL DEFERRABLE INITIALLY DEFERRED
            );",
            [],
        ).map_err(|e| e.to_string())?;
        // ...
    ```

---

### Task 2: Refactor Sync Tasks Transaction Management
**Files:**
* Modify: `src-tauri/src/db.rs`

**Interfaces:**
* Produces: `pub fn sync_tasks_to_db(conn: &mut Connection, tasks: &[DevTask]) -> Result<(), String>`

- [ ] **Step 1: Modify `sync_tasks_to_db` signature and transactional logic**
    Edit `src-tauri/src/db.rs` to change `conn: &Connection` to `conn: &mut Connection`, create a transaction `tx`, and execute all sync queries against `tx`. Commit at the end.
    
    Expected code:
    ```rust
    pub fn sync_tasks_to_db(conn: &mut Connection, tasks: &[DevTask]) -> Result<(), String> {
        let mut tx = conn.transaction().map_err(|e| e.to_string())?;

        // Collect existing statuses before sync to detect transitions
        let mut stmt = tx.prepare("SELECT id, status FROM tasks").map_err(|e| e.to_string())?;
        let existing_statuses: std::collections::HashMap<u32, String> = stmt
            .query_map([], |row| Ok((row.get::<_, u32>(0)?, row.get::<_, String>(1)?)))
            .map_err(|e| e.to_string())?
            .filter_map(Result::ok)
            .collect();

        // 1. Upsert projects
        for task in tasks {
            if let Some(ref p) = task.project {
                tx.execute(
                    "INSERT OR IGNORE INTO projects (id, name) VALUES (?1, ?2)",
                    params![p, p],
                ).map_err(|e| e.to_string())?;
            }
        }

        // 2. Sync tasks
        let mut active_ids = Vec::new();
        for task in tasks {
            active_ids.push(task.id);
            let status_str = task.status.to_str().to_string();

            tx.execute(
                "INSERT INTO tasks (id, priority, is_completed, completion_date, creation_date, description, project_id, status, due_date, parent_id, line_number)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                 ON CONFLICT(id) DO UPDATE SET
                     priority = excluded.priority,
                     is_completed = excluded.is_completed,
                     completion_date = excluded.completion_date,
                     creation_date = excluded.creation_date,
                     description = excluded.description,
                     project_id = excluded.project_id,
                     status = excluded.status,
                     due_date = excluded.due_date,
                     parent_id = excluded.parent_id,
                     line_number = excluded.line_number",
                params![
                    task.id,
                    task.priority.map(|c| c.to_string()),
                    if task.is_completed { 1 } else { 0 },
                    task.completion_date,
                    task.creation_date,
                    task.description,
                    task.project,
                    status_str,
                    task.due_date,
                    task.parent_id,
                    task.line_number
                ],
            ).map_err(|e| e.to_string())?;

            // Check status change for history logging after task exists to satisfy FOREIGN KEY constraint
            if let Some(prev_status) = existing_statuses.get(&task.id) {
                if prev_status != &status_str {
                    tx.execute(
                        "INSERT INTO state_history (task_id, from_status, to_status) VALUES (?1, ?2, ?3)",
                        params![task.id, prev_status, status_str],
                    ).map_err(|e| e.to_string())?;
                }
            } else {
                // New task insertion logs initial status transition
                tx.execute(
                    "INSERT INTO state_history (task_id, from_status, to_status) VALUES (?1, NULL, ?2)",
                    params![task.id, status_str],
                ).map_err(|e| e.to_string())?;
            }
        }

        // 3. Remove deleted tasks
        if !active_ids.is_empty() {
            let id_list = active_ids.iter().map(|id| id.to_string()).collect::<Vec<String>>().join(",");
            let delete_query = format!("DELETE FROM tasks WHERE id NOT IN ({})", id_list);
            tx.execute(&delete_query, []).map_err(|e| e.to_string())?;
        } else {
            tx.execute("DELETE FROM tasks", []).map_err(|e| e.to_string())?;
        }

        tx.commit().map_err(|e| e.to_string())?;
        Ok(())
    }
    ```

---

### Task 3: Create Temporary Test Harness and Verify Changes
**Files:**
* Create: `temp_check/Cargo.toml`
* Create: `temp_check/src/lib.rs`
* Create: `temp_check/src/db_tests.rs`

**Interfaces:**
* Produces: A clean test run showing all parser and database sync tests passing with the new implementation.

- [ ] **Step 1: Setup a temporary standalone crate for testing**
    Create `temp_check/Cargo.toml` specifying only `rusqlite`, `serde`, `chrono`, and local source files copy.
    Copy `db.rs`, `todo_parser.rs`, and `todo_parser_tests.rs` into `temp_check/src/`.
    Add integration tests for the transactions, out-of-order parent reference deferral, and foreign keys.

- [ ] **Step 2: Run verification tests**
    Run: `cargo test` in `temp_check/`
    Expected: Compile and PASS all tests.

- [ ] **Step 3: Cleanup `temp_check` directory**
    Delete the temporary `temp_check` directory so it's not tracked.

- [ ] **Step 4: Commit changes**
    Run git commit for `src-tauri/src/db.rs`.

- [ ] **Step 5: Update the report**
    Update `/home/ling/dev2step/.git/sdd/task-3-report.md`.
