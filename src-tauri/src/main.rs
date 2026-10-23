//! Konstruct for Linux and Windows: a Tauri window. The client runs in Rust (`bridge`); the
//! webview draws typed view events and sends back what the person did. No key, token or protocol
//! state reaches it (decisions/desktop-web-interface.md).

// No console window behind the app on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod bridge;

use bridge::{Intent, ViewEvent};
use tauri::{State, ipc::Channel};
use tokio::sync::mpsc;
use zeroize::Zeroizing;

struct Intents(mpsc::UnboundedSender<Intent>);

impl Intents {
    fn send(&self, intent: Intent) {
        let _ = self.0.send(intent);
    }
}

/// The page is up (or was reloaded): send it everything to draw, then each change.
#[tauri::command]
fn attach(on_event: Channel<ViewEvent>, intents: State<Intents>) {
    intents.send(Intent::Attach(on_event));
}

#[tauri::command]
fn register(username: String, intents: State<Intents>) {
    intents.send(Intent::Register(username));
}

#[tauri::command]
fn start_link(intents: State<Intents>) {
    intents.send(Intent::StartLink);
}

#[tauri::command]
fn cancel_link(intents: State<Intents>) {
    intents.send(Intent::CancelLink);
}

#[tauri::command]
fn unlock(passphrase: String, intents: State<Intents>) {
    intents.send(Intent::Unlock(Zeroizing::new(passphrase.into_bytes())));
}

#[tauri::command]
fn set_passphrase(passphrase: String, intents: State<Intents>) {
    intents.send(Intent::SetPassphrase(Zeroizing::new(
        passphrase.into_bytes(),
    )));
}

#[tauri::command]
fn open_chat(peer_id: String, intents: State<Intents>) {
    intents.send(Intent::OpenChat(peer_id));
}

#[tauri::command]
fn send_message(peer_id: String, text: String, intents: State<Intents>) {
    intents.send(Intent::Send { peer_id, text });
}

#[tauri::command]
fn find_contact(query: String, intents: State<Intents>) {
    intents.send(Intent::Find(query));
}

#[tauri::command]
fn add_contact(user_id: String, name: String, intents: State<Intents>) {
    intents.send(Intent::AddContact { user_id, name });
}

#[tauri::command]
fn delete_contact(peer_id: String, intents: State<Intents>) {
    intents.send(Intent::DeleteContact(peer_id));
}

#[tauri::command]
fn safety_number(peer_id: String, intents: State<Intents>) {
    intents.send(Intent::SafetyNumber(peer_id));
}

#[tauri::command]
fn mint_invite(intents: State<Intents>) {
    intents.send(Intent::MintInvite);
}

#[tauri::command]
fn logout(intents: State<Intents>) {
    intents.send(Intent::Logout);
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let (intents, inbox) = mpsc::unbounded_channel();
    let config = construct_client::config::load_config().unwrap_or_default();

    tauri::Builder::default()
        .manage(Intents(intents))
        .setup(move |_| {
            tauri::async_runtime::spawn(bridge::run(
                construct_client::ClientConfig {
                    server_url: config.server,
                    transport: config.transport,
                    no_encrypt: false,
                    pq_active: cfg!(feature = "post-quantum"),
                },
                inbox,
            ));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            attach,
            register,
            start_link,
            cancel_link,
            unlock,
            set_passphrase,
            open_chat,
            send_message,
            find_contact,
            add_contact,
            delete_contact,
            safety_number,
            mint_invite,
            logout,
        ])
        .run(tauri::generate_context!())
        .expect("the Tauri runtime failed to start");
}
