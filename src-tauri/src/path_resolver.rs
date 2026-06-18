use std::path::PathBuf;
use std::env;

pub fn resolve_paths() -> (PathBuf, PathBuf) {
    let current_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut cursor = current_dir.as_path();

    loop {
        let todo_path = cursor.join("todo.txt");
        let git_path = cursor.join(".git");
        if todo_path.exists() || git_path.exists() {
            return (todo_path, cursor.join("dev2step.db"));
        }
        if let Some(parent) = cursor.parent() {
            cursor = parent;
        } else {
            break;
        }
    }

    // Fallback to global
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let global_dir = home.join(".dev2step");
    if !global_dir.exists() {
        let _ = std::fs::create_dir_all(&global_dir);
    }
    (global_dir.join("todo.txt"), global_dir.join("dev2step.db"))
}
