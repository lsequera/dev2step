// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod todo_parser;
pub mod path_resolver;
pub mod db;
pub mod sync;
pub mod cli;
pub mod watcher;

#[cfg(test)]
pub mod todo_parser_tests;
#[cfg(test)]
pub mod sync_tests;

use std::path::PathBuf;
use std::time::Instant;
use tauri::{State, Manager};
use todo_parser::{DevTask, TaskStatus};
use path_resolver::resolve_paths;
use db::init_db;
use sync::{load_file_and_sync, write_tasks_to_file};
use watcher::LAST_WRITE_TIME;

struct AppState {
    todo_path: PathBuf,
    db_path: PathBuf,
}

#[tauri::command]
async fn get_tasks(state: State<'_, AppState>) -> Result<Vec<DevTask>, String> {
    let mut conn = init_db(&state.db_path)?;
    load_file_and_sync(&state.todo_path, &mut conn)
}

#[tauri::command]
async fn add_task(
    state: State<'_, AppState>,
    description: String,
    project: Option<String>,
    priority: Option<String>,
    status: String,
) -> Result<Vec<DevTask>, String> {
    let mut conn = init_db(&state.db_path)?;
    let mut tasks = load_file_and_sync(&state.todo_path, &mut conn)?;

    let max_id = tasks.iter().map(|t| t.id).max().unwrap_or(0);
    let pri_char = priority.and_then(|s| s.chars().next());

    let new_task = DevTask {
        id: max_id + 1,
        priority: pri_char,
        is_completed: status.to_lowercase() == "done",
        completion_date: if status.to_lowercase() == "done" { Some(chrono::Local::now().format("%Y-%m-%d").to_string()) } else { None },
        creation_date: chrono::Local::now().format("%Y-%m-%d").to_string(),
        description,
        project,
        status: TaskStatus::from_str(&status),
        due_date: None,
        parent_id: None,
        line_number: tasks.len() + 1,
    };

    tasks.push(new_task);
    if let Ok(mut last_write) = LAST_WRITE_TIME.lock() {
        *last_write = Some(Instant::now());
    }
    write_tasks_to_file(&state.todo_path, &tasks)?;
    let updated = load_file_and_sync(&state.todo_path, &mut conn)?;

    Ok(updated)
}

#[tauri::command]
async fn update_task_status(
    state: State<'_, AppState>,
    id: u32,
    status: String,
) -> Result<Vec<DevTask>, String> {
    let mut conn = init_db(&state.db_path)?;
    let mut tasks = load_file_and_sync(&state.todo_path, &mut conn)?;

    let next_status = TaskStatus::from_str(&status);
    if let Some(task) = tasks.iter_mut().find(|t| t.id == id) {
        task.status = next_status;
        if status.to_lowercase() == "done" {
            task.is_completed = true;
            task.completion_date = Some(chrono::Local::now().format("%Y-%m-%d").to_string());
        } else {
            task.is_completed = false;
            task.completion_date = None;
        }
        if let Ok(mut last_write) = LAST_WRITE_TIME.lock() {
            *last_write = Some(Instant::now());
        }
        write_tasks_to_file(&state.todo_path, &tasks)?;
    }

    let updated = load_file_and_sync(&state.todo_path, &mut conn)?;

    Ok(updated)
}

#[tauri::command]
async fn delete_task(state: State<'_, AppState>, id: u32) -> Result<Vec<DevTask>, String> {
    let mut conn = init_db(&state.db_path)?;
    let mut tasks = load_file_and_sync(&state.todo_path, &mut conn)?;

    tasks.retain(|t| t.id != id);
    if let Ok(mut last_write) = LAST_WRITE_TIME.lock() {
        *last_write = Some(Instant::now());
    }
    write_tasks_to_file(&state.todo_path, &tasks)?;
    let updated = load_file_and_sync(&state.todo_path, &mut conn)?;

    Ok(updated)
}

#[tauri::command]
async fn get_velocity_metrics(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let conn = init_db(&state.db_path)?;
    
    // Count tasks per status
    let mut stmt = conn.prepare("SELECT status, count(*) FROM tasks GROUP BY status").map_err(|e| e.to_string())?;
    let counts: Vec<(String, i64)> = stmt
        .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)))
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();

    // Calculate average transition times (cycle time from progress to done)
    let mut stmt_cycle = conn.prepare(
        "SELECT avg(julianday(b.changed_at) - julianday(a.changed_at)) 
         FROM state_history a 
         JOIN state_history b ON a.task_id = b.task_id
         WHERE a.to_status = 'progress' AND b.to_status = 'done' AND b.changed_at > a.changed_at"
    ).map_err(|e| e.to_string())?;

    let avg_cycle_days: f64 = stmt_cycle
        .query_row([], |row| row.get::<_, Option<f64>>(0))
        .map_err(|e| e.to_string())?
        .unwrap_or(0.0);

    Ok(serde_json::json!({
        "status_counts": counts,
        "avg_cycle_days": avg_cycle_days
    }))
}

fn main() {
    // Run CLI handler. If it returns true, we terminate (CLI command completed)
    match cli::handle_cli() {
        Ok(true) => return,
        Ok(false) => {}
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }

    let (todo_path, db_path) = resolve_paths();

    // Ensure file exists
    if !todo_path.exists() {
        let _ = std::fs::File::create(&todo_path);
    }

    // Initialize SQLite DB cache on start
    if let Ok(mut conn) = init_db(&db_path) {
        let _ = load_file_and_sync(&todo_path, &mut conn);
    }

    let todo_path_watcher = todo_path.clone();
    let db_path_watcher = db_path.clone();

    tauri::Builder::default()
        .setup(move |app| {
            let app_handle = app.handle().clone();
            watcher::start_file_watcher(app_handle, todo_path_watcher, db_path_watcher);
            Ok(())
        })
        .manage(AppState { todo_path, db_path })
        .invoke_handler(tauri::generate_handler![
            get_tasks,
            add_task,
            update_task_status,
            delete_task,
            get_velocity_metrics
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
