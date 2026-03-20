#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ui;
mod engine;
mod storage;
mod grpc;
mod wire;

use engine::ConstructEngine;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    // Logging for debugging.
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse().unwrap()))
        .init();

    // Construct shared app engine state.
    let engine = ConstructEngine::new_default().await;

    // Start Tauri.
    tauri::Builder::default()
        .manage(engine)
        .invoke_handler(tauri::generate_handler![
            ui::get_app_status,
            ui::ensure_authenticated,
            ui::send_text,
            ui::list_chats,
            ui::list_messages
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
