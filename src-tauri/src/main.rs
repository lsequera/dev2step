// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod todo_parser;
pub mod path_resolver;
pub mod db;
#[cfg(test)]
pub mod todo_parser_tests;

fn main() {
    tauri_app_lib::run()
}
