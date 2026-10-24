//! Konstruct for Linux and Windows: a Tauri window. The client runs in Rust (`bridge`); the
//! webview draws typed view events and sends back what the person did. No key, token or protocol
//! state reaches it (decisions/desktop-web-interface.md).

// No console window behind the app on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod bridge;

use bridge::{Intent, ViewEvent};
use tauri::{Manager, State, ipc::Channel};
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
fn reply(peer_id: String, text: String, to: String, intents: State<Intents>) {
    intents.send(Intent::Reply { peer_id, text, to });
}

#[tauri::command]
fn edit_message(message_id: String, text: String, intents: State<Intents>) {
    intents.send(Intent::Edit { message_id, text });
}

#[tauri::command]
fn react(message_id: String, emoji: String, intents: State<Intents>) {
    intents.send(Intent::React { message_id, emoji });
}

#[tauri::command]
fn delete_message(message_id: String, intents: State<Intents>) {
    intents.send(Intent::DeleteMessage(message_id));
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

/// Show the recovery phrase that waits for its copy: the one secret the window is ever given,
/// because the person has to read it.
#[tauri::command]
fn reveal_recovery_phrase(intents: State<Intents>) {
    intents.send(Intent::RevealRecoveryPhrase);
}

/// Words of the phrase by position, as the person typed them back; the client checks them.
#[tauri::command]
fn confirm_recovery_copy(answers: Vec<(usize, String)>, intents: State<Intents>) {
    intents.send(Intent::ConfirmRecoveryCopy(answers));
}

#[tauri::command]
fn enter_recovery_phrase(phrase: String, intents: State<Intents>) {
    intents.send(Intent::EnterRecoveryPhrase(Zeroizing::new(phrase)));
}

/// Where the logs are written.
struct LogDir(std::path::PathBuf);

/// Open the log folder in the system's file manager.
#[tauri::command]
fn open_logs(dir: State<LogDir>) -> Result<(), String> {
    let opener = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(target_os = "windows") {
        "explorer"
    } else {
        "xdg-open"
    };
    std::process::Command::new(opener)
        .arg(&dir.0)
        .spawn()
        .map(drop)
        .map_err(|e| format!("{opener}: {e}"))
}

#[tauri::command]
fn logs_path(dir: State<LogDir>) -> String {
    dir.0.display().to_string()
}

/// Log to the terminal and to a file a day in `dir`, the last week kept: an app started from
/// the menu has no terminal, and the logs are what a person can send when something is wrong.
/// They stay on this computer; message text is never logged.
fn init_logging(dir: &std::path::Path) {
    use tracing_subscriber::{EnvFilter, fmt, prelude::*};
    let filter = || {
        EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("info,h2=warn,rustls=warn"))
    };
    let file = std::fs::create_dir_all(dir)
        .map_err(|e| e.to_string())
        .and_then(|()| {
            tracing_appender::rolling::Builder::new()
                .rotation(tracing_appender::rolling::Rotation::DAILY)
                .filename_prefix("konstruct")
                .filename_suffix("log")
                .max_log_files(7)
                .build(dir)
                .map_err(|e| e.to_string())
        });
    let registry = tracing_subscriber::registry().with(fmt::layer().with_filter(filter()));
    match file {
        Ok(appender) => registry
            .with(
                fmt::layer()
                    .with_ansi(false)
                    .with_writer(appender)
                    .with_filter(filter()),
            )
            .init(),
        Err(e) => {
            registry.init();
            tracing::warn!(error = %e, dir = %dir.display(), "logs go to the terminal only");
        }
    }
}

fn main() {
    let (intents, inbox) = mpsc::unbounded_channel();
    let config = construct_client::config::load_config().unwrap_or_default();

    let builder = tauri::Builder::default();
    // WKWebView takes copy and paste only from an Edit menu; Linux and Windows need none, and a
    // menu there would be a menu bar in the window.
    #[cfg(target_os = "macos")]
    let builder = builder.menu(tauri::menu::Menu::default);
    builder
        .manage(Intents(intents))
        .setup(move |app| {
            let dir = app
                .path()
                .app_log_dir()
                .unwrap_or_else(|_| std::env::temp_dir().join("konstruct-logs"));
            init_logging(&dir);
            tracing::info!(version = env!("CARGO_PKG_VERSION"), "Konstruct starting");
            app.manage(LogDir(dir));
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
            reply,
            edit_message,
            react,
            delete_message,
            find_contact,
            add_contact,
            delete_contact,
            safety_number,
            mint_invite,
            logout,
            reveal_recovery_phrase,
            confirm_recovery_copy,
            enter_recovery_phrase,
            open_logs,
            logs_path,
        ])
        .run(tauri::generate_context!())
        .expect("the Tauri runtime failed to start");
}
