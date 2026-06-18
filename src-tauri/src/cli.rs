use clap::{Parser, Subcommand};
use super::path_resolver::resolve_paths;
use super::db::init_db;
use super::sync::{load_file_and_sync, write_tasks_to_file};
use super::todo_parser::{DevTask, TaskStatus};

#[derive(Parser, Debug)]
#[command(name = "dev2step", author = "Solo Dev", version = "1.0", about = "Minimalist Todo.txt & SQLite board")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Status,
    Add {
        description: String,
        #[arg(long)]
        project: Option<String>,
        #[arg(long)]
        priority: Option<char>,
        #[arg(long)]
        todo: bool,
        #[arg(long)]
        progress: bool,
    },
    Transition {
        #[arg(long)]
        id: u32,
        #[arg(long)]
        to: String,
    },
    Complete {
        #[arg(long)]
        id: u32,
    },
    Remove {
        #[arg(long)]
        id: u32,
    },
}

pub fn handle_cli() -> Result<bool, String> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 || args[1] == "tauri" {
        // Default to starting Tauri UI if no subcommand or is just Tauri runner args
        return Ok(false);
    }

    let cli = match Cli::try_parse() {
        Ok(c) => c,
        Err(e) => {
            e.exit();
        }
    };

    let (todo_path, db_path) = resolve_paths();
    let mut conn = init_db(&db_path)?;
    let mut tasks = load_file_and_sync(&todo_path, &mut conn)?;

    match cli.command {
        Commands::Status => {
            let columns = vec![TaskStatus::Icebox, TaskStatus::Todo, TaskStatus::Progress, TaskStatus::Done];
            for col in columns {
                println!("\n=== {} ===", col.to_str().to_uppercase());
                let col_tasks: Vec<&DevTask> = tasks.iter().filter(|t| t.status == col).collect();
                if col_tasks.is_empty() {
                    println!("  (Empty)");
                }
                for t in col_tasks {
                    let pri_str = t.priority.map(|p| format!("({}) ", p)).unwrap_or_default();
                    println!("  #{}: {}{}", t.id, pri_str, t.description);
                }
            }
        }
        Commands::Add { description, project, priority, todo, progress } => {
            let status = if progress {
                TaskStatus::Progress
            } else if todo {
                TaskStatus::Todo
            } else {
                TaskStatus::Icebox
            };

            let max_id = tasks.iter().map(|t| t.id).max().unwrap_or(0);
            let new_task = DevTask {
                id: max_id + 1,
                priority,
                is_completed: false,
                completion_date: None,
                creation_date: chrono::Local::now().format("%Y-%m-%d").to_string(),
                description,
                project,
                status,
                due_date: None,
                parent_id: None,
                line_number: tasks.len() + 1,
            };
            tasks.push(new_task);
            write_tasks_to_file(&todo_path, &tasks)?;
            load_file_and_sync(&todo_path, &mut conn)?;
            println!("Task added successfully.");
        }
        Commands::Transition { id, to } => {
            let to_lower = to.to_lowercase();
            if to_lower != "icebox" && to_lower != "todo" && to_lower != "progress" && to_lower != "done" {
                eprintln!("Invalid status. Choose from icebox, todo, progress, done");
                return Err("Invalid status".to_string());
            }
            let status = TaskStatus::from_str(&to_lower);
            if let Some(task) = tasks.iter_mut().find(|t| t.id == id) {
                task.status = status;
                if status == TaskStatus::Done {
                    task.is_completed = true;
                    if task.completion_date.is_none() {
                        task.completion_date = Some(chrono::Local::now().format("%Y-%m-%d").to_string());
                    }
                } else {
                    task.is_completed = false;
                    task.completion_date = None;
                }
                write_tasks_to_file(&todo_path, &tasks)?;
                load_file_and_sync(&todo_path, &mut conn)?;
                println!("Task #{} status updated to {}.", id, to);
            } else {
                eprintln!("Task #{} not found.", id);
                return Err("Task not found".to_string());
            }
        }
        Commands::Complete { id } => {
            if let Some(task) = tasks.iter_mut().find(|t| t.id == id) {
                task.is_completed = true;
                task.status = TaskStatus::Done;
                task.completion_date = Some(chrono::Local::now().format("%Y-%m-%d").to_string());
                write_tasks_to_file(&todo_path, &tasks)?;
                load_file_and_sync(&todo_path, &mut conn)?;
                println!("Task #{} marked completed.", id);
            } else {
                eprintln!("Task #{} not found.", id);
                return Err("Task not found".to_string());
            }
        }
        Commands::Remove { id } => {
            let initial_len = tasks.len();
            tasks.retain(|t| t.id != id);
            if tasks.len() < initial_len {
                write_tasks_to_file(&todo_path, &tasks)?;
                load_file_and_sync(&todo_path, &mut conn)?;
                println!("Task #{} removed.", id);
            } else {
                eprintln!("Task #{} not found.", id);
                return Err("Task not found".to_string());
            }
        }
    }

    Ok(true)
}
