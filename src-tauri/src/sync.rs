use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use super::todo_parser::{parse_line, format_task, DevTask};
use super::db::sync_tasks_to_db;

pub fn load_file_and_sync(todo_path: &Path, conn: &mut rusqlite::Connection) -> Result<Vec<DevTask>, String> {
    let mut tasks = Vec::new();
    if !todo_path.exists() {
        // Write empty file
        File::create(todo_path).map_err(|e| e.to_string())?;
        return Ok(tasks);
    }

    let file = File::open(todo_path).map_err(|e| e.to_string())?;
    let reader = BufReader::new(file);

    let mut line_num = 0;
    let mut needs_rewrite = false;
    let mut max_id = 0;

    for line in reader.lines() {
        line_num += 1;
        let line_str = line.map_err(|e| e.to_string())?;
        if line_str.trim().is_empty() {
            continue;
        }

        match parse_line(&line_str, line_num) {
            Ok(task) => {
                if task.id == 0 {
                    needs_rewrite = true;
                } else if task.id > max_id {
                    max_id = task.id;
                }
                tasks.push(task);
            }
            Err(_) => {
                // Skip invalid lines
            }
        }
    }

    // Fill missing IDs
    for task in &mut tasks {
        if task.id == 0 {
            max_id += 1;
            task.id = max_id;
            needs_rewrite = true;
        }
    }

    if needs_rewrite {
        write_tasks_to_file(todo_path, &tasks)?;
    }

    sync_tasks_to_db(conn, &tasks)?;
    Ok(tasks)
}

pub fn write_tasks_to_file(todo_path: &Path, tasks: &[DevTask]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(todo_path)
        .map_err(|e| e.to_string())?;

    for task in tasks {
        let formatted = format_task(task);
        writeln!(file, "{}", formatted).map_err(|e| e.to_string())?;
    }
    Ok(())
}
