use rusqlite::{params, Connection};
use std::path::Path;
use super::todo_parser::DevTask;

pub fn init_db(db_path: &Path) -> Result<Connection, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    
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
            FOREIGN KEY(parent_id) REFERENCES tasks(id) ON DELETE SET NULL
        );",
        [],
    ).map_err(|e| e.to_string())?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS state_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            task_id INTEGER NOT NULL,
            from_status TEXT,
            to_status TEXT,
            changed_at TEXT DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY(task_id) REFERENCES tasks(id) ON DELETE CASCADE
        );",
        [],
    ).map_err(|e| e.to_string())?;

    Ok(conn)
}

pub fn sync_tasks_to_db(conn: &Connection, tasks: &[DevTask]) -> Result<(), String> {
    // Collect existing statuses before sync to detect transitions
    let mut stmt = conn.prepare("SELECT id, status FROM tasks").map_err(|e| e.to_string())?;
    let existing_statuses: std::collections::HashMap<u32, String> = stmt
        .query_map([], |row| Ok((row.get::<_, u32>(0)?, row.get::<_, String>(1)?)))
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();

    // Start transaction
    conn.execute("BEGIN TRANSACTION", []).map_err(|e| e.to_string())?;

    // 1. Upsert projects
    for task in tasks {
        if let Some(ref p) = task.project {
            conn.execute(
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

        conn.execute(
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
                conn.execute(
                    "INSERT INTO state_history (task_id, from_status, to_status) VALUES (?1, ?2, ?3)",
                    params![task.id, prev_status, status_str],
                ).map_err(|e| e.to_string())?;
            }
        } else {
            // New task insertion logs initial status transition
            conn.execute(
                "INSERT INTO state_history (task_id, from_status, to_status) VALUES (?1, NULL, ?2)",
                params![task.id, status_str],
            ).map_err(|e| e.to_string())?;
        }
    }

    // 3. Remove deleted tasks
    if !active_ids.is_empty() {
        let id_list = active_ids.iter().map(|id| id.to_string()).collect::<Vec<String>>().join(",");
        let delete_query = format!("DELETE FROM tasks WHERE id NOT IN ({})", id_list);
        conn.execute(&delete_query, []).map_err(|e| e.to_string())?;
    } else {
        conn.execute("DELETE FROM tasks", []).map_err(|e| e.to_string())?;
    }

    conn.execute("COMMIT TRANSACTION", []).map_err(|e| e.to_string())?;
    Ok(())
}
