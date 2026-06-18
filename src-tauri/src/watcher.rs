use notify::{Watcher, RecursiveMode, Event, RecommendedWatcher};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;
use tauri::Emitter;

pub static IGNORE_WATCHER: Mutex<bool> = Mutex::new(false);

pub fn start_file_watcher<R: tauri::Runtime>(
    app_handle: tauri::AppHandle<R>,
    todo_path: PathBuf,
    db_path: PathBuf,
) {
    let todo_path_canonical = todo_path.canonicalize().unwrap_or_else(|_| todo_path.clone());
    let todo_path_clone = todo_path.clone();
    
    let mut watcher = RecommendedWatcher::new(
        move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                if event.kind.is_modify() {
                    let matches_todo = event.paths.iter().any(|p| {
                        if let Ok(p_canon) = p.canonicalize() {
                            p_canon == todo_path_canonical
                        } else {
                            p.file_name() == todo_path_canonical.file_name()
                        }
                    });
                    if !matches_todo {
                        return;
                    }

                    // Check if we are ignoring self-writes
                    if let Ok(ignore) = IGNORE_WATCHER.lock() {
                        if *ignore {
                            return;
                        }
                    }

                    // Debounce manually via small sleep
                    std::thread::sleep(Duration::from_millis(150));

                    let todo_p = todo_path_clone.clone();
                    let db_p = db_path.clone();
                    let app = app_handle.clone();

                    let _ = std::thread::spawn(move || {
                        if let Ok(mut conn) = crate::db::init_db(&db_p) {
                            if let Ok(tasks) = crate::sync::load_file_and_sync(&todo_p, &mut conn) {
                                let _ = app.emit("todo-updated", tasks);
                            }
                        }
                    });
                }
            }
        },
        notify::Config::default(),
    ).unwrap();

    if let Some(parent) = todo_path.parent() {
        let _ = watcher.watch(parent, RecursiveMode::NonRecursive);
        // Keep watcher alive in thread
        std::thread::spawn(move || {
            let _watcher = watcher;
            loop {
                std::thread::sleep(Duration::from_secs(10));
            }
        });
    }
}
