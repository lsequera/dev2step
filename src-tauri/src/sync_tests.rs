#[cfg(test)]
mod tests {
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::Path;
    use crate::db::{self, sync_tasks_to_db};
    use crate::todo_parser::{DevTask, TaskStatus};
    use crate::sync::load_file_and_sync;

    fn clean_file(path: &Path) {
        if path.exists() {
            let _ = fs::remove_file(path);
        }
    }

    #[test]
    fn test_sync_nonexistent_file_creates_empty_file() {
        let todo_path = Path::new("target/nonexistent_todo_test.txt");
        clean_file(todo_path);

        let mut conn = db::init_db(Path::new(":memory:")).unwrap();

        // Seed DB with an existing task to verify it gets cleared when syncing an empty file
        let seed_task = DevTask {
            id: 42,
            priority: None,
            is_completed: false,
            completion_date: None,
            creation_date: "2026-06-17".to_string(),
            description: "Seed Task".to_string(),
            project: None,
            status: TaskStatus::Todo,
            due_date: None,
            parent_id: None,
            line_number: 1,
        };
        sync_tasks_to_db(&mut conn, &[seed_task]).unwrap();

        // Verify task is in DB
        let count_before: i64 = conn
            .query_row("SELECT COUNT(*) FROM tasks", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count_before, 1);

        // Run sync on nonexistent file
        let tasks = load_file_and_sync(todo_path, &mut conn).unwrap();
        assert!(tasks.is_empty());
        assert!(todo_path.exists());

        // Verify DB is cleared
        let count_after: i64 = conn
            .query_row("SELECT COUNT(*) FROM tasks", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count_after, 0);

        clean_file(todo_path);
    }

    #[test]
    fn test_sync_assigns_missing_ids_and_updates_line_numbers() {
        let todo_path = Path::new("target/sync_assign_test.txt");
        clean_file(todo_path);

        // Create initial file content with missing IDs and an empty line
        // Line 1: Missing ID
        // Line 2: Empty line (which should cause line numbers of subsequent tasks to shift after rewrite)
        // Line 3: Existing task with ID
        let mut file = File::create(todo_path).unwrap();
        writeln!(file, "Task without ID status:todo").unwrap();
        writeln!(file, "").unwrap();
        writeln!(file, "Task with ID status:progress id:10").unwrap();
        drop(file);

        let mut conn = db::init_db(Path::new(":memory:")).unwrap();

        // Run sync
        let tasks = load_file_and_sync(todo_path, &mut conn).unwrap();
        assert_eq!(tasks.len(), 2);

        // Task 1 should get ID 11 (max_id = 10, next is 11)
        assert_eq!(tasks[0].id, 11);
        assert_eq!(tasks[0].line_number, 1);

        // Task 2 should have ID 10 and line number updated to 2 (was 3 originally)
        assert_eq!(tasks[1].id, 10);
        assert_eq!(tasks[1].line_number, 2);

        // Check DB task records
        let mut stmt = conn
            .prepare("SELECT id, line_number, description FROM tasks ORDER BY line_number")
            .unwrap();
        let db_tasks: Vec<(u32, usize, String)> = stmt
            .query_map([], |row| {
                Ok((row.get::<_, u32>(0)?, row.get::<_, usize>(1)?, row.get::<_, String>(2)?))
            })
            .unwrap()
            .map(|r| r.unwrap())
            .collect();

        assert_eq!(db_tasks.len(), 2);
        assert_eq!(db_tasks[0], (11, 1, "Task without ID".to_string()));
        assert_eq!(db_tasks[1], (10, 2, "Task with ID".to_string()));

        // Check file was rewritten correctly with consecutive lines and updated IDs
        let rewritten_content = fs::read_to_string(todo_path).unwrap();
        let lines: Vec<&str> = rewritten_content.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("id:11"));
        assert!(lines[0].contains("status:todo"));
        assert!(lines[1].contains("id:10"));
        assert!(lines[1].contains("status:progress"));

        clean_file(todo_path);
    }

    #[test]
    fn test_sync_removes_deleted_tasks() {
        let todo_path = Path::new("target/sync_delete_test.txt");
        clean_file(todo_path);

        // 1. Seed two tasks in the file
        let mut file = File::create(todo_path).unwrap();
        writeln!(file, "Task one status:todo id:1").unwrap();
        writeln!(file, "Task two status:todo id:2").unwrap();
        drop(file);

        let mut conn = db::init_db(Path::new(":memory:")).unwrap();

        // Sync initially
        load_file_and_sync(todo_path, &mut conn).unwrap();

        // Verify both exist in DB
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM tasks", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 2);

        // 2. Rewrite file to only contain Task 1
        let mut file = File::create(todo_path).unwrap();
        writeln!(file, "Task one status:todo id:1").unwrap();
        drop(file);

        // Sync again
        load_file_and_sync(todo_path, &mut conn).unwrap();

        // Verify only Task 1 is left in DB
        let count_after: i64 = conn
            .query_row("SELECT COUNT(*) FROM tasks", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count_after, 1);

        let left_id: u32 = conn
            .query_row("SELECT id FROM tasks", [], |r| r.get(0))
            .unwrap();
        assert_eq!(left_id, 1);

        clean_file(todo_path);
    }
}
