// Prevents additional console window on Windows in release, DO NOT REMOVE!!
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

fn main() {
    match cli::handle_cli() {
        Ok(true) => {}
        Ok(false) => {
            tauri_app_lib::run();
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
