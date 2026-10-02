//! Konstruct for Linux and Windows: a Tauri window whose content is a character grid drawn by
//! Ratatui and painted by xterm.js. The webview receives finished frames and sends keys back; it
//! never sees the client, its keys or its state.
//!
//! `decisions/desktop-is-the-tui-client-with-a-second-shell.md` in the vault.

// No console window behind the app on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod grid;
mod keys;
mod screen;

use tauri::{State, ipc::Channel};
use tokio::sync::mpsc;

use grid::Input;

struct GridInbox(mpsc::UnboundedSender<Input>);

/// The page is up (or was reloaded): draw the whole grid into `on_frame`.
#[tauri::command]
fn attach(cols: u16, rows: u16, on_frame: Channel<String>, inbox: State<GridInbox>) {
    let _ = inbox.0.send(Input::Attach {
        cols,
        rows,
        frames: on_frame,
    });
}

#[tauri::command]
fn resize(cols: u16, rows: u16, inbox: State<GridInbox>) {
    let _ = inbox.0.send(Input::Resize { cols, rows });
}

/// A named key, or a key held with Ctrl, Alt or Super.
#[tauri::command]
fn key(key: keys::DomKey, inbox: State<GridInbox>) {
    if let Some(event) = keys::to_key_event(&key) {
        let _ = inbox
            .0
            .send(Input::Event(ratatui::crossterm::event::Event::Key(event)));
    }
}

/// Typed text, an IME result or a paste.
#[tauri::command]
fn text(text: String, inbox: State<GridInbox>) {
    if let Some(event) = grid::text_event(text) {
        let _ = inbox.0.send(Input::Event(event));
    }
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let (inbox, inputs) = mpsc::unbounded_channel();
    let screen = screen::Screen::new(&construct_client::stored_session_state());

    tauri::Builder::default()
        .manage(GridInbox(inbox))
        .setup(|_| {
            tauri::async_runtime::spawn(grid::run(inputs, screen));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![attach, resize, key, text])
        .run(tauri::generate_context!())
        .expect("the Tauri runtime failed to start");
}
